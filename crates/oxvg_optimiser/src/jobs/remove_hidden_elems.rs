use std::{
    cell::RefCell,
    collections::{HashMap, HashSet},
};

use lightningcss::{
    properties::display::{Display, DisplayKeyword, Visibility},
    values::{alpha::AlphaValue, percentage::DimensionPercentage},
};
use oxvg_ast::{
    element::{Element, HashableElement},
    get_attribute, get_computed_style, has_attribute, has_computed_style, is_attribute, is_element,
    style::{ComputedStyles, Mode},
    visitor::{Context, ContextFlags, PrepareOutcome, Visitor},
};
use oxvg_collections::{
    atom::Atom,
    attribute::{
        inheritable::Inheritable, presentation::LengthPercentage, uncategorised::Radius, Attr,
    },
    element::{ElementId, ElementInfo},
};
use serde::{Deserialize, Serialize};

#[cfg(feature = "wasm")]
use tsify::Tsify;

use crate::error::JobsError;

#[cfg_attr(feature = "wasm", derive(Tsify))]
#[cfg_attr(feature = "napi", napi(object))]
#[derive(Clone, Default, Deserialize, Serialize, Debug)]
#[serde(rename_all = "camelCase")]
/// Removes hidden or invisible elements from the document.
///
/// # Correctness
///
/// This job should never visually change the document.
///
/// Animations on removed element may end up breaking.
///
/// # Errors
///
/// Never.
///
/// If this job produces an error or panic, please raise an [issue](https://github.com/noahbald/oxvg/issues)
pub struct RemoveHiddenElems {
    /// Whether to remove elements with `visibility` set to `hidden`
    #[cfg_attr(feature = "wasm", tsify(optional))]
    pub is_hidden: Option<bool>,
    /// Whether to remove elements with `display` set to `none`
    #[cfg_attr(feature = "wasm", tsify(optional))]
    pub display_none: Option<bool>,
    /// Whether to remove elements with `opacity` set to `0`
    #[cfg_attr(feature = "wasm", tsify(optional))]
    pub opacity_zero: Option<bool>,
    /// Whether to remove `<circle>` with `radius` set to `0`
    #[cfg_attr(feature = "wasm", tsify(optional))]
    pub circle_r_zero: Option<bool>,
    /// Whether to remove `<ellipse>` with `rx` set to `0`
    #[cfg_attr(feature = "wasm", tsify(optional))]
    pub ellipse_rx_zero: Option<bool>,
    /// Whether to remove `<ellipse>` with `ry` set to `0`
    #[cfg_attr(feature = "wasm", tsify(optional))]
    pub ellipse_ry_zero: Option<bool>,
    /// Whether to remove `<rect>` with `width` set to `0`
    #[cfg_attr(feature = "wasm", tsify(optional))]
    pub rect_width_zero: Option<bool>,
    /// Whether to remove `<rect>` with `height` set to `0`
    #[cfg_attr(feature = "wasm", tsify(optional))]
    pub rect_height_zero: Option<bool>,
    /// Whether to remove `<pattern>` with `width` set to `0`
    #[cfg_attr(feature = "wasm", tsify(optional))]
    pub pattern_width_zero: Option<bool>,
    /// Whether to remove `<pattern>` with `height` set to `0`
    #[cfg_attr(feature = "wasm", tsify(optional))]
    pub pattern_height_zero: Option<bool>,
    /// Whether to remove `<image>` with `width` set to `0`
    #[cfg_attr(feature = "wasm", tsify(optional))]
    pub image_width_zero: Option<bool>,
    /// Whether to remove `<image>` with `height` set to `0`
    #[cfg_attr(feature = "wasm", tsify(optional))]
    pub image_height_zero: Option<bool>,
    /// Whether to remove `<path>` with empty `d`
    #[cfg_attr(feature = "wasm", tsify(optional))]
    pub path_empty_d: Option<bool>,
    /// Whether to remove `<polyline>` with empty `points`
    #[cfg_attr(feature = "wasm", tsify(optional))]
    pub polyline_empty_points: Option<bool>,
    /// Whether to remove `<polygon>` with empty `points`
    #[cfg_attr(feature = "wasm", tsify(optional))]
    pub polygon_empty_points: Option<bool>,
}

