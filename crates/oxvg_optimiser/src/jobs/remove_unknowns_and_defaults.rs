use std::sync::LazyLock;

use oxvg_ast::{
    has_attribute, is_attribute,
    node::{self, Ref},
    style::{ComputedStyles, Mode},
    visitor::{Context, ContextFlags, PrepareOutcome},
};

use oxvg_ast::{element::Element, visitor::Visitor};
use oxvg_collections::{
    attribute::{AttributeGroup, AttributeInfo},
    content_type::ContentTypeId,
    element::ElementId,
    is_prefix,
    name::Prefix,
};
use serde::{Deserialize, Serialize};

#[cfg(feature = "wasm")]
use tsify::Tsify;

use crate::error::JobsError;

#[cfg_attr(feature = "wasm", derive(Tsify))]
#[cfg_attr(feature = "napi", napi(object))]
#[derive(Deserialize, Serialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
#[allow(clippy::struct_excessive_bools)]
/// Removes elements and attributes that are not expected in an SVG document. Removes
/// attributes that are not expected on a given element. Removes attributes that are
/// the default for a given element. Removes elements that are not expected as a child
/// for a given element.
///
/// # Differences to SVGO
///
/// SVGO will avoid removing `<tspan>` from `<a>`, whereas we will remove it.
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
pub struct RemoveUnknownsAndDefaults {
    #[serde(default = "default_unknown_content")]
    /// Whether to remove elements that are unknown or unknown for it's parent element.
    pub unknown_content: bool,
    #[serde(default = "default_unknown_attrs")]
    /// Whether to remove attributes that are unknown or unknown for it's element.
    pub unknown_attrs: bool,
    #[serde(default = "default_default_attrs")]
    /// Whether to remove attributes that are equivalent to the default for it's element.
    pub default_attrs: bool,
    #[serde(default = "default_default_markup_declarations")]
    /// Whether to remove xml declarations equivalent to the default.
    pub default_markup_declarations: bool,
    #[serde(default = "default_useless_overrides")]
    /// Whether to remove attributes equivalent to it's inherited value.
    pub useless_overrides: bool,
    #[serde(default = "default_keep_data_attrs")]
    /// Whether to keep attributes prefixed with `data-`
    pub keep_data_attrs: bool,
    #[serde(default = "default_keep_aria_attrs")]
    /// Whether to keep attributes prefixed with `aria-`
    pub keep_aria_attrs: bool,
    #[serde(default = "default_keep_role_attr")]
    /// Whether to keep the `role` attribute
    pub keep_role_attr: bool,
}

impl Default for RemoveUnknownsAndDefaults {
    fn default() -> Self {
        RemoveUnknownsAndDefaults {
            unknown_content: default_unknown_content(),
            unknown_attrs: default_unknown_attrs(),
            default_attrs: default_default_attrs(),
            default_markup_declarations: default_default_markup_declarations(),
            useless_overrides: default_useless_overrides(),
            keep_data_attrs: default_keep_data_attrs(),
            keep_aria_attrs: default_keep_aria_attrs(),
            keep_role_attr: default_keep_role_attr(),
        }
    }
}

impl<'input, 'arena> Visitor<'input, 'arena> for RemoveUnknownsAndDefaults {
    type Error = JobsError<'input>;

    fn prepare(
        &self,
        document: &Element<'input, 'arena>,
        context: &mut Context<'input, 'arena, '_>,
    ) -> Result<PrepareOutcome, Self::Error> {
        context.query_has_stylesheet(document);
        Ok(PrepareOutcome::none)
    }

    fn processing_instruction(
        &self,
        processing_instruction: Ref<'input, 'arena>,
        context: &Context<'input, 'arena, '_>,
    ) -> Result<(), Self::Error> {
        if !self.default_markup_declarations {
            return Ok(());
        }

        let (target, data) = processing_instruction.processing_instruction().unwrap();
        let Some(data) = data else {
            return Ok(());
        };
        let Some(parent) = processing_instruction.parent_node() else {
            return Ok(());
        };
        let data = PI_STANDALONE.replace(data.as_str(), "").to_string().into();
        let new_pi = context.root.as_document().create_processing_instruction(
            target.clone(),
            data,
            &context.info.allocator,
        );
        log::debug!("replacing processing instruction");
        parent.replace_child(new_pi, &processing_instruction);
        Ok(())
    }

