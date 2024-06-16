use std::cell::RefCell;

use lightningcss::{
    properties::PropertyId,
    stylesheet::{ParserOptions, StyleAttribute},
    vendor_prefix::VendorPrefix,
};
use markup5ever::{local_name, Attribute};
use oxvg_ast::{node, Attributes};
use oxvg_selectors::{
    collections::{ElementGroup, Group, INHERITABLE_ATTRS},
    Element, ElementData,
};
use serde::Deserialize;

use crate::{Job, PrepareOutcome};

#[derive(Deserialize, Default, Clone)]
#[serde(rename_all = "camelCase")]
pub struct CollapseGroups(bool);

impl Job for CollapseGroups {
    fn prepare(&mut self, _document: &rcdom::RcDom) -> crate::PrepareOutcome {
        if self.0 {
            PrepareOutcome::None
        } else {
            PrepareOutcome::Skip
        }
    }

    fn run(&self, node: &std::rc::Rc<rcdom::Node>) {
        use rcdom::NodeData::Element as ElementData;

        let ElementData { attrs, .. } = &node.data else {
            return;
        };
        let element = Element::new(node.clone());
        let Some(parent) = element.get_parent() else {
            return;
        };

        if element.is_root() || parent.get_name() == Some(local_name!("switch")) {
            return;
        }
        if element.get_name() != Some(local_name!("g")) || node.children.borrow().len() == 0 {
            return;
        }

        move_attributes_to_child(&element, attrs);
        flatten_when_all_attributes_moved(&element, attrs);
    }
}

fn move_attributes_to_child(element: &Element, attrs: &RefCell<Vec<Attribute>>) {
    dbg!("collapse_groups: move_attributes_to_child");
    if attrs.borrow().len() == 0 {
        dbg!("collapse_groups: not moving attrs: no attrs to move");
        return;
    }

    let children: Vec<_> = element.children().collect();
    if children.len() != 1 {
        dbg!("collapse_groups: not moving attrs: many children");
        return;
    }

    let child = children.first().expect("No child after checking length");
    let ElementData {
        attrs: child_attrs, ..
    } = &child.data();

    if dbg!(is_group_identifiable(element, child))
        || dbg!(is_position_visually_unstable(element, child))
        || dbg!(is_node_with_filter(element))
    {
        dbg!("collapse_groups: not moving attrs: see true condition");
        return;
    };

    let mut new_node_attrs = Attributes::default();
    let mut new_child_attrs = Attributes::from(&*child_attrs.borrow());
    for attr in attrs.borrow().iter() {
        let Attribute { name, value } = attr.clone();
        let name = node::QualName(name.clone());

        if has_animated_attr(child, &name) {
            dbg!("collapse_groups: canelled moves: has animated_attr");
            return;
        }

        let Some(old_value) = new_child_attrs.get(&name) else {
            dbg!("collapse_groups: moved {}: same as parent", &name.0.local);
            new_child_attrs.insert(name, value);
            continue;
        };

        if name.0.local == local_name!("transform") {
            dbg!("collapse_groups: moved transform: is transform");
            new_child_attrs.insert(name, format!("{value} {old_value}").into());
        } else if old_value.to_string() == *"inherit" {
            dbg!(
                "collapse_groups: moved {}: is explicit inherit",
                &name.0.local
            );
            new_child_attrs.insert(name, value);
        } else if INHERITABLE_ATTRS.contains(name.0.local.as_ref())
            && new_child_attrs.get(&name).is_none()
        {
            dbg!("collapse_groups: canelled moves: inheritable attr is applicable");
            return;
        } else if INHERITABLE_ATTRS.contains(name.0.local.as_ref())
            || new_child_attrs.get(&name) == Some(&attr.value)
        {
            dbg!("collapse_groups: removing {}: inheritable attr is not inherited");
        } else {
            dbg!(
                "collapse_groups: ignoring {}: fails to meet movement criteria",
                &name.0.local,
            );
            new_node_attrs.insert(name, value);
        }
    }

    attrs.replace(new_node_attrs.into());
    let attrs = child.attrs();
    attrs.replace(
        new_child_attrs
            .iter()
            .map(|(name, value)| Attribute {
                name: name.0.clone(),
                value: value.clone(),
            })
            .collect(),
    );
}

fn flatten_when_all_attributes_moved(element: &Element, attrs: &RefCell<Vec<Attribute>>) {
    if attrs.borrow().len() > 0 {
        return;
    }

    let animation_group = oxvg_selectors::collections::Group::set(&ElementGroup::Animation);
    {
        let children: Vec<_> = element.children().collect();
        if children.iter().any(|child| {
            child
                .get_name()
                .is_some_and(|name| animation_group.contains(name.to_string().as_str()))
        }) {
            return;
        }
    }

    element.flatten();
}

fn has_animated_attr(node: &Element, name: &node::QualName) -> bool {
    let Some(node_name) = node.get_name() else {
        return false;
    };
    if Group::set(&ElementGroup::Animation).contains(node_name.as_ref())
        && node
            .get_attr(&local_name!("attributeName"))
            .is_some_and(|attr| attr.value.to_string() == name.to_string())
    {
        return true;
    }
    node.node
        .children
        .borrow()
        .iter()
        .any(|child| has_animated_attr(&Element::new(child.clone()), name))
}

fn is_group_identifiable(node: &Element, child: &Element) -> bool {
    child.get_attr(&local_name!("id")).is_some()
        && (node.get_attr(&local_name!("class")).is_none()
            || child.get_attr(&local_name!("class")).is_none())
}

fn is_position_visually_unstable(node: &Element, child: &Element) -> bool {
    let is_node_clipping = node.get_attr(&local_name!("clip-path")).is_some()
        || node.get_attr(&local_name!("mask")).is_some();
    let is_child_transformed_group = child.get_name() == Some(local_name!("g"))
        && child.get_attr(&local_name!("transform")).is_some();
    dbg!(is_node_clipping) || dbg!(is_child_transformed_group)
}

fn is_node_with_filter(node: &Element) -> bool {
    node.get_attr(&local_name!("filter")).is_some()
        || node.get_attr(&local_name!("style")).is_some_and(|code| {
            StyleAttribute::parse(code.value.as_ref(), ParserOptions::default()).is_ok_and(
                |style| {
                    dbg!(&style.declarations);
                    style
                        .declarations
                        .get(&PropertyId::Filter(VendorPrefix::None))
                        .is_some()
                },
            )
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