#[derive(Clone, Default, Debug)]
struct Data<'input, 'arena> {
    opacity_zero: bool,
    non_rendered_nodes: RefCell<HashSet<HashableElement<'input, 'arena>>>,
    removed_def_ids: RefCell<HashSet<Atom<'input>>>,
    all_defs: RefCell<HashSet<HashableElement<'input, 'arena>>>,
    all_references: RefCell<HashSet<String>>,
    references_by_id: RefCell<HashMap<String, Vec<Element<'input, 'arena>>>>,
}

struct State<'o, 'input, 'arena> {
    options: &'o RemoveHiddenElems,
    data: &'o mut Data<'input, 'arena>,
}

impl<'input, 'arena> Visitor<'input, 'arena> for Data<'input, 'arena> {
    type Error = JobsError<'input>;

    fn prepare(
        &self,
        document: &Element<'input, 'arena>,
        context: &mut Context<'input, 'arena, '_>,
    ) -> Result<PrepareOutcome, Self::Error> {
        context.query_has_stylesheet(document);
        Ok(PrepareOutcome::none)
    }

    fn element(
        &self,
        element: &Element<'input, 'arena>,
        context: &mut Context<'input, 'arena, '_>,
    ) -> Result<(), Self::Error> {
        if element
            .qual_name()
            .info()
            .contains(ElementInfo::NonRendering)
        {
            self.non_rendered_nodes
                .borrow_mut()
                .insert(HashableElement::new(element.clone()));
            context.flags.visit_skip();
            return Ok(());
        }

        self.ref_element(element);
        let computed_styles = ComputedStyles::default()
            .with_all(element, &context.query_has_stylesheet_result)
            .map_err(JobsError::ComputedStylesError)?;
        if self.opacity_zero
            && matches!(
                get_computed_style!(computed_styles, Opacity),
                Some((Inheritable::Defined(AlphaValue(0.0)), Mode::Static))
            )
        {
            if is_element!(element, Path) {
                self.non_rendered_nodes
                    .borrow_mut()
                    .insert(HashableElement::new(element.clone()));
                context.flags.visit_skip();
                return Ok(());
            }
            self.remove_element(element);
        }
        Ok(())
    }
}

impl<'input, 'arena> Data<'input, 'arena> {
    fn remove_element(&self, element: &Element<'input, 'arena>) {
        if let Some(parent) = Element::parent_element(element) {
            if is_element!(parent, Defs) {
                if let Some(id) = get_attribute!(element, Id) {
                    self.removed_def_ids.borrow_mut().insert(id.clone());
                }
                if parent.child_element_count() == 1 {
                    log::debug!("data: removing parent");
                    parent.remove();
                    return;
                }
            }
        }
        log::debug!("data: removing element: {element:?}");
        element.remove();
    }

    fn ref_element(&self, element: &Element<'input, 'arena>) {
        match element.qual_name().unaliased() {
            ElementId::Defs => {
                self.all_defs
                    .borrow_mut()
                    .insert(HashableElement::new(element.clone()));
            }
            ElementId::Use => {
                for attr in element.attributes() {
                    let (Attr::Href(value) | Attr::XLinkHref(value)) = &*attr else {
                        continue;
                    };
                    let id = &value[1..];

                    let mut references_by_id = self.references_by_id.borrow_mut();
                    let refs = references_by_id.get_mut(id);
                    match refs {
                        Some(refs) => refs.push(element.clone()),
                        None => {
                            references_by_id.insert(id.into(), vec![element.clone()]);
                        }
                    }
                }
            }
            _ => {}
        }
    }
}

impl<'input, 'arena> Visitor<'input, 'arena> for RemoveHiddenElems {
    type Error = JobsError<'input>;