    fn element(
        &self,
        element: &Element<'input, 'arena>,
        context: &mut Context<'input, 'arena, '_>,
    ) -> Result<(), Self::Error> {
        if context.flags.contains(ContextFlags::within_foreign_object) {
            return Ok(());
        }

        let name = element.qual_name();
        if !name.prefix().is_empty() {
            return Ok(());
        }

        self.remove_unknown_content(element);
        let inherited = ComputedStyles::default()
            .with_inherited(element, &context.query_has_stylesheet_result)
            .map_err(JobsError::ComputedStylesError)?;
        self.remove_unknown_and_default_attrs(element, &inherited);

        Ok(())
    }
}

impl RemoveUnknownsAndDefaults {
    fn remove_unknown_content(&self, element: &Element) {
        if !self.unknown_content {
            return;
        }

        let name = element.qual_name().unaliased();
        if matches!(name, ElementId::Unknown(_)) {
            log::debug!("removing unknown element type");
            element.remove();
        }

        let Some(parent) = Element::parent_element(element) else {
            return;
        };
        if parent.node_type() == node::Type::Document {
            return;
        }

        let parent_name = parent.qual_name();
        if !parent_name.is_permitted_child(name) {
            log::debug!("removing unknown element of parent");
            element.remove();
        }
    }

