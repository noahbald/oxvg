use std::collections::{HashMap, HashSet};

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
    visitor::{Context, ContextFlags, PrepareOutcome, Visitor},
};
use oxvg_collections::collections::NON_RENDERING;
use oxvg_path::Path;
use serde::Deserialize;

use crate::utils::find_references;

#[derive(Clone, Default, Deserialize)]
pub struct Options {
    is_hidden: Option<bool>,
    display_none: Option<bool>,
    opacity_zero: Option<bool>,
    circle_r_zero: Option<bool>,
    ellipse_rx_zero: Option<bool>,
    ellipse_ry_zero: Option<bool>,
    rect_width_zero: Option<bool>,
    rect_height_zero: Option<bool>,
    pattern_width_zero: Option<bool>,
    pattern_height_zero: Option<bool>,
    image_width_zero: Option<bool>,
    image_height_zero: Option<bool>,
    path_empty_d: Option<bool>,
    polyline_empty_points: Option<bool>,
    polygon_empty_points: Option<bool>,
}

#[derive(Clone)]
#[derive_where(Default)]
pub struct RemoveHiddenElems<E: Element> {
    options: Options,
    data: Data<E>,
}

#[derive(Clone)]
#[derive_where(Default)]
struct Data<E: Element> {
    opacity_zero: bool,
    non_rendered_nodes: HashSet<E>,
    removed_def_ids: HashSet<String>,
    all_defs: HashSet<E>,
    all_references: HashSet<String>,
    references_by_id: HashMap<String, Vec<(E, E)>>,
}

impl<E: Element> Visitor<E> for Data<E> {
    type Error = String;

    fn prepare(&mut self, document: &E, context_flags: &mut ContextFlags) -> super::PrepareOutcome {
        context_flags.query_has_script(document);
        context_flags.query_has_stylesheet(document);
        PrepareOutcome::use_style
    }

    fn use_style(&self, element: &E) -> bool {
        let name = element.qual_name().to_string();
        !NON_RENDERING.contains(&name)
    }