    fn prepare(
        &self,
        document: &Element<'input, 'arena>,
        context: &mut Context<'input, 'arena, '_>,
    ) -> Result<PrepareOutcome, Self::Error> {
        log::debug!("collecting data");
        context.query_has_script(document);
        let document = &mut document.clone();
        let mut data = Data {
            opacity_zero: self.opacity_zero.unwrap_or(true),
            ..Data::default()
        };
        data.start(document, context.info, Some(context.flags.clone()))?;
        log::debug!("data collected");
        State {
            options: self,
            data: &mut data,
        }
        .start(document, context.info, Some(context.flags.clone()))?;
        Ok(PrepareOutcome::skip)
    }
}

impl<'input, 'arena> Visitor<'input, 'arena> for State<'_, 'input, 'arena> {
    type Error = JobsError<'input>;

    fn prepare(
        &self,
        document: &Element<'input, 'arena>,
        context: &mut Context<'input, 'arena, '_>,
    ) -> Result<PrepareOutcome, Self::Error> {
        context.query_has_stylesheet(document);
        Ok(PrepareOutcome::none)
    }

    fn element(
        &self,
        element: &Element<'input, 'arena>,
        context: &mut Context<'input, 'arena, '_>,
    ) -> Result<(), Self::Error> {
        let computed_styles = ComputedStyles::default()
            .with_all(element, &context.query_has_stylesheet_result)
            .map_err(JobsError::ComputedStylesError)?;
        if self.is_hidden_style(element, &computed_styles, context)
            || self.is_hidden_ellipse(element)
            || self.is_hidden_rect(element)
            || self.is_hidden_pattern(element)
            || self.is_hidden_image(element)
            || self.is_hidden_path(element, &computed_styles)
            || self.is_hidden_poly(element)
        {
            log::debug!("RemoveHiddenElems: removing hidden");
            self.data.remove_element(element);
            return Ok(());
        }

        for mut attr in element.attributes().into_iter_mut() {
            if is_attribute!(attr, Id) {
                continue;
            }
            let mut all_references = self.data.all_references.borrow_mut();
            let mut value = attr.value_mut();
            value.visit_url(|url| {
                if let Some(url) = url.strip_prefix('#') {
                    all_references.insert(url.to_string());
                }
            });
            value.visit_id(|id| {
                all_references.insert(id.to_string());
            });
        }
        Ok(())
    }

    fn exit_document(
        &self,
        _document: &Element<'input, 'arena>,
        context: &Context<'input, 'arena, '_>,
    ) -> Result<(), Self::Error> {
        for id in &*self.data.removed_def_ids.borrow() {
            if let Some(refs) = self.data.references_by_id.borrow().get(&**id) {
                for node in refs {
                    log::debug!("RemoveHiddenElems: remove referenced by id");
                    node.remove();
                }
            }
        }

        let deoptimized = context.flags.intersects(
            ContextFlags::query_has_stylesheet_result | ContextFlags::query_has_script_result,
        );
        if !deoptimized {
            for non_rendered_node in &*self.data.non_rendered_nodes.borrow() {
                if self.can_remove_non_rendering_node(non_rendered_node) {
                    log::debug!("RemoveHiddenElems: remove non-rendered node");
                    non_rendered_node.remove();
                }
            }
        }

        for node in &*self.data.all_defs.borrow() {
            if node.is_empty() {
                log::debug!("RemoveHiddenElems: remove def");
                node.remove();
            }
        }

        Ok(())
    }
}