    fn remove_unknown_and_default_attrs<'input>(
        &self,
        element: &Element<'input, '_>,
        inherited_styles: &ComputedStyles<'input>,
    ) {
        let element_name = element.qual_name();
        let has_id = has_attribute!(element, Id);

        element.attributes().retain(|attr| {
            let name = attr.name().unaliased();
            let local_name = name.local_name();
            let prefix = attr.prefix();
            let inheritable = matches!(name.r#type(), ContentTypeId::Inheritable(_));
            if is_prefix!(prefix, XML | XLink | XMLNS) || matches!(prefix, Prefix::Unknown { .. }) {
                log::debug!("ignoring prefix: {prefix:?}");
                return true;
            } else if self.keep_data_attrs && local_name.starts_with("data-") {
                log::debug!("keeping data attribute");
                return true;
            } else if local_name.as_str().starts_with("aria-") {
                log::debug!("keeping aria attribute: {}", self.keep_aria_attrs);
                return self.keep_aria_attrs;
            } else if is_attribute!(name, Role) {
                log::debug!("keeping role attribute: {}", self.keep_role_attr);
                return self.keep_role_attr;
            }

            if self.unknown_attrs
                && !is_attribute!(name, XMLNS)
                && !element_name.is_permitted_attribute(name)
            {
                log::debug!("removing unknown attr");
                return false;
            }

            let inherited_value = if name.prefix().is_empty() {
                if inheritable {
                    inherited_styles.get(name.unaliased())
                } else {
                    None
                }
            } else {
                None
            };
            if self.default_attrs
                && !has_id
                && inherited_value.is_none()
                && name.default().is_some_and(|a| a == *attr)
            {
                log::debug!(r#"removing "{name}" attr with default value"#);
                return false;
            }

            if self.useless_overrides
                && !has_id
                && name
                    .attribute_group()
                    .contains(AttributeGroup::Presentation)
                && !name
                    .info()
                    .contains(AttributeInfo::PresentationNonInheritableGroupAttrs)
                && inherited_value.is_some_and(|(inherited, mode)| {
                    if matches!(mode, Mode::Dynamic) {
                        log::debug!("not removing attr with inherited dynamic value");
                        return false;
                    }
                    inherited.value() == attr.value()
                })
            {
                log::debug!("removing useless override");
                return false;
            }
            true
        });
    }
}

const fn default_unknown_content() -> bool {
    true
}
const fn default_unknown_attrs() -> bool {
    true
}
const fn default_default_attrs() -> bool {
    true
}
const fn default_useless_overrides() -> bool {
    true
}
const fn default_default_markup_declarations() -> bool {
    true
}
const fn default_keep_data_attrs() -> bool {
    true
}
const fn default_keep_aria_attrs() -> bool {
    true
}
const fn default_keep_role_attr() -> bool {
    false
}

static PI_STANDALONE: LazyLock<regex::Regex> =
    LazyLock::new(|| regex::Regex::new(r#"\s*standalone\s*=\s*["']no["']"#).unwrap());

#[test]
#[allow(clippy::too_many_lines)]
fn remove_unknowns_and_defaults() -> anyhow::Result<()> {
    use crate::test_config;

    insta::assert_snapshot!(test_config(
        r#"{ "removeUnknownsAndDefaults": {} }"#,
        Some(
            r##"<svg version="1.1" xmlns="http://www.w3.org/2000/svg" xmlns:test="http://" attr="val" x="0" y="10" test:attr="val" xml:space="preserve">
    <!-- preserve xmlns and unknown prefixes -->
    <!-- preserves id'd attributes -->
    <rect fill="#000" d="M0 0"/>
    <rect fill="#000" d="M0 0" id="black-rect"/>
</svg>"##
        ),
    )?);

    insta::assert_snapshot!(test_config(
        r#"{ "removeUnknownsAndDefaults": {} }"#,
        Some(
            r#"<svg xmlns="http://www.w3.org/2000/svg" xmlns:test="http://">
    <!-- unknown elements are removed -->
    <test>
        test
    </test>
    <test:test>
        test
    </test:test>
    <g>
        test
    </g>
</svg>"#
        ),
    )?);

    insta::assert_snapshot!(test_config(
        r#"{ "removeUnknownsAndDefaults": {} }"#,
        Some(
            r##"<svg xmlns="http://www.w3.org/2000/svg">
    <!-- default values are preserved when inheritable -->
    <g fill="red">
        <path fill="#000" d="M118.8 186.9l79.2"/>
    </g>
</svg>"##
        ),
    )?);

    insta::assert_snapshot!(test_config(
        r#"{ "removeUnknownsAndDefaults": {} }"#,
        Some(
            r#"<svg xmlns="http://www.w3.org/2000/svg">
    <!-- remove attributes equal to inherited value -->
    <g fill="black">
        <g fill="red">
            <path fill="red" d="M118.8 186.9l79.2"/>
        </g>
    </g>
</svg>"#
        ),
    )?);

    insta::assert_snapshot!(test_config(
        r#"{ "removeUnknownsAndDefaults": {} }"#,
        Some(
            r#"<svg xmlns="http://www.w3.org/2000/svg">
    <!-- remove attributes equal to inherited value, excluding those with id -->
    <g fill="red">
        <g fill="red">
            <g fill="green">
                <g fill="green">
                    <path fill="red" d="M18.8 86.9l39.2"/>
                </g>
            </g>
            <path fill="red" d="M118.8 186.9l79.2"/>
            <path id="red" fill="red" d="M118.8 186.9l79.2"/>
        </g>
    </g>
</svg>"#
        ),
    )?);

    insta::assert_snapshot!(test_config(
        r#"{ "removeUnknownsAndDefaults": {} }"#,
        Some(
            r##"<svg xmlns="http://www.w3.org/2000/svg">
    <!-- allow data attributes -->
    <g fill="red" data-foo="bar">
        <path fill="#000" d="M118.8 186.9l79.2" data-bind="smth"/>
    </g>
</svg>"##
        ),
    )?);

    insta::assert_snapshot!(test_config(
        r#"{ "removeUnknownsAndDefaults": {} }"#,
        Some(
            r#"<svg xmlns="http://www.w3.org/2000/svg" xmlns:test="http://">
    <!-- skip `foreignObject` and it's children -->
    <foreignObject>
        <div class="test">
            fallback test
        </div>
    </foreignObject>

    <test>
        test
    </test>
    <test:test>
        test
    </test:test>
    <g>
        test
    </g>
</svg>"#
        ),
    )?);

    insta::assert_snapshot!(test_config(
        r#"{ "removeUnknownsAndDefaults": {} }"#,
        Some(
            r#"<svg xmlns="http://www.w3.org/2000/svg" x="0" y="0">
    <!-- remove defaults of non-inheritable values -->
    <svg x="10" y="10">
        <svg x="0" y="0">
            <path/>
        </svg>
        <svg x="0" y="10">
            <path/>
        </svg>
        <svg x="50" y="0">
            <path/>
        </svg>
    </svg>
    <svg x="100" y="100">
        <path/>
    </svg>
</svg>"#
        ),
    )?);

    insta::assert_snapshot!(test_config(
        r#"{ "removeUnknownsAndDefaults": {} }"#,
        Some(
            r#"<svg xmlns="http://www.w3.org/2000/svg">
    <!-- remove unknown elements -->
    <metadata>
        <sfw>
            <slices></slices>
            <sliceSourceBounds height="67.3" width="85.9" y="-40.8" x="-42.5" bottomLeftOrigin="true"></sliceSourceBounds>
        </sfw>
        <ellipse/>
    </metadata>
    <ellipse>
        <font-face/>
    </ellipse>
</svg>"#
        ),
    )?);

    insta::assert_snapshot!(test_config(
        r#"{ "removeUnknownsAndDefaults": {} }"#,
        Some(
            r#"<svg xmlns="http://www.w3.org/2000/svg">
    <!-- retain matching non-inheritable attributes -->
    <g transform="translate(792)">
        <g transform="translate(792)">
            <path d="M118.8 186.9l79.2"/>
        </g>
    </g>
</svg>"#
        ),
    )?);

    insta::assert_snapshot!(test_config(
        r#"{ "removeUnknownsAndDefaults": {} }"#,
        Some(
            r##"<svg xmlns="http://www.w3.org/2000/svg" aria-labelledby="title">
    <!-- retain aria attributes -->
    <title id="title">
        Title
    </title>
    <g aria-label="foo">
        test
    </g>
    <path id="t" d="M10 10h10L10 20"/>
    <use href="#t"/>
</svg>"##
        ),
    )?);

    insta::assert_snapshot!(test_config(
        r#"{ "removeUnknownsAndDefaults": { "keepAriaAttrs": false } }"#,
        Some(
            r#"<svg xmlns="http://www.w3.org/2000/svg" aria-labelledby="title">
    <!-- remove aria attrs -->
    <title id="title">
        Title
    </title>
    <g aria-label="foo">
        test
    </g>
</svg>"#
        ),
    )?);

    insta::assert_snapshot!(test_config(
        r#"{ "removeUnknownsAndDefaults": {} }"#,
        Some(
            r#"<svg xmlns="http://www.w3.org/2000/svg" role="img">
    <!-- remove default role -->
    <g/>
</svg>"#
        ),
    )?);

    insta::assert_snapshot!(test_config(
        r#"{ "removeUnknownsAndDefaults": { "keepRoleAttr": true } }"#,
        Some(
            r#"<svg xmlns="http://www.w3.org/2000/svg" role="img">
    <!-- retain default role -->
    <g/>
</svg>"#
        ),
    )?);

    insta::assert_snapshot!(test_config(
        r#"{ "removeUnknownsAndDefaults": {} }"#,
        Some(
            r##"<svg width="480" height="360" xmlns="http://www.w3.org/2000/svg" xmlns:xlink="http://www.w3.org/1999/xlink">
    <!-- handle xlink and xmlns -->
    <text x="50" y="50">
        A <a xlink:href="#"><tspan>link around tspan</tspan></a> for testing
    </text>
</svg>"##
        ),
    )?);

    insta::assert_snapshot!(test_config(
        r#"{ "removeUnknownsAndDefaults": {} }"#,
        Some(
            r#"<svg width="64" height="18" xmlns="http://www.w3.org/2000/svg">
    <!-- removes `standalone="no" from xml declaration -->
    <text x="4" y="18">uwu</text>
</svg>"#
        ),
    )?);

    insta::assert_snapshot!(test_config(
        r#"{ "removeUnknownsAndDefaults": {} }"#,
        Some(
            r##"<svg width="50" height="50" xmlns="http://www.w3.org/2000/svg">
    <!-- do not remove default when inherited value differs -->
    <g fill="#fff">
      <g>
        <rect x="0" y="0" width="50" height="50" fill="#000" />
      </g>
    </g>
</svg>"##
        ),
    )?);

    Ok(())
}