    fn element(&mut self, element: &mut E, context: &mut Context<E>) -> Result<(), Self::Error> {
        if !context.flags.contains(ContextFlags::use_style) {
            self.non_rendered_nodes.insert(element.clone());
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
                        self.non_rendered_nodes.insert(element.clone());
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

impl<E: Element> Data<E> {
    fn remove_element(&mut self, element: &E) {
        if let Some(id) = element.get_attribute(&"id".into()) {
            if let Some(parent) = Element::parent_element(element) {
                if parent.prefix().is_none() && parent.local_name().as_ref() == "defs" {
                    self.removed_def_ids.insert(id.to_string());
                }
            }
        }
        element.remove();
    }
}

impl<E: Element> Visitor<E> for RemoveHiddenElems<E> {
    type Error = String;

    fn prepare(&mut self, document: &E, context_flags: &mut ContextFlags) -> super::PrepareOutcome {
        context_flags.query_has_script(document);
        context_flags.query_has_stylesheet(document);
        PrepareOutcome::use_style
    }

    fn document(&mut self, document: &mut E) -> Result<(), Self::Error> {
        self.data.start(document).map(|_| ())
    }

    fn use_style(&self, _element: &E) -> bool {
        true
    }

    fn element(&mut self, element: &mut E, context: &mut Context<E>) -> Result<(), String> {
        let Some(parent) = Element::parent_element(element) else {
            return Ok(());
        };
        let name = element.qual_name().to_string();

        self.ref_element(element, &parent, &name);
        if self.is_hidden_style(element, &name, context)
            || self.is_hidden_ellipse(element, &name)
            || self.is_hidden_rect(element, &name)
            || self.is_hidden_pattern(element, &name)
            || self.is_hidden_image(element, &name)
            || self.is_hidden_path(element, &name, context)
            || self.is_hidden_poly(element, &name)
        {
            self.data.remove_element(element);
            return Ok(());
        }

        for attr in element.attributes().iter() {
            let local_name = attr.local_name();
            let value = attr.value();

            let ids = find_references(local_name.as_ref(), value.as_ref());
            if let Some(ids) = ids {
                ids.filter_map(|id| id.get(1))
                    .map(|id| id.as_str().to_string())
                    .for_each(|id| {
                        self.data.all_references.insert(id);
                    });
            }
        }
        Ok(())
    }

    fn exit_document(
        &mut self,
        _document: &mut E,
        context: &Context<E>,
    ) -> Result<(), Self::Error> {
        for id in &self.data.removed_def_ids {
            if let Some(refs) = self.data.references_by_id.get(id) {
                for (node, _parent_node) in refs {
                    node.remove();
                }
            }
        }

        let deoptimized = context
            .flags
            .intersects(ContextFlags::has_stylesheet & ContextFlags::has_script_ref);
        if !deoptimized {
            for non_rendered_node in &self.data.non_rendered_nodes {
                if self.can_remove_non_rendering_node(non_rendered_node) {
                    non_rendered_node.remove();
                }
            }
        }

        for node in &self.data.all_defs {
            if node.is_empty() {
                node.remove();
            }
        }

        Ok(())
    }
}

impl<E: Element> RemoveHiddenElems<E> {
    fn can_remove_non_rendering_node(&self, element: &E) -> bool {
        if let Some(id) = element.get_attribute(&"id".into()) {
            if self.data.all_references.contains(id.as_ref()) {
                return false;
            }
        }
        element.all_children(|e| E::new(e).is_none_or(|e| self.can_remove_non_rendering_node(&e)))
    }

    fn ref_element(&mut self, element: &E, parent: &E, name: &str) {
        if name == "defs" {
            self.data.all_defs.insert(element.clone());
        } else if name == "use" {
            for attr in element.attributes().iter() {
                if attr.local_name().as_ref() != "href" {
                    continue;
                }
                let value = attr.value();
                let id = &value.as_ref()[1..];

                let refs = self.data.references_by_id.get_mut(id);
                match refs {
                    Some(refs) => refs.push((element.clone(), parent.clone())),
                    None => {
                        self.data
                            .references_by_id
                            .insert(id.to_string(), vec![(element.clone(), parent.clone())]);
                    }
                }
            }
        }
    }

    fn is_hidden_style(&self, element: &E, name: &str, context: &Context<E>) -> bool {
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
                .get_attribute(&"r".into())
                .is_some_and(|v| v.as_ref() == "0")
        {
            element.remove();
            return true;
        }

        if name == "ellipse" && element.is_empty() {
            if self.options.ellipse_rx_zero.unwrap_or(true)
                && element
                    .get_attribute(&"rx".into())
                    .is_some_and(|v| v.as_ref() == "0")
            {
                return true;
            }

            if self.options.ellipse_ry_zero.unwrap_or(true)
                && element
                    .get_attribute(&"ry".into())
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
                .get_attribute(&"width".into())
                .is_some_and(|v| v.as_ref() == "0")
            {
                return true;
            }
            if self.options.rect_height_zero.unwrap_or(true)
                && element
                    .get_attribute(&"height".into())
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
                    .get_attribute(&"width".into())
                    .is_some_and(|v| v.as_ref() == "0")
            {
                return true;
            }

            if self.options.pattern_height_zero.unwrap_or(true)
                && element
                    .get_attribute(&"height".into())
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
                    .get_attribute(&"width".into())
                    .is_some_and(|v| v.as_ref() == "0")
            {
                return true;
            }

            if self.options.image_height_zero.unwrap_or(true)
                && element
                    .get_attribute(&"height".into())
                    .is_some_and(|v| v.as_ref() == "0")
            {
                return true;
            }
        }
        false
    }

    fn is_hidden_path(&self, element: &E, name: &str, context: &Context<E>) -> bool {
        let computed_styles = &context.computed_styles;
        get_computed_styles_factory!(computed_styles);
        if self.options.path_empty_d.unwrap_or(true) && name == "path" {
            let Some(d) = element.get_attribute(&"d".into()) else {
                return true;
            };
            return match Path::parse(d) {
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
            && element.get_attribute(&"points".into()).is_none()
        {
            return true;
        }

        if self.options.polygon_empty_points.unwrap_or(true)
            && name == "polygon"
            && element.get_attribute(&"points".into()).is_none()
        {
            return true;
        }
        false
    }
}

impl<'de, E: Element> Deserialize<'de> for RemoveHiddenElems<E> {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let options = Options::deserialize(deserializer)?;
        let opacity_zero = options.opacity_zero.unwrap_or(true);
        Ok(RemoveHiddenElems {
            options,
            data: Data {
                opacity_zero,
                ..Data::default()
            },
        })
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