impl<'input, 'arena> State<'_, 'input, 'arena> {
    fn can_remove_non_rendering_node(&self, element: &Element<'input, 'arena>) -> bool {
        if let Some(id) = get_attribute!(element, Id) {
            if self.data.all_references.borrow().contains(&**id) {
                return false;
            }
        }
        element
            .children_iter()
            .all(|e| self.can_remove_non_rendering_node(&e))
    }

    fn is_hidden_style(
        &self,
        element: &Element,
        computed_styles: &ComputedStyles,
        context: &mut Context,
    ) -> bool {
        let mut is_hidden = false;
        if self.options.is_hidden.unwrap_or(true) {
            if let Some((Inheritable::Defined(Visibility::Hidden), Mode::Static)) =
                get_computed_style!(computed_styles, Visibility)
            {
                if !element.breadth_first().any(|child| {
                    matches!(
                        get_attribute!(child, Visibility).as_deref(),
                        Some(Inheritable::Defined(Visibility::Visible) | Inheritable::Inherited)
                    )
                }) {
                    is_hidden = true;
                }
            }
        }

        if !is_hidden && self.options.display_none.unwrap_or(true) {
            if let Some((Inheritable::Defined(Display::Keyword(DisplayKeyword::None)), _)) =
                get_computed_style!(computed_styles, Display)
            {
                is_hidden = !is_element!(element, Marker);
            }
        }
        if is_hidden {
            // Protect references that may use non-visible data
            let references_by_id = self.data.references_by_id.borrow();
            if let Some(id) = get_attribute!(element, Id) {
                if references_by_id.contains_key(id.as_str()) {
                    context.flags.visit_skip();
                    return false;
                }
            }
            return !element.breadth_first().any(|child| {
                if let Some(id) = get_attribute!(child, Id) {
                    if references_by_id.contains_key(id.as_str()) {
                        return true;
                    }
                }
                false
            });
        }
        is_hidden
    }

    fn is_hidden_ellipse(&self, element: &Element<'input, 'arena>) -> bool {
        if is_element!(element, Circle)
            && element.is_empty()
            && self.options.circle_r_zero.unwrap_or(true)
        {
            if let Some(LengthPercentage(DimensionPercentage::Dimension(length))) =
                get_attribute!(element, RGeometry).as_deref()
            {
                if length.to_px() == Some(0.0) {
                    log::debug!("RemoveHiddenElement: removing hidden ellipse");
                    element.remove();
                    return true;
                }
            }
        }

        if is_element!(element, Ellipse) {
            if self.options.ellipse_rx_zero.unwrap_or(true) {
                if let Some(Radius::LengthPercentage(LengthPercentage(
                    DimensionPercentage::Dimension(length),
                ))) = get_attribute!(element, RX).as_deref()
                {
                    if length.to_px() == Some(0.0) {
                        return true;
                    }
                }
            }

            if self.options.ellipse_ry_zero.unwrap_or(true) {
                if let Some(Radius::LengthPercentage(LengthPercentage(
                    DimensionPercentage::Dimension(length),
                ))) = get_attribute!(element, RY).as_deref()
                {
                    if length.to_px() == Some(0.0) {
                        return true;
                    }
                }
            }
        }

        false
    }

    fn is_hidden_rect(&self, element: &Element<'input, 'arena>) -> bool {
        if is_element!(element, Rect) && element.is_empty() {
            if self.options.rect_width_zero.unwrap_or(true) {
                if let Some(LengthPercentage(DimensionPercentage::Dimension(length))) =
                    get_attribute!(element, WidthRect).as_deref()
                {
                    if length.to_px() == Some(0.0) {
                        return true;
                    }
                }
            }
            if self.options.rect_height_zero.unwrap_or(true) {
                if let Some(LengthPercentage(DimensionPercentage::Dimension(length))) =
                    get_attribute!(element, HeightRect).as_deref()
                {
                    if length.to_px() == Some(0.0) {
                        return true;
                    }
                }
            }
        }
        false
    }

    fn is_hidden_pattern(&self, element: &Element<'input, 'arena>) -> bool {
        if is_element!(element, Pattern) {
            if self.options.pattern_width_zero.unwrap_or(true) {
                if let Some(LengthPercentage(DimensionPercentage::Dimension(length))) =
                    get_attribute!(element, WidthPattern).as_deref()
                {
                    if length.to_px() == Some(0.0) {
                        return true;
                    }
                }
            }
            if self.options.pattern_height_zero.unwrap_or(true) {
                if let Some(LengthPercentage(DimensionPercentage::Dimension(length))) =
                    get_attribute!(element, HeightPattern).as_deref()
                {
                    if length.to_px() == Some(0.0) {
                        return true;
                    }
                }
            }
        }
        false
    }

    fn is_hidden_image(&self, element: &Element<'input, 'arena>) -> bool {
        if is_element!(element, Image) {
            if self.options.image_width_zero.unwrap_or(true) {
                if let Some(LengthPercentage(DimensionPercentage::Dimension(length))) =
                    get_attribute!(element, WidthImage).as_deref()
                {
                    if length.to_px() == Some(0.0) {
                        return true;
                    }
                }
            }
            if self.options.image_height_zero.unwrap_or(true) {
                if let Some(LengthPercentage(DimensionPercentage::Dimension(length))) =
                    get_attribute!(element, HeightImage).as_deref()
                {
                    if length.to_px() == Some(0.0) {
                        return true;
                    }
                }
            }
        }
        false
    }

    fn is_hidden_path(
        &self,
        element: &Element<'input, 'arena>,
        computed_styles: &ComputedStyles<'input>,
    ) -> bool {
        if self.options.path_empty_d.unwrap_or(true) && is_element!(element, Path) {
            let Some(d) = get_attribute!(element, D) else {
                return true;
            };
            return d.0 .0.is_empty()
                || (d.0 .0.len() == 1
                    && !has_computed_style!(computed_styles, MarkerStart)
                    && !has_computed_style!(computed_styles, MarkerEnd));
        }
        false
    }

    fn is_hidden_poly(&self, element: &Element<'input, 'arena>) -> bool {
        if self.options.polyline_empty_points.unwrap_or(true)
            && is_element!(element, Polyline)
            && !has_attribute!(element, Points)
        {
            return true;
        }

        if self.options.polygon_empty_points.unwrap_or(true)
            && is_element!(element, Polygon)
            && !has_attribute!(element, Points)
        {
            return true;
        }
        false
    }
}

