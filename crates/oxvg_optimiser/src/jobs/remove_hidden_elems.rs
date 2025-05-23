use std::{
    cell::RefCell,
    collections::{HashMap, HashSet},
    marker::PhantomData,
};

use derive_where::derive_where;
use lightningcss::{
    properties::{
        display::{Display, DisplayKeyword, Visibility},
        Property, PropertyId,
    },
    values::alpha::AlphaValue,
};
use oxvg_ast::{
    attribute::{Attr, Attributes},
    element::Element,
    get_computed_styles_factory,
    name::Name,
    style::{Id, PresentationAttr, PresentationAttrId, Static},
    visitor::{Context, ContextFlags, Info, PrepareOutcome, Visitor},
};
use oxvg_collections::collections::NON_RENDERING;
use oxvg_path::Path;
use serde::{Deserialize, Serialize};

use crate::utils::find_references;

#[cfg(feature = "wasm")]
use tsify::Tsify;

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

#[derive(Clone)]
#[derive_where(Default, Debug)]
struct Data<'arena, E: Element<'arena>> {
    opacity_zero: bool,
    non_rendered_nodes: RefCell<HashSet<E>>,
    removed_def_ids: RefCell<HashSet<String>>,
    all_defs: RefCell<HashSet<E>>,
    all_references: RefCell<HashSet<String>>,
    references_by_id: RefCell<HashMap<String, Vec<(E, E)>>>,
    marker: PhantomData<&'arena ()>,
}

struct State<'o, 'arena, E: Element<'arena>> {
    options: &'o RemoveHiddenElems,
    data: &'o mut Data<'arena, E>,
}

impl<'arena, E: Element<'arena>> Visitor<'arena, E> for Data<'arena, E> {
    type Error = String;

    fn prepare(
        &self,
        document: &E,
        _info: &Info<'arena, E>,
        context_flags: &mut ContextFlags,
    ) -> Result<PrepareOutcome, Self::Error> {
        context_flags.query_has_script(document);
        context_flags.query_has_stylesheet(document);
        Ok(PrepareOutcome::use_style)
    }

    fn use_style(&self, element: &E) -> bool {
        let name = element.qual_name().formatter().to_string();
        !NON_RENDERING.contains(&name)
    }

    fn element(
        &self,
        element: &mut E,
        context: &mut Context<'arena, '_, '_, E>,
    ) -> Result<(), Self::Error> {
        if !context.flags.contains(ContextFlags::use_style) {
            self.non_rendered_nodes.borrow_mut().insert(element.clone());
            context.flags.visit_skip();
            return Ok(());
        }

        let computed_styles = &context.computed_styles;
        get_computed_styles_factory!(computed_styles);
        if self.opacity_zero {
            if let Some(opacity) = get_computed_styles!(Opacity) {
                if opacity.is_static()
                    && matches!(
                        opacity.inner(),
                        Static::Attr(PresentationAttr::Opacity(AlphaValue(0.0)))
                            | Static::Css(Property::Opacity(AlphaValue(0.0)))
                    )
                {
                    let name = element.qual_name();
                    if name.prefix().is_none() && name.local_name().as_ref() == "path" {
                        self.non_rendered_nodes.borrow_mut().insert(element.clone());
                        context.flags.visit_skip();
                        return Ok(());
                    }
                    self.remove_element(element);
                }
            }
        }
        Ok(())
    }
}

