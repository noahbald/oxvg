use lightningcss::{
    properties::PropertyId,
    stylesheet::{ParserOptions, StyleAttribute},
    vendor_prefix::VendorPrefix,
};
use oxvg_ast::{
    attribute::{Attr, Attributes},
    element::Element,
    name::Name,
    node::Node,
};
use oxvg_derive::OptionalDefault;
use oxvg_selectors::collections::{ElementGroup, Group, INHERITABLE_ATTRS};
use serde::Deserialize;

use crate::{Context, Job, JobDefault, PrepareOutcome};

#[derive(Deserialize, Clone, OptionalDefault)]
#[serde(rename_all = "camelCase")]
pub struct CollapseGroups(bool);

impl Job for CollapseGroups {
    fn prepare(&mut self, _document: &impl Node) -> crate::PrepareOutcome {
        if self.0 {
            PrepareOutcome::None
        } else {
            PrepareOutcome::Skip
        }
    }

    fn run(&self, element: &impl Element, _context: &Context) {
        let Some(parent) = Element::parent_element(element) else {
            return;
        };

        if element.is_root() || parent.local_name().as_ref() == "switch" {
            return;
        }
        if element.local_name().as_ref() != "g" || !element.has_child_elements() {
            return;
        }

        move_attributes_to_child(element);
        flatten_when_all_attributes_moved(element);
    }
}

impl Default for CollapseGroups {
    fn default() -> Self {
        Self(true)
    }
}

fn move_attributes_to_child(element: &impl Element) {
    log::debug!("collapse_groups: move_attributes_to_child");

    let mut children = element.children_iter();
    let Some(first_child) = children.next() else {
        log::debug!("collapse_groups: not moving attrs: no children");
        return;
    };
    if children.next().is_some() {
        log::debug!("collapse_groups: not moving attrs: many children");
        return;
    }

    let attrs = element.attributes();
    if attrs.len() == 0 {
        log::debug!("collapse_groups: not moving attrs: no attrs to move");
        return;
    }

    if dbg!(is_group_identifiable(element, &first_child))
        || dbg!(is_position_visually_unstable(element, &first_child))
        || dbg!(is_node_with_filter(element))
    {
        log::debug!("collapse_groups: not moving attrs: see true condition");
        return;
    };

    let mut removals = Vec::default();
    let first_child_attrs = first_child.attributes();
    for attr in attrs.iter() {
        let name = attr.name();
        let local_name = name.local_name();
        let local_name: &str = local_name.as_ref();
        let value = attr.value();

        let child_attr = first_child_attrs.get_named_item(&name);
        if has_animated_attr(&first_child, &name) {
            log::debug!("collapse_groups: canelled moves: has animated_attr");
            return;
        }

        removals.push(attr.name());
        let Some(mut child_attr) = child_attr else {
            log::debug!("collapse_groups: moved {name}: same as parent",);
            first_child_attrs.set_named_item(attr.into_owned());
            continue;
        };

        let child_attr_value = child_attr.value();
        if name.local_name().as_ref() == "transform" {
            log::debug!("collapse_groups: moved transform: is transform");
            child_attr.set_value(format!("{value} {child_attr_value}").into());
        } else if child_attr_value.as_ref() == "inherit" {
            log::debug!("collapse_groups: moved {name}: is explicit inherit");
            child_attr.set_value(value);
        } else if !INHERITABLE_ATTRS.contains(local_name) && child_attr.value() != value {
            log::debug!("collapse_groups: removing {name}: inheritable attr is not inherited");
            return;
        }
    }

    for attr in removals {
        element.remove_attribute(&attr);
    }
}

fn flatten_when_all_attributes_moved(element: &impl Element) {
    if !element.attributes().is_empty() {
        log::debug!("skipping flatten: has attributes");
        return;
    }

    let animation_group = oxvg_selectors::collections::Group::set(&ElementGroup::Animation);
    {
        if element.depth_first().any(|child| {
            let local_name = child.local_name();
            let name: &str = local_name.as_ref();
            dbg!(animation_group.contains(dbg!(name)))
        }) {
            log::debug!("skipping flatten: has animating child");
            return;
        }
    }

    element.flatten();
}

fn has_animated_attr(element: &impl Element, name: &impl Name) -> bool {
    let local_name = name.local_name();
    let node_name: &str = local_name.as_ref();
    for child in std::iter::once(element.clone()).chain(element.depth_first()) {
        let child_name_local = child.local_name();
        let child_name: &str = child_name_local.as_ref();
        if Group::set(&ElementGroup::Animation).contains(child_name)
            && child
                .get_attribute_local(&"attributeName".into())
                .is_some_and(|attr| attr.as_ref() == node_name)
        {
            return true;
        }
    }
    false
}