#[test]
#[allow(clippy::too_many_lines)]
fn remove_hidden_elems() -> anyhow::Result<()> {
    use crate::test_config;

    insta::assert_snapshot!(test_config(
        r#"{ "removeHiddenElems": {} }"#,
        Some(
            r#"<svg xmlns="http://www.w3.org/2000/svg">
    <!-- remove element with `display` of `none` -->
    <style>
      .a { display: block; }
    </style>
    <g>
        <rect display="none" x="0" y="0" width="20" height="20" />
        <rect display="none" class="a" x="0" y="0" width="20" height="20" />
    </g>
</svg>"#
        ),
    )?);

    insta::assert_snapshot!(test_config(
        r#"{ "removeHiddenElems": {} }"#,
        Some(
            r#"<svg xmlns="http://www.w3.org/2000/svg">
    <!-- remove element with `opacity` of `0` -->
    <style>
      .a { opacity: 0.5; }
    </style>
    <g>
        <rect opacity="0" x="0" y="0" width="20" height="20" />
        <rect opacity="0" class="a" x="0" y="0" width="20" height="20" />
    </g>
</svg>"#
        ),
    )?);

    insta::assert_snapshot!(test_config(
        r#"{ "removeHiddenElems": {} }"#,
        Some(
            r#"<svg xmlns="http://www.w3.org/2000/svg">
    <!-- Remove non-animated circle with zero radius -->
    <g>
        <circle r="0"/>
    </g>
    <circle cx="16" cy="3" r="0">
        <animate attributeName="r" values="0;3;0;0" dur="1s" repeatCount="indefinite" begin="0" keySplines="0.2 0.2 0.4 0.8;0.2 0.2 0.4 0.8;0.2 0.2 0.4 0.8" calcMode="spline"/>
    </circle>
</svg>"#
        ),
    )?);

    insta::assert_snapshot!(test_config(
        r#"{ "removeHiddenElems": {} }"#,
        Some(
            r#"<svg xmlns="http://www.w3.org/2000/svg">
    <!-- remove ellipse with zero radius -->
    <g>
        <ellipse rx="0"/>
        <ellipse ry="0"/>
    </g>
</svg>"#
        ),
    )?);

    insta::assert_snapshot!(test_config(
        r#"{ "removeHiddenElems": {} }"#,
        Some(
            r#"<svg xmlns="http://www.w3.org/2000/svg">
    <!-- remove rect with zero size -->
    <g>
        <rect width="0"/>
        <rect height="0"/>
    </g>
</svg>"#
        ),
    )?);

    insta::assert_snapshot!(test_config(
        r#"{ "removeHiddenElems": {} }"#,
        Some(
            r#"<svg xmlns="http://www.w3.org/2000/svg">
    <!-- remove pattern with zero size -->
    <g>
        <pattern width="0"/>
        <pattern height="0"/>
    </g>
</svg>"#
        ),
    )?);

    insta::assert_snapshot!(test_config(
        r#"{ "removeHiddenElems": {} }"#,
        Some(
            r#"<svg xmlns="http://www.w3.org/2000/svg">
    <!-- remove image with zero size -->
    <g>
        <image width="0"/>
        <image height="0"/>
    </g>
</svg>"#
        ),
    )?);

    insta::assert_snapshot!(test_config(
        r#"{ "removeHiddenElems": {} }"#,
        Some(
            r#"<svg xmlns="http://www.w3.org/2000/svg">
    <!-- remove empty or single points without markers -->
    <g>
        <path/>
        <path d="z"/>
        <path d="M 50 50"/>
        <path d="M 50 50 L 0"/>
        <path d="M1.25.75"/>
        <path d="M 50 50 20 20"/>
        <path d="M 50,50 20,20"/>
        <path d="M 50 50 H 10"/>
        <path d="M4.1.5.5.1"/>
        <path d="M10.77.45c-.19-.2-.51-.2-.7 0"/>
        <path d="M 6.39441613e-11,8.00287799 C2.85816855e-11,3.58301052 3.5797863,0 8.00005106,0"/>
        <path d="" marker-start="url(#id)"/>
        <path d="" marker-end="url(#id)"/>
        <path d="M 50 50" marker-start="url(#id)"/>
        <path d="M 50 50" marker-end="url(#id)"/>
    </g>
</svg>"#
        ),
    )?);

    insta::assert_snapshot!(test_config(
        r#"{ "removeHiddenElems": {} }"#,
        Some(
            r#"<svg xmlns="http://www.w3.org/2000/svg">
    <!-- remove polyline without points -->
    <g>
        <polyline/>
    </g>
</svg>"#
        ),
    )?);

    insta::assert_snapshot!(test_config(
        r#"{ "removeHiddenElems": {} }"#,
        Some(
            r#"<svg xmlns="http://www.w3.org/2000/svg">
    <!-- remove polygon without points -->
    <g>
        <polygon/>
    </g>
</svg>"#
        ),
    )?);

    insta::assert_snapshot!(test_config(
        r#"{ "removeHiddenElems": {} }"#,
        Some(
            r#"<svg width="480" height="360" xmlns="http://www.w3.org/2000/svg">
    <!-- preserve transparent rect inside clip-path -->
    <clipPath id="opacityclip">
        <rect width="100" height="100" opacity="0"/>
    </clipPath>
    <rect x="0.5" y="0.5" width="99" height="99" fill="red"/>
    <rect width="100" height="100" fill="lime" clip-path="url(#opacityclip)"/>
</svg>"#
        ),
    )?);

    insta::assert_snapshot!(test_config(
        r#"{ "removeHiddenElems": {} }"#,
        Some(
            r#"<svg width="480" height="360" xmlns="http://www.w3.org/2000/svg">
    <!-- remove only hidden visibility without visible children -->
    <style>
        .a { visibility: visible; }
    </style>
    <rect x="96" y="96" width="96" height="96" fill="lime" />
    <g visibility="hidden">
        <rect x="96" y="96" width="96" height="96" fill="red" />
    </g>
    <rect x="196.5" y="196.5" width="95" height="95" fill="red"/>
    <g visibility="hidden">
        <rect x="196" y="196" width="96" height="96" fill="lime" visibility="visible" />
    </g>
    <rect x="96" y="96" width="96" height="96" visibility="hidden" class="a" />
</svg>"#
        ),
    )?);

    insta::assert_snapshot!(test_config(
        r#"{ "removeHiddenElems": {} }"#,
        Some(
            r##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 64 64">
    <!-- remove references to useless defs -->
    <defs>
        <path d="M15.852 62.452" id="a"/>
    </defs>
    <use href="#a"/>
    <use opacity=".35" href="#a"/>
</svg>
"##
        ),
    )?);

    insta::assert_snapshot!(test_config(
        r#"{ "removeHiddenElems": {} }"#,
        Some(
            r#"<svg xmlns="http://www.w3.org/2000/svg">
    <!-- remove unused defs -->
    <defs>
        <linearGradient id="a">
        </linearGradient>
    </defs>
</svg>"#
        ),
    )?);

    insta::assert_snapshot!(test_config(
        r#"{ "removeHiddenElems": {} }"#,
        Some(
            r#"<svg xmlns="http://www.w3.org/2000/svg">
    <!-- don't remove used defs -->
    <rect fill="url(#a)" width="64" height="64"/>
    <defs>
        <linearGradient id="a">
        </linearGradient>
    </defs>
</svg>"#
        ),
    )?);

    insta::assert_snapshot!(test_config(
        r#"{ "removeHiddenElems": {} }"#,
        Some(
            r#"<svg xmlns="http://www.w3.org/2000/svg">
    <!-- don't remove elements with id'd children -->
    <rect fill="url(#a)" width="64" height="64"/>
    <symbol>
        <linearGradient id="a">
            <stop offset="5%" stop-color="gold" />
        </linearGradient>
    </symbol>
</svg>"#
        ),
    )?);

    insta::assert_snapshot!(test_config(
        r#"{ "removeHiddenElems": {} }"#,
        Some(
            r#"<svg xmlns="http://www.w3.org/2000/svg">
    <!-- don't remove nodes with referenced children -->
    <rect fill="url(#a)" width="64" height="64"/>
    <g>
        <linearGradient id="a">
            <stop offset="5%" stop-color="gold" />
        </linearGradient>
    </g>
</svg>"#
        ),
    )?);

    insta::assert_snapshot!(test_config(
        r#"{ "removeHiddenElems": {} }"#,
        Some(
            r##"<svg xmlns="http://www.w3.org/2000/svg" xmlns:xlink="http://www.w3.org/1999/xlink">
    <!-- preserve defs with referenced path -->
    <g id="test-body-content">
        <defs>
            <path id="reference" d="M240 1h239v358H240z"/>
        </defs>
        <use xlink:href="#reference" id="use" fill="gray" onclick="test(evt)"/>
    </g>
</svg>"##
        ),
    )?);

    insta::assert_snapshot!(test_config(
        r#"{ "removeHiddenElems": {} }"#,
        Some(
            r##"<svg xmlns="http://www.w3.org/2000/svg" xmlns:xlink="http://www.w3.org/1999/xlink">
    <!-- preserve referenced path, even with zero opacity -->
    <defs>
        <path id="path2" d="M200 200 l50 -300" style="opacity:0"/>
    </defs>
    <text style="font-size:24px;">
        <textPath xlink:href="#path2">
        this is path 2
        </textPath>
    </text>
    <path id="path1" d="M200 200 l50 -300" style="opacity:0"/>
</svg>"##
        ),
    )?);

    Ok(())
}