impl<'arena, E: Element<'arena>> Data<'arena, E> {
    fn remove_element(&self, element: &E) {
        if let Some(parent) = Element::parent_element(element) {
            if parent.prefix().is_none() && parent.local_name().as_ref() == "defs" {
                if let Some(id) = element.get_attribute_local(&"id".into()) {
                    self.removed_def_ids.borrow_mut().insert(id.to_string());
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
}

impl<'arena, E: Element<'arena>> Visitor<'arena, E> for RemoveHiddenElems {
    type Error = String;

    fn prepare(
        &self,
        document: &E,
        info: &Info<'arena, E>,
        context_flags: &mut ContextFlags,
    ) -> Result<PrepareOutcome, Self::Error> {
        log::debug!("collecting data");
        context_flags.query_has_script(document);
        context_flags.query_has_stylesheet(document);
        let document = &mut document.clone();
        let mut data = Data {
            opacity_zero: self.opacity_zero.unwrap_or(true),
            ..Data::default()
        };
        data.start(document, info, Some(context_flags.clone()))?;
        log::debug!("data collected");
        State {
            options: self,
            data: &mut data,
        }
        .start(document, info, Some(context_flags.clone()))?;
        Ok(PrepareOutcome::skip)
    }
}

impl<'arena, E: Element<'arena>> Visitor<'arena, E> for State<'_, 'arena, E> {
    type Error = String;

    fn prepare(
        &self,
        _document: &E,
        _info: &Info<'arena, E>,
        _context_flags: &mut ContextFlags,
    ) -> Result<PrepareOutcome, Self::Error> {
        Ok(PrepareOutcome::use_style)
    }

    fn use_style(&self, _element: &E) -> bool {
        true
    }

    fn element(
        &self,
        element: &mut E,
        context: &mut Context<'arena, '_, '_, E>,
    ) -> Result<(), String> {
        let Some(parent) = Element::parent_element(element) else {
            return Ok(());
        };
        let name = element.qual_name().formatter().to_string();

        self.ref_element(element, &parent, &name);
        if self.is_hidden_style(element, &name, context)
            || self.is_hidden_ellipse(element, &name)
            || self.is_hidden_rect(element, &name)
            || self.is_hidden_pattern(element, &name)
            || self.is_hidden_image(element, &name)
            || self.is_hidden_path(element, &name, context)
            || self.is_hidden_poly(element, &name)
        {
            log::debug!("RemoveHiddenElems: removing hidden");
            self.data.remove_element(element);
            return Ok(());
        }

        for attr in element.attributes().into_iter() {
            let local_name = attr.local_name();
            let value = attr.value();

            let ids = find_references(local_name.as_ref(), value.as_ref());
            if let Some(ids) = ids {
                let mut all_references = self.data.all_references.borrow_mut();
                ids.filter_map(|id| id.get(1))
                    .map(|id| id.as_str().to_string())
                    .for_each(|id| {
                        all_references.insert(id);
                    });
            }
        }
        Ok(())
    }

    fn exit_document(
        &self,
        _document: &mut E,
        context: &Context<'arena, '_, '_, E>,
    ) -> Result<(), Self::Error> {
        for id in &*self.data.removed_def_ids.borrow() {
            if let Some(refs) = self.data.references_by_id.borrow().get(id) {
                for (node, _parent_node) in refs {
                    log::debug!("RemoveHiddenElems: remove referenced by id");
                    node.remove();
                }
            }
        }

        let deoptimized = context
            .flags
            .intersects(ContextFlags::has_stylesheet & ContextFlags::has_script_ref);
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

impl<'arena, E: Element<'arena>> State<'_, 'arena, E> {
    fn can_remove_non_rendering_node(&self, element: &E) -> bool {
        if let Some(id) = element.get_attribute_local(&"id".into()) {
            if self.data.all_references.borrow().contains(id.as_ref()) {
                return false;
            }
        }
        element
            .child_nodes_iter()
            .all(|e| E::new(e).is_none_or(|e| self.can_remove_non_rendering_node(&e)))
    }

    fn ref_element(&self, element: &E, parent: &E, name: &str) {
        if name == "defs" {
            self.data.all_defs.borrow_mut().insert(element.clone());
        } else if name == "use" {
            for attr in element.attributes().into_iter() {
                if attr.local_name().as_ref() != "href" {
                    continue;
                }
                let value = attr.value();
                let id = &value.as_ref()[1..];

                let mut references_by_id = self.data.references_by_id.borrow_mut();
                let refs = references_by_id.get_mut(id);
                match refs {
                    Some(refs) => refs.push((element.clone(), parent.clone())),
                    None => {
                        references_by_id
                            .insert(id.to_string(), vec![(element.clone(), parent.clone())]);
                    }
                }
            }
        }
    }

    fn is_hidden_style(
        &self,
        element: &E,
        name: &str,
        context: &Context<'arena, '_, '_, E>,
    ) -> bool {
        let computed_styles = &context.computed_styles;
        get_computed_styles_factory!(computed_styles);
        if self.options.is_hidden.unwrap_or(true) {
            if let Some(visibility) = get_computed_styles!(Visibility) {
                if visibility.is_static()
                    && matches!(
                        visibility.inner(),
                        Static::Attr(PresentationAttr::Visibility(Visibility::Hidden))
                            | Static::Css(Property::Visibility(Visibility::Hidden))
                    )
                    && element
                        .select("[visibility=visible]")
                        .unwrap()
                        .next()
                        .is_none()
                {
                    return true;
                }
            }
        }

        if self.options.display_none.unwrap_or(true) {
            if let Some(display) = get_computed_styles!(Display) {
                if display.is_static()
                    && matches!(
                        display.inner(),
                        Static::Attr(PresentationAttr::Display(Display::Keyword(
                            DisplayKeyword::None
                        ))) | Static::Css(Property::Display(Display::Keyword(
                            DisplayKeyword::None
                        )))
                    )
                    && name != "marker"
                {
                    return true;
                }
            }
        }
        false
    }

    fn is_hidden_ellipse(&self, element: &E, name: &str) -> bool {
        if name == "circle"
            && element.is_empty()
            && self.options.circle_r_zero.unwrap_or(true)
            && element
                .get_attribute_local(&"r".into())
                .is_some_and(|v| v.as_ref() == "0")
        {
            log::debug!("RemoveHiddenElement: removing hidden ellipse");
            element.remove();
            return true;
        }

        if name == "ellipse" && element.is_empty() {
            if self.options.ellipse_rx_zero.unwrap_or(true)
                && element
                    .get_attribute_local(&"rx".into())
                    .is_some_and(|v| v.as_ref() == "0")
            {
                return true;
            }

            if self.options.ellipse_ry_zero.unwrap_or(true)
                && element
                    .get_attribute_local(&"ry".into())
                    .is_some_and(|v| v.as_ref() == "0")
            {
                return true;
            }
        }
        false
    }

    fn is_hidden_rect(&self, element: &E, name: &str) -> bool {
        if name == "rect" && element.is_empty() && self.options.rect_width_zero.unwrap_or(true) {
            if element
                .get_attribute_local(&"width".into())
                .is_some_and(|v| v.as_ref() == "0")
            {
                return true;
            }
            if self.options.rect_height_zero.unwrap_or(true)
                && element
                    .get_attribute_local(&"height".into())
                    .is_some_and(|v| v.as_ref() == "0")
            {
                return true;
            }
        }
        false
    }

    fn is_hidden_pattern(&self, element: &E, name: &str) -> bool {
        if name == "pattern" {
            if self.options.pattern_width_zero.unwrap_or(true)
                && element
                    .get_attribute_local(&"width".into())
                    .is_some_and(|v| v.as_ref() == "0")
            {
                return true;
            }

            if self.options.pattern_height_zero.unwrap_or(true)
                && element
                    .get_attribute_local(&"height".into())
                    .is_some_and(|v| v.as_ref() == "0")
            {
                return true;
            }
        }
        false
    }

    fn is_hidden_image(&self, element: &E, name: &str) -> bool {
        if name == "image" {
            if self.options.image_width_zero.unwrap_or(true)
                && element
                    .get_attribute_local(&"width".into())
                    .is_some_and(|v| v.as_ref() == "0")
            {
                return true;
            }

            if self.options.image_height_zero.unwrap_or(true)
                && element
                    .get_attribute_local(&"height".into())
                    .is_some_and(|v| v.as_ref() == "0")
            {
                return true;
            }
        }
        false
    }

    fn is_hidden_path(
        &self,
        element: &E,
        name: &str,
        context: &Context<'arena, '_, '_, E>,
    ) -> bool {
        let computed_styles = &context.computed_styles;
        get_computed_styles_factory!(computed_styles);
        if self.options.path_empty_d.unwrap_or(true) && name == "path" {
            let Some(d) = element.get_attribute_local(&"d".into()) else {
                return true;
            };
            return match Path::parse(d.as_ref()) {
                Ok(d) => {
                    d.0.is_empty()
                        || (d.0.len() == 1
                            && get_computed_styles!(MarkerStart).is_none()
                            && get_computed_styles!(MarkerEnd).is_none())
                }
                Err(_) => true,
            };
        }
        false
    }

    fn is_hidden_poly(&self, element: &E, name: &str) -> bool {
        if self.options.polyline_empty_points.unwrap_or(true)
            && name == "polyline"
            && element.get_attribute_local(&"points".into()).is_none()
        {
            return true;
        }

        if self.options.polygon_empty_points.unwrap_or(true)
            && name == "polygon"
            && element.get_attribute_local(&"points".into()).is_none()
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