fn is_group_identifiable<E: Element>(node: &E, child: &E) -> bool {
    let class = &"class".into();
    child.has_attribute_local(&"id".into())
        && (!node.has_attribute_local(class) || !child.has_attribute_local(class))
}

fn is_position_visually_unstable<E: Element>(node: &E, child: &E) -> bool {
    let is_node_clipping =
        node.has_attribute_local(&"clip-path".into()) || node.has_attribute_local(&"mask".into());
    let is_child_transformed_group =
        child.local_name().as_ref() == "g" && child.has_attribute_local(&"transform".into());
    is_node_clipping || is_child_transformed_group
}

fn is_node_with_filter(node: &impl Element) -> bool {
    node.has_attribute_local(&"filter".into())
        || node
            .get_attribute_local(&"style".into())
            .is_some_and(|code| {
                StyleAttribute::parse(code.as_ref(), ParserOptions::default()).is_ok_and(|style| {
                    style
                        .declarations
                        .get(&PropertyId::Filter(VendorPrefix::None))
                        .is_some()
                })
            })
}

#[test]
#[allow(clippy::too_many_lines)]
fn collapse_groups() -> anyhow::Result<()> {
    use crate::test_config;

    insta::assert_snapshot!(test_config(
        r#"{ "collapseGroups": true }"#,
        Some(
            r#"<svg xmlns="http://www.w3.org/2000/svg">
    <!-- Should remove both useless `g`s -->
    <g>
        <g>
            <path d="..."/>
        </g>
    </g>
</svg>"#
        )
    )?);

    insta::assert_snapshot!(test_config(
        r#"{ "collapseGroups": true }"#,
        Some(
            r#"<svg xmlns="http://www.w3.org/2000/svg">
    <!-- Should pass all inheritable attributes to children -->
    <g>
        <g attr1="val1">
            <path d="..."/>
        </g>
    </g>
    <g attr1="val1">
        <g attr2="val2">
            <path d="..."/>
        </g>
    </g>
    <g attr1="val1">
        <g>
            <path d="..."/>
        </g>
        <path d="..."/>
    </g>
    <g attr1="val1">
        <g attr2="val2">
            <path d="..."/>
        </g>
        <path d="..."/>
    </g>
</svg>"#
        )
    )?);

    insta::assert_snapshot!(test_config(
        r#"{ "collapseGroups": true }"#,
        Some(
            r#"<svg xmlns="http://www.w3.org/2000/svg">
    <!-- Should remove inheritable overridden attributes -->
    <g attr1="val1">
        <g fill="red">
            <path fill="green" d="..."/>
        </g>
        <path d="..."/>
    </g>
</svg>"#
        )
    )?);

    insta::assert_snapshot!(test_config(
        r#"{ "collapseGroups": true }"#,
        Some(
            r#"<svg xmlns="http://www.w3.org/2000/svg">
    <!-- Should remove group with equal attribute values to child -->
    <g attr1="val1">
        <g attr2="val2">
            <path attr2="val2" d="..."/>
        </g>
        <g attr2="val2">
            <path attr2="val3" d="..."/>
        </g>
    </g>
</svg>"#
        )
    )?);

    insta::assert_snapshot!(test_config(
        r#"{ "collapseGroups": true }"#,
        Some(
            r#"<svg xmlns="http://www.w3.org/2000/svg">
    <!-- Should join transform attributes into `transform="rotate(45) scale(2)"` -->
    <g attr1="val1">
        <g transform="rotate(45)">
            <path transform="scale(2)" d="..."/>
        </g>
        <path d="..."/>
    </g>
</svg>"#
        )
    )?);

    insta::assert_snapshot!(test_config(
        r#"{ "collapseGroups": true }"#,
        Some(
            r#"<svg xmlns="http://www.w3.org/2000/svg">
    <!-- Should preserve groups with `clip-path` -->
    <clipPath id="a">
       <path d="..."/>
    </clipPath>
    <clipPath id="b">
       <path d="..."/>
    </clipPath>
    <g transform="matrix(0 -1.25 -1.25 0 100 100)" clip-path="url(#a)">
        <g transform="scale(.2)">
            <path d="..."/>
            <path d="..."/>
        </g>
    </g>
    <g transform="matrix(0 -1.25 -1.25 0 100 100)" clip-path="url(#a)">
        <g transform="scale(.2)">
            <g>
                <g clip-path="url(#b)">
                    <path d="..."/>
                    <path d="..."/>
                </g>
            </g>
        </g>
    </g>
</svg>"#
        )
    )?);

    insta::assert_snapshot!(test_config(
        r#"{ "collapseGroups": true }"#,
        Some(
            r#"<svg xmlns="http://www.w3.org/2000/svg">
    <!-- Should preserve groups with `clip-path` and `mask` -->
    <clipPath id="a">
       <path d="..."/>
    </clipPath>
    <path d="..."/>
    <g clip-path="url(#a)">
        <path d="..." transform="scale(.2)"/>
    </g>
    <g mask="url(#a)">
        <path d="..." transform="scale(.2)"/>
    </g>
</svg>"#
        )
    )?);

    insta::assert_snapshot!(test_config(
        r#"{ "collapseGroups": true }"#,
        Some(
            r##"<svg xmlns="http://www.w3.org/2000/svg">
    <!-- Should preserve groups with `id` or animation children -->
    <g stroke="#000">
        <g id="star">
            <path id="bar" d="..."/>
        </g>
    </g>
    <g>
        <animate id="frame0" attributeName="visibility" values="visible" dur="33ms" begin="0s;frame27.end"/>
        <path d="..." fill="#272727"/>
        <path d="..." fill="#404040"/>
        <path d="..." fill="#2d2d2d"/>
    </g>
    <g transform="rotate(-90 25 0)">
        <circle stroke-dasharray="110" r="20" stroke="#10cfbd" fill="none" stroke-width="3" stroke-linecap="round">
            <animate attributeName="stroke-dashoffset" values="360;140" dur="2.2s" keyTimes="0;1" calcMode="spline" fill="freeze" keySplines="0.41,0.314,0.8,0.54" repeatCount="indefinite" begin="0"/>
            <animateTransform attributeName="transform" type="rotate" values="0;274;360" keyTimes="0;0.74;1" calcMode="linear" dur="2.2s" repeatCount="indefinite" begin="0"/>
        </circle>
    </g>
</svg>"##
        )
    )?);

    insta::assert_snapshot!(test_config(
        r#"{ "collapseGroups": true }"#,
        Some(
            r#"<svg xmlns="http://www.w3.org/2000/svg">
    <!-- Should preserve groups with classes -->
    <style>
        .n{display:none}
        .i{display:inline}
    </style>
    <g id="a">
        <g class="i"/>
    </g>
    <g id="b" class="n">
        <g class="i"/>
    </g>
</svg>"#
        )
    )?);

    insta::assert_snapshot!(test_config(
        r#"{ "collapseGroups": true }"#,
        Some(
            r#"<svg xmlns="http://www.w3.org/2000/svg">
    <!-- Should preserve children of `<switch>` -->
    <switch>
        <g id="a">
            <g class="i"/>
        </g>
        <g id="b" class="n">
            <g class="i"/>
        </g>
        <g>
            <g/>
        </g>
    </switch>
</svg>"#
        )
    )?);

    insta::assert_snapshot!(test_config(
        r#"{ "collapseGroups": true }"#,
        Some(
            r#"<svg xmlns="http://www.w3.org/2000/svg">
    <!-- Should replace inheritable value -->
	<g color="red">
		<g color="inherit" fill="none" stroke="none">
			<circle cx="130" cy="80" r="60" fill="currentColor"/>
			<circle cx="350" cy="80" r="60" stroke="currentColor" stroke-width="4"/>
		</g>
	</g>
</svg>"#
        )
    )?);

    insta::assert_snapshot!(test_config(
        r#"{ "collapseGroups": true }"#,
        Some(
            r#"<svg xmlns="http://www.w3.org/2000/svg">
    <!-- Should remove useless group -->
    <g filter="url(#...)">
        <g>
            <path d="..."/>
        </g>
    </g>
</svg>"#
        )
    )?);

    insta::assert_snapshot!(test_config(
        r#"{ "collapseGroups": true }"#,
        Some(
            r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 88 88">
  <!-- Should preserve group if some attrs cannot be moved -->
  <filter id="a">
    <feGaussianBlur stdDeviation="1"/>
  </filter>
  <g transform="matrix(0.6875,0,0,0.6875,20.34375,66.34375)" style="filter:url(#a)">
    <path d="M 33.346591,-83.471591 L -10.744318,-36.471591 L -10.49989,-32.5" style="fill-opacity:1"/>
  </g>
</svg>"#
        )
    )?);

    insta::assert_snapshot!(test_config(
        r#"{ "collapseGroups": true }"#,
        Some(
            r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 100 100">
    <!-- Should preserve group if parent has `filter` -->
    <clipPath id="a">
        <circle cx="25" cy="15" r="10"/>
    </clipPath>
    <filter id="b">
        <feColorMatrix type="saturate"/>
    </filter>
    <g filter="url(#b)">
        <g clip-path="url(#a)">
            <circle cx="30" cy="10" r="10" fill="yellow" id="c1"/>
        </g>
    </g>
    <g style="filter:url(#b)">
        <g clip-path="url(#a)">
            <circle cx="20" cy="10" r="10" fill="blue" id="c2"/>
        </g>
    </g>
    <circle cx="25" cy="15" r="10" stroke="black" stroke-width=".1" fill="none"/>
</svg>"#
        )
    )?);

    Ok(())
}
