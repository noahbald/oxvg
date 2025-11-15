use std::mem;

use lightningcss::{properties::PropertyId, vendor_prefix::VendorPrefix};
use oxvg_ast::{
    element::Element,
    get_attribute, has_attribute, is_element,
    visitor::{Context, PrepareOutcome, Visitor},
};
use oxvg_collections::{
    atom::Atom,
    attribute::{inheritable::Inheritable, Attr, AttrId},
    content_type::ContentType,
    element::ElementCategory,
};
use serde::{Deserialize, Serialize};

#[cfg(feature = "wasm")]
use tsify::Tsify;

use crate::error::JobsError;

#[cfg_attr(feature = "wasm", derive(Tsify))]
#[cfg_attr(feature = "napi", napi(object))]
#[derive(Deserialize, Serialize, Debug, Clone)]
#[serde(transparent)]
/// Filters `<g>` elements that have no effect.
///
/// For removing empty groups, see [`super::RemoveEmptyContainers`].
///
/// # Correctness
///
/// This job should never visually change the document.
///
/// # Errors
///
/// Never.
///
/// If this job produces an error or panic, please raise an [issue](https://github.com/noahbald/oxvg/issues)
pub struct CollapseGroups(pub bool);

impl<'input, 'arena> Visitor<'input, 'arena> for CollapseGroups {
    type Error = JobsError<'input>;

    fn prepare(
        &self,
        _document: &Element<'input, 'arena>,
        _context: &mut Context<'input, 'arena, '_>,
    ) -> Result<PrepareOutcome, Self::Error> {
        Ok(if self.0 {
            PrepareOutcome::none
        } else {
            PrepareOutcome::skip
        })
    }

    fn exit_element(
        &self,
        element: &Element<'input, 'arena>,
        _context: &mut Context<'input, 'arena, '_>,
    ) -> Result<(), Self::Error> {
        let Some(parent) = Element::parent_element(element) else {
            return Ok(());
        };

        if element.is_root() || is_element!(parent, Switch) {
            return Ok(());
        }
        if !is_element!(element, G) || !element.has_child_elements() {
            return Ok(());
        }

        move_attributes_to_child(element);
        flatten_when_all_attributes_moved(element);
        Ok(())
    }
}

impl Default for CollapseGroups {
    fn default() -> Self {
        Self(true)
    }
}

fn move_attributes_to_child(element: &Element) {
    log::debug!("collapse_groups: move_attributes_to_child");

    let children = element.children();
    let mut children = children.iter();
    let Some(first_child) = children.next() else {
        log::debug!("collapse_groups: not moving attrs: no children");
        return;
    };
    if children.next().is_some() {
        log::debug!("collapse_groups: not moving attrs: many children");
        return;
    }

    let attrs = element.attributes();
    if attrs.is_empty() {
        log::debug!("collapse_groups: not moving attrs: no attrs to move");
        return;
    }

    if is_group_identifiable(element, first_child) {
        log::debug!("collapse_groups: not moving attrs: identifiable");
        return;
    } else if is_position_visually_unstable(element, first_child) {
        log::debug!("collapse_groups: not moving attrs: visually unstable");
        return;
    } else if is_node_with_filter(element) {
        log::debug!("collapse_groups: not moving attrs: filter");
        return;
    }

    let mut removals = Vec::default();
    let first_child_attrs = first_child.attributes();
    for mut attr in attrs.into_iter_mut() {
        let name = attr.name().clone();
        let child_attr = first_child_attrs.get_named_item_mut(&name);
        if has_animated_attr(first_child, name.local_name()) {
            log::debug!("collapse_groups: canelled moves: has animated_attr");
            return;
        }

        removals.push(name);
        let Some(mut child_attr) = child_attr else {
            log::debug!("collapse_groups: moved {attr:?}: same as parent",);
            first_child_attrs.set_named_item(attr.clone());
            continue;
        };

        if let Attr::Transform(Inheritable::Defined(value)) = &mut *attr {
            let Attr::Transform(Inheritable::Defined(child_value)) = &mut *child_attr else {
                continue;
            };
            log::debug!("collapse_groups: moved transform: is transform");
            value.0.extend(mem::take(&mut child_value.0));
            mem::swap(&mut value.0, &mut child_value.0);
        } else if let ContentType::Inheritable(inheritable) = child_attr.value() {
            if Inheritable::Inherited == inheritable {
                log::debug!("collapse_groups: moved {attr:?}: is explicit inherit");
                *child_attr = attr.clone();
            }
        } else if *attr != *child_attr {
            log::debug!("collapse_groups: removing {attr:?}: inheritable attr is not inherited");
            removals.pop();
            break;
        }
    }

    for attr in removals {
        element.remove_attribute(&attr);
    }
}

fn flatten_when_all_attributes_moved(element: &Element) {
    if !element.attributes().is_empty() {
        log::debug!("skipping flatten: has attributes");
        return;
    }

    {
        if element.breadth_first().any(|child| {
            child
                .qual_name()
                .categories()
                .contains(ElementCategory::Animation)
        }) {
            log::debug!("skipping flatten: has animating child");
            return;
        }
    }

    element.flatten();
}

fn has_animated_attr<'input>(element: &Element<'input, '_>, local_name: &Atom<'input>) -> bool {
    for child in std::iter::once(element.clone()).chain(element.breadth_first()) {
        if child
            .qual_name()
            .categories()
            .intersects(ElementCategory::Animation)
            && get_attribute!(child, AttributeName).is_some_and(|attr| &*attr == local_name)
        {
            return true;
        }
    }
    false
}

fn is_group_identifiable<'input, 'arena>(
    node: &Element<'input, 'arena>,
    child: &Element<'input, 'arena>,
) -> bool {
    child.has_attribute(&AttrId::Id)
        && (!has_attribute!(node, Class) || !has_attribute!(child, Class))
}

fn is_position_visually_unstable<'input, 'arena>(
    node: &Element<'input, 'arena>,
    child: &Element<'input, 'arena>,
) -> bool {
    let is_node_clipping = has_attribute!(node, ClipPath | Mask);
    let is_child_transformed_group = is_element!(child, G) && has_attribute!(child, Transform);
    is_node_clipping || is_child_transformed_group
}

fn is_node_with_filter(node: &Element) -> bool {
    has_attribute!(node, Filter)
        || get_attribute!(node, Style)
            .is_some_and(|style| style.get(&PropertyId::Filter(VendorPrefix::None)).is_some())
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
