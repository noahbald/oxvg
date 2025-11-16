//! Data that can be assigned to an element node.
//!
//! Essentially an embedding of the [SVG 1.1](https://www.w3.org/TR/2011/REC-SVG11-20110816/eltindex.html) and [SVG 2](https://svgwg.org/svg2-draft/eltindex.html) spec.

use crate::{
    atom::Atom,
    attribute::{AttrId, AttributeGroup},
    name::{Prefix, QualName},
};
use std::fmt::Display;

pub use category::{ElementCategory, ElementInfo};

mod category;

/// An element's category.
type C = ElementCategory;
/// Permitted category of a child.
type PC = ElementCategory;
/// Permitted child.
type PE = &'static [ElementId<'static>];
/// Expected attributes.
type EA = &'static [AttrId<'static>];
/// Expected attribute groups.
#[allow(clippy::upper_case_acronyms)]
type EAG = AttributeGroup;

macro_rules! define_elements {
    ($($element:ident {
        name: $name:tt,
        categories: $categories:expr,
        permitted_categories: $permitted_categories:expr,
        permitted_elements: $permitted_elements:expr,
        expected_attribute_groups: $expected_attribute_groups:expr,
        expected_attributes: $expected_attributes:expr,
        $(info: $info:expr,)?
    },)+) => {
        #[allow(non_upper_case_globals)]
        mod _c {
            use super::{C, ElementCategory};
            $(pub const $element: C = $categories;)+
        }
        #[allow(non_upper_case_globals)]
        mod _pc {
            use super::{PC, ElementCategory};
            $(pub const $element: PC = $permitted_categories;)+
        }
        #[allow(non_upper_case_globals)]
        mod _pe {
            use super::{PE, ElementId};
            $(pub const $element: PE = $permitted_elements;)+
        }
        #[allow(non_upper_case_globals)]
        mod _eag {
            use super::{EAG, AttributeGroup};
            $(pub const $element: EAG = $expected_attribute_groups;)+
        }
        #[allow(non_upper_case_globals)]
        mod _ea {
            use super::{EA, AttrId};
            $(pub const $element: EA = $expected_attributes;)+
        }
        #[allow(non_upper_case_globals)]
        #[cfg(not(feature = "markup5ever"))]
        mod _qual_name {
            use crate::name::{Prefix, QualName};
            use crate::atom::Atom;
            $(pub const $element: &'static QualName<'static> = &QualName {
                prefix: Prefix::SVG,
                local: Atom::Static($name),
            };)+
        }
        #[allow(non_upper_case_globals)]
        #[cfg(feature = "markup5ever")]
        mod _qual_name {
            use crate::name::{Prefix, QualName};
            use crate::atom::Atom;
            $(pub const $element: &'static QualName<'static> = &QualName {
                prefix: Prefix::SVG,
                local: Atom::Local(xml5ever::local_name!($name)),
            };)+
        }
        #[allow(non_upper_case_globals)]
        mod _local_name {
            use crate::atom::Atom;
            use super::_qual_name;
            $(pub const $element: &'static Atom<'static> = &_qual_name::$element.local;)+
        }

        #[derive(Clone, Debug, Hash, Eq)]
        /// Identifies an element by it's local-name and namespace
        ///
        /// [MDN | SVG element reference](https://developer.mozilla.org/en-US/docs/Web/SVG/Reference/Element)
        pub enum ElementId<'input> {
            $(
                #[doc=concat!("The `", $name, "` element")]
                $element,
            )+
            /// A known element aliased by a different prefix
            Aliased {
                /// The prefix assigned to the element
                prefix: Prefix<'input>,
                /// The associated element
                element_id: Box<ElementId<'input>>,
            },
            /// An element that isn't a well-known `ElementId`
            Unknown(QualName<'input>),
        }

        impl<'input> ElementId<'input> {
            /// Creates a qualified name from a prefix and local part
            pub fn new(prefix: Prefix<'input>, local: Atom<'input>) -> Self {
                match (prefix, &*local) {
                    $((Prefix::SVG, $name) => Self::$element,)+
                    (prefix, _) => Self::Unknown(QualName { prefix, local }),
                }
            }

            /// Returns the prefix of the qualified name.
            pub fn prefix(&self) -> &Prefix<'input> {
                match self {
                    Self::Unknown(name) => &name.prefix,
                    Self::Aliased { prefix, .. } => prefix,
                    _ => &Prefix::SVG,
                }
            }

            // For use in macros interoperable with `Attr`
            #[doc(hidden)]
            pub fn qual_name<'a>(&'a self) -> &'a ElementId<'input> {
                self
            }

            /// Returns the local part of the qualified name.
            pub fn local_name(&self) -> &Atom<'input> {
                match self {
                    $(Self::$element => _local_name::$element,)+
                    Self::Aliased { element_id, .. } => element_id.local_name(),
                    Self::Unknown(name) => &name.local,
                }
            }

            /// Returns the element's category
            pub fn categories(&self) -> C {
                match self {
                    $(Self::$element => _c::$element,)+
                    Self::Aliased { element_id, .. } => element_id.categories(),
                    Self::Unknown(_) => ElementCategory::empty(),
                }
            }

            /// Returns element categories allowed as children
            pub fn permitted_categories(&self) -> PC {
                match self {
                    $(Self::$element => _pc::$element,)+
                    Self::Aliased { element_id, .. } => element_id.permitted_categories(),
                    Self::Unknown(_) => ElementCategory::all(),
                }
            }

            /// Returns specific elements allowed as children
            pub fn permitted_elements(&self) -> PE {
                match self {
                    $(Self::$element => _pe::$element,)+
                    Self::Aliased { element_id, .. } => element_id.permitted_elements(),
                    Self::Unknown(_) => &[],
                }
            }

            /// Whether the child is allowed within the SVG element.
            pub fn is_permitted_child(&self, child: &Self) -> bool {
                let permitted_categories = self.permitted_categories();
                if permitted_categories.contains(ElementCategory::all()) {
                    return true;
                }

                let child_categories = child.categories();
                if child_categories.contains(ElementCategory::all()) {
                    return true;
                }
                permitted_categories.intersects(child_categories)
                    || self.permitted_elements().contains(child)
            }

            /// Whether the attribute is allow on the SVG element.
            pub fn is_permitted_attribute(&self, attribute: &AttrId<'_>) -> bool {
                let permitted_attributes = self.expected_attribute_groups();
                if permitted_attributes.contains(AttributeGroup::all()) {
                    return true;
                }

                let attr_groups = attribute.attribute_group();
                permitted_attributes.intersects(attr_groups)
                    || self.expected_attributes().contains(attribute)
            }

            /// Returns specific attributes allowed for this element.
            pub fn expected_attributes(&self) -> EA {
                match self {
                    $(Self::$element => _ea::$element,)+
                    Self::Aliased { element_id, .. } => element_id.expected_attributes(),
                    Self::Unknown(_) => &[],
                }
            }

            /// Returns attribute groups allowed for this element.
            pub fn expected_attribute_groups(&self) -> EAG {
                (match self {
                    // Presentation attribute can apply to any `SVG` namespaced element
                    // https://svgwg.org/svg2-draft/styling.html#TermPresentationAttribute
                    $(Self::$element => _eag::$element.union(AttributeGroup::Presentation),)+
                    Self::Aliased { element_id, .. } => element_id.expected_attribute_groups(),
                    Self::Unknown(_) => AttributeGroup::XLink,
                // Core attributes can apply to any `SVG` element
                // https://svgwg.org/svg2-draft/struct.html#TermCoreAttribute
                }).union(AttributeGroup::Core)
            }

            /// Returns info flags about this element.
            pub fn info(&self) -> ElementInfo {
                match self {
                    $($(Self::$element => $info,)?)+
                    _ => ElementInfo::empty(),
                }
            }

            /// Returns the length of joining the prefix and local part of a name with a `:`
            pub fn len(&self) -> usize {
                match self.prefix().value() {
                    Some(p) => p.len() + 1 + self.local_name().len(),
                    None => self.local_name().len(),
                }
            }

            /// Returns whether the name is equivalent to an empty string
            pub fn is_empty(&self) -> bool {
                self.prefix().is_empty() && self.local_name().is_empty()
            }
        }

        impl PartialEq for ElementId<'_> {
            fn eq(&self, other: &Self) -> bool {
                match (self.unaliased(), other.unaliased()) {
                    $((Self::$element, Self::$element) => true,)+
                    (Self::Aliased { .. }, _) => unreachable!(),
                    (_, Self::Aliased { .. }) => unreachable!(),
                    (Self::Unknown(a), Self::Unknown(b)) => a == b,
                    _ => false,

                }
            }
        }
    };
}
impl PartialOrd for ElementId<'_> {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}
impl Ord for ElementId<'_> {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.prefix()
            .cmp(other.prefix())
            .then_with(|| self.local_name().cmp(other.local_name()))
    }
}

impl<'input> ElementId<'input> {
    /// Returns an `AttrId` that matches the elements expected attributes/attribute groups.
    /// Returns an unknown `AttrId` otherwise
    pub fn parse_attr_id(&self, prefix: &Prefix<'input>, local: Atom<'input>) -> AttrId<'input> {
        // NOTE: `expected_attributes` must be looked at first, since they are always more
        // specialised than those from `expected_attribute_groups`.
        self.expected_attributes()
            .iter()
            .find(|attr| attr.prefix() == prefix && *attr.local_name() == local)
            .map_or_else(
                || {
                    self.expected_attribute_groups()
                        .parse_attr_id(prefix, local)
                },
                |attr| {
                    if prefix.is_aliased() {
                        AttrId::Aliased {
                            prefix: prefix.clone(),
                            attr_id: Box::new(attr.clone()),
                        }
                    } else {
                        attr.clone()
                    }
                },
            )
    }

    /// Returns an `ElementId` that may be prefixed as `ElementId::Aliased` as the inner id
    /// that's aliased.
    pub fn unaliased(&self) -> &Self {
        match self {
            Self::Aliased { element_id, .. } => {
                let result = element_id.as_ref();
                debug_assert!(!matches!(result, Self::Aliased { .. }));
                result
            }
            _ => self,
        }
    }
}

impl Display for ElementId<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let local_name: &str = self.local_name();
        match self.prefix().value() {
            None => Display::fmt(local_name, f),
            Some(p) => f.write_fmt(format_args!("{p}:{local_name}")),
        }
    }
}

// NOTE: Elements are ordered according to the spec's element index
// NOTE: SVG 1.1 and SVG 2 spec for each element is merged. So, if it's
//       in only one-of SVG 1.1 or SVG 2 it will still be listed here.
// NOTE: Core attributes are not included in the definition here, by merit
//       of them being common to all elements.
//
// [w3 | SVG 1.1](https://www.w3.org/TR/2011/REC-SVG11-20110816/attindex.html)
// [w3 | SVG 2](https://svgwg.org/svg2-draft/attindex.html)
define_elements! {
    A {
        name: "a",
        categories: ElementCategory::Container.union(ElementCategory::Renderable),
        permitted_categories: ElementCategory::Animation
            .union(ElementCategory::Descriptive)
            .union(ElementCategory::Shape)
            .union(ElementCategory::Structural)
            .union(ElementCategory::Gradient),
        permitted_elements: &[
            ElementId::A,
            ElementId::AltGlyphDef,
            ElementId::ClipPath,
            ElementId::ColorProfile,
            ElementId::Cursor,
            ElementId::Filter,
            ElementId::Font,
            ElementId::FontFace,
            ElementId::ForeignObject,
            ElementId::Image,
            ElementId::Marker,
            ElementId::Mask,
            ElementId::Pattern,
            ElementId::Script,
            ElementId::Style,
            ElementId::Switch,
            ElementId::Text,
            ElementId::View,
        ],
        expected_attribute_groups: AttributeGroup::Aria
            .union(AttributeGroup::ConditionalProcessing)
            .union(AttributeGroup::DocumentElementEvent)
            .union(AttributeGroup::GlobalEvent)
            .union(AttributeGroup::GraphicalEvent)
            .union(AttributeGroup::Presentation)
            .union(AttributeGroup::XLink),
        expected_attributes: &[
            AttrId::Class,
            AttrId::ExternalResourcesRequired,
            AttrId::Download,
            AttrId::Href,
            AttrId::Hreflang,
            AttrId::Ping,
            AttrId::ReferrerPolicy,
            AttrId::Rel,
            AttrId::Target,
            AttrId::TypeA,
        ],
    },
    AltGlyph {
        name: "altGlyph",
        categories: ElementCategory::TextContent
            .union(ElementCategory::TextContentChild),
        permitted_categories: ElementCategory::all(),
        permitted_elements: &[],
        expected_attribute_groups: AttributeGroup::ConditionalProcessing
            .union(AttributeGroup::GraphicalEvent)
            .union(AttributeGroup::Presentation)
            .union(AttributeGroup::XLink),
        expected_attributes: &[
            AttrId::ExternalResourcesRequired,
            AttrId::XAltGlyph,
            AttrId::YAltGlyph,
            AttrId::DXAltGlyph,
            AttrId::DYAltGlyph,
            AttrId::GlyphRef,
            AttrId::Format,
            AttrId::RotateText,
        ],
    },
    AltGlyphDef {
        name: "altGlyphDef",
        categories: ElementCategory::Uncategorised,
        permitted_categories: ElementCategory::empty(),
        permitted_elements: &[ElementId::GlyphRef, ElementId::AltGlyphItem],
        expected_attribute_groups: AttributeGroup::empty(),
        expected_attributes: &[],
    },
    AltGlyphItem {
        name: "altGlyphItem",
        categories: ElementCategory::Uncategorised,
        permitted_categories: ElementCategory::empty(),
        permitted_elements: &[ElementId::GlyphRef],
        expected_attribute_groups: AttributeGroup::empty(),
        expected_attributes: &[],
    },
    Animate {
        name: "animate",
        categories: ElementCategory::Animation,
        permitted_categories: ElementCategory::Descriptive,
        permitted_elements: &[ElementId::Script],
        expected_attribute_groups: AttributeGroup::AnimationAddition
            .union(AttributeGroup::AnimationEvent)
            .union(AttributeGroup::AnimationTargetElement)
            .union(AttributeGroup::AnimationAttributeTarget)
            .union(AttributeGroup::AnimationTiming)
            .union(AttributeGroup::AnimationValue)
            .union(AttributeGroup::ConditionalProcessing)
            .union(AttributeGroup::GlobalEvent)
            .union(AttributeGroup::Presentation)
            .union(AttributeGroup::DocumentElementEvent)
            .union(AttributeGroup::XLink),
        expected_attributes: &[AttrId::ExternalResourcesRequired],
    },
    AnimateColor {
        name: "animateColor",
        categories: ElementCategory::Animation,
        permitted_categories: ElementCategory::Descriptive,
        permitted_elements: &[],
        expected_attribute_groups: AttributeGroup::ConditionalProcessing
            .union(AttributeGroup::AnimationEvent)
            .union(AttributeGroup::XLink)
            .union(AttributeGroup::AnimationAttributeTarget)
            .union(AttributeGroup::AnimationTiming)
            .union(AttributeGroup::AnimationValue)
            .union(AttributeGroup::AnimationAddition)
            .union(AttributeGroup::Presentation),
        expected_attributes: &[AttrId::ExternalResourcesRequired],
    },
    AnimateMotion {
        name: "animateMotion",
        categories: ElementCategory::Animation,
        permitted_categories: ElementCategory::Descriptive,
        permitted_elements: &[ElementId::MPath, ElementId::Script],
        expected_attribute_groups: AttributeGroup::AnimationAddition
            .union(AttributeGroup::AnimationEvent)
            .union(AttributeGroup::AnimationTargetElement)
            .union(AttributeGroup::AnimationTiming)
            .union(AttributeGroup::AnimationValue)
            .union(AttributeGroup::ConditionalProcessing)
            .union(AttributeGroup::GlobalEvent)
            .union(AttributeGroup::DocumentElementEvent)
            .union(AttributeGroup::XLink),
        expected_attributes: &[
            AttrId::ExternalResourcesRequired,
            AttrId::Path,
            AttrId::KeyPoints,
            AttrId::RotateAnimate,
            AttrId::Origin
        ],
    },
    AnimateTransform {
        name: "animateTransform",
        categories: ElementCategory::Animation,
        permitted_categories: ElementCategory::Descriptive,
        permitted_elements: &[ElementId::Script],
        expected_attribute_groups:
            AttributeGroup::AnimationAddition
                .union(AttributeGroup::AnimationEvent)
                .union(AttributeGroup::AnimationTargetElement)
                .union(AttributeGroup::AnimationAttributeTarget)
                .union(AttributeGroup::AnimationTiming)
                .union(AttributeGroup::AnimationValue)
                .union(AttributeGroup::ConditionalProcessing)
                .union(AttributeGroup::GlobalEvent)
                .union(AttributeGroup::DocumentElementEvent)
                .union(AttributeGroup::XLink),
        expected_attributes: &[AttrId::TypeAnimateTransform],
    },
    Circle {
        name: "circle",
        categories: ElementCategory::Shape
            .union(ElementCategory::BasicShape)
            .union(ElementCategory::Graphics)
            .union(ElementCategory::Renderable),
        permitted_categories: ElementCategory::Animation
            .union(ElementCategory::Descriptive)
            .union(ElementCategory::PaintServer),
        permitted_elements: &[
            ElementId::ClipPath,
            ElementId::Marker,
            ElementId::Mask,
            ElementId::Script,
            ElementId::Style
        ],
        expected_attribute_groups:
            AttributeGroup::Aria
                .union(AttributeGroup::ConditionalProcessing)
                .union(AttributeGroup::GlobalEvent)
                .union(AttributeGroup::DocumentElementEvent)
                .union(AttributeGroup::GraphicalEvent)
                .union(AttributeGroup::Presentation),
        expected_attributes: &[
            AttrId::ExternalResourcesRequired,
            AttrId::PathLength,
            AttrId::CXGeometry,
            AttrId::CYGeometry,
            AttrId::RGeometry,
        ],
    },
    ClipPath {
        name: "clipPath",
        categories: ElementCategory::Uncategorised,
        permitted_categories: ElementCategory::Animation
            .union(ElementCategory::Descriptive)
            .union(ElementCategory::Shape),
        permitted_elements: &[ElementId::Text, ElementId::Use, ElementId::Script],
        expected_attribute_groups:
            AttributeGroup::ConditionalProcessing
                .union(AttributeGroup::Presentation),
        expected_attributes: &[
            AttrId::ExternalResourcesRequired,
            AttrId::ClipPathUnits,
        ],
        info: ElementInfo::NonRendering,
    },
    ColorProfile {
        name: "color-profile",
        categories: ElementCategory::Uncategorised,
        permitted_categories: ElementCategory::Descriptive,
        permitted_elements: &[],
        expected_attribute_groups: AttributeGroup::XLink,
        expected_attributes: &[
            AttrId::Local,
            AttrId::NameColorProfile,
            AttrId::RenderingIntent,
        ],
        info: ElementInfo::Legacy,
    },
    Cursor {
        name: "cursor",
        categories: ElementCategory::Uncategorised,
        permitted_categories: ElementCategory::Descriptive,
        permitted_elements: &[],
        expected_attribute_groups: AttributeGroup::ConditionalProcessing
            .union(AttributeGroup::XLink),
        expected_attributes: &[
            AttrId::ExternalResourcesRequired,
            AttrId::XCursor,
            AttrId::YCursor,
        ],
        info: ElementInfo::Legacy,
    },
    Defs {
        name: "defs",
        categories: ElementCategory::Container
            .union(ElementCategory::NeverRendered)
            .union(ElementCategory::Structural),
        permitted_categories: ElementCategory::Animation
            .union(ElementCategory::Descriptive)
            .union(ElementCategory::PaintServer)
            .union(ElementCategory::Shape)
            .union(ElementCategory::Structural)
            .union(ElementCategory::Gradient),
        permitted_elements: &[
            ElementId::A,
            ElementId::AltGlyphDef,
            ElementId::ClipPath,
            ElementId::ColorProfile,
            ElementId::Cursor,
            ElementId::Filter,
            ElementId::Font,
            ElementId::FontFace,
            ElementId::ForeignObject,
            ElementId::Image,
            ElementId::Marker,
            ElementId::Mask,
            ElementId::Script,
            ElementId::Style,
            ElementId::Switch,
            ElementId::Text,
            ElementId::View,
        ],
        expected_attribute_groups: AttributeGroup::GlobalEvent
            .union(AttributeGroup::DocumentElementEvent)
            .union(AttributeGroup::GraphicalEvent)
            .union(AttributeGroup::Presentation)
            .union(AttributeGroup::ConditionalProcessing),
        expected_attributes: &[
            AttrId::ExternalResourcesRequired,
        ],
    },
    Desc {
        name: "desc",
        categories: ElementCategory::Descriptive.union(ElementCategory::NeverRendered),
        permitted_categories: ElementCategory::all(),
        permitted_elements: &[],
        expected_attribute_groups: AttributeGroup::GlobalEvent
            .union(AttributeGroup::DocumentElementEvent),
        expected_attributes: &[],
    },
    Ellipse {
        name: "ellipse",
        categories: ElementCategory::BasicShape
            .union(ElementCategory::Renderable)
            .union(ElementCategory::Graphics)
            .union(ElementCategory::Shape),
        permitted_categories: ElementCategory::Animation
            .union(ElementCategory::Descriptive)
            .union(ElementCategory::PaintServer),
        permitted_elements: &[
            ElementId::ClipPath,
            ElementId::Marker,
            ElementId::Mask,
            ElementId::Script,
            ElementId::Style,
        ],
        expected_attribute_groups: AttributeGroup::Aria
            .union(AttributeGroup::ConditionalProcessing)
            .union(AttributeGroup::GlobalEvent)
            .union(AttributeGroup::DocumentElementEvent)
            .union(AttributeGroup::GraphicalEvent)
            .union(AttributeGroup::Presentation),
        expected_attributes: &[
            AttrId::ExternalResourcesRequired,
            AttrId::PathLength,
            AttrId::CXGeometry,
            AttrId::CYGeometry,
            AttrId::RX,
            AttrId::RY,
        ],
    },
    FeBlend {
        name: "feBlend",
        categories: ElementCategory::FilterPrimitive,
        permitted_categories: ElementCategory::Descriptive,
        permitted_elements: &[ElementId::Animate, ElementId::Script, ElementId::Set],
        expected_attribute_groups: AttributeGroup::Presentation
            .union(AttributeGroup::FilterPrimitive),
        expected_attributes: &[
            AttrId::In,
            AttrId::In2,
            AttrId::Mode,
        ],
    },
    FeColorMatrix {
        name: "feColorMatrix",
        categories: ElementCategory::FilterPrimitive,
        permitted_categories: ElementCategory::Descriptive,
        permitted_elements: &[ElementId::Animate, ElementId::Script, ElementId::Set],
        expected_attribute_groups: AttributeGroup::Presentation
            .union(AttributeGroup::FilterPrimitive),
        expected_attributes: &[
            AttrId::In,
            AttrId::TypeFeColorMatrix,
            AttrId::ValuesFeColorMatrix,
        ],
    },
    FeComponentTransfer {
        name: "feComponentTransfer",
        categories: ElementCategory::FilterPrimitive,
        permitted_categories: ElementCategory::Descriptive,
        permitted_elements: &[
            ElementId::FeFuncB,
            ElementId::FeFuncR,
            ElementId::FeFuncG,
            ElementId::FeFuncA,
            ElementId::Script,
        ],
        expected_attribute_groups: AttributeGroup::Presentation
            .union(AttributeGroup::FilterPrimitive),
        expected_attributes: &[AttrId::In],
    },
    FeComposite {
        name: "feComposite",
        categories: ElementCategory::FilterPrimitive,
        permitted_categories: ElementCategory::Descriptive,
        permitted_elements: &[ElementId::Animate, ElementId::Script, ElementId::Set],
        expected_attribute_groups: AttributeGroup::Presentation
            .union(AttributeGroup::FilterPrimitive),
        expected_attributes: &[
            AttrId::In,
            AttrId::In2,
            AttrId::OperatorFeComposite,
            AttrId::K1,
            AttrId::K2,
            AttrId::K3,
            AttrId::K4,
        ],
    },
    FeConvolveMatrix {
        name: "feConvolveMatrix",
        categories: ElementCategory::FilterPrimitive,
        permitted_categories: ElementCategory::Descriptive,
        permitted_elements: &[ElementId::Animate, ElementId::Script, ElementId::Set],
        expected_attribute_groups: AttributeGroup::Presentation
            .union(AttributeGroup::FilterPrimitive),
        expected_attributes: &[
            AttrId::In,
            AttrId::Order,
            AttrId::KernalMatrix,
            AttrId::Divisor,
            AttrId::Bias,
            AttrId::TargetX,
            AttrId::TargetY,
            AttrId::EdgeModeFeConvolveMatrix,
            AttrId::KernelUnitLength,
            AttrId::PreserveAlpha,
        ],
    },
    FeDiffuseLighting {
        name: "feDiffuseLighting",
        categories: ElementCategory::FilterPrimitive,
        permitted_categories: ElementCategory::Descriptive
            .union(ElementCategory::LightSource),
        permitted_elements: &[ElementId::Script],
        expected_attribute_groups: AttributeGroup::Presentation
            .union(AttributeGroup::FilterPrimitive),
        expected_attributes: &[
            AttrId::In,
            AttrId::SurfaceScale,
            AttrId::DiffuseConstant,
            AttrId::KernelUnitLength,
        ],
    },
    FeDisplacementMap {
        name: "feDisplacementMap",
        categories: ElementCategory::FilterPrimitive,
        permitted_categories: ElementCategory::Descriptive,
        permitted_elements: &[ElementId::Animate, ElementId::Script, ElementId::Set],
        expected_attribute_groups: AttributeGroup::Presentation
            .union(AttributeGroup::FilterPrimitive),
        expected_attributes: &[
            AttrId::In,
            AttrId::In2,
            AttrId::Scale,
            AttrId::XChannelSelector,
            AttrId::YChannelSelector,
        ],
    },
    FeDistantLight {
        name: "feDistantLight",
        categories: ElementCategory::LightSource,
        permitted_categories: ElementCategory::Descriptive,
        permitted_elements: &[ElementId::Animate, ElementId::Script, ElementId::Set],
        expected_attribute_groups: AttributeGroup::empty(),
        expected_attributes: &[AttrId::Azimuth, AttrId::Elevation],
    },
    FeDropShadow {
        name: "feDropShadow",
        categories: ElementCategory::FilterPrimitive,
        permitted_categories: ElementCategory::Descriptive,
        permitted_elements: &[ElementId::Animate, ElementId::Script, ElementId::Set],
        expected_attribute_groups: AttributeGroup::Presentation
            .union(AttributeGroup::FilterPrimitive),
        expected_attributes: &[
            AttrId::In,
            AttrId::StdDeviationFeDropShadow,
            AttrId::DXFeDropShadow,
            AttrId::DYFeDropShadow,
        ],
    },
    FeFlood {
        name: "feFlood",
        categories: ElementCategory::FilterPrimitive,
        permitted_categories: ElementCategory::Descriptive,
        permitted_elements: &[
            ElementId::Animate,
            ElementId::AnimateColor,
            ElementId::Script,
            ElementId::Set
        ],
        expected_attribute_groups: AttributeGroup::Presentation
            .union(AttributeGroup::FilterPrimitive),
        expected_attributes: &[],
    },
    FeFuncA {
        name: "feFuncA",
        categories: ElementCategory::TransferFunction,
        permitted_categories: ElementCategory::Descriptive,
        permitted_elements: &[ElementId::Animate, ElementId::Script, ElementId::Set],
        expected_attribute_groups: AttributeGroup::TransferFunction,
        expected_attributes: &[],
    },
    FeFuncB {
        name: "feFuncB",
        categories: ElementCategory::TransferFunction,
        permitted_categories: ElementCategory::Descriptive,
        permitted_elements: &[ElementId::Animate, ElementId::Script, ElementId::Set],
        expected_attribute_groups: AttributeGroup::TransferFunction,
        expected_attributes: &[],
    },
    FeFuncG {
        name: "feFuncG",
        categories: ElementCategory::TransferFunction,
        permitted_categories: ElementCategory::Descriptive,
        permitted_elements: &[ElementId::Animate, ElementId::Script, ElementId::Set],
        expected_attribute_groups: AttributeGroup::TransferFunction,
        expected_attributes: &[],
    },
    FeFuncR {
        name: "feFuncR",
        categories: ElementCategory::TransferFunction,
        permitted_categories: ElementCategory::Descriptive,
        permitted_elements: &[ElementId::Animate, ElementId::Script, ElementId::Set],
        expected_attribute_groups: AttributeGroup::TransferFunction,
        expected_attributes: &[],
    },
    FeGaussianBlur {
        name: "feGaussianBlur",
        categories: ElementCategory::FilterPrimitive,
        permitted_categories: ElementCategory::Descriptive,
        permitted_elements: &[ElementId::Animate, ElementId::Script, ElementId::Set],
        expected_attribute_groups: AttributeGroup::Presentation
            .union(AttributeGroup::FilterPrimitive),
        expected_attributes: &[
            AttrId::In,
            AttrId::StdDeviationFeGaussianBlur,
            AttrId::EdgeModeFeGaussianBlur,
        ],
    },
    FeImage {
        name: "feImage",
        categories: ElementCategory::FilterPrimitive,
        permitted_categories: ElementCategory::Descriptive,
        permitted_elements: &[
            ElementId::Animate,
            ElementId::AnimateTransform,
            ElementId::Script,
            ElementId::Set,
        ],
        expected_attribute_groups: AttributeGroup::Presentation
            .union(AttributeGroup::FilterPrimitive)
            .union(AttributeGroup::XLink),
        expected_attributes: &[
            AttrId::ExternalResourcesRequired,
            AttrId::PreserveAspectRatio,
            AttrId::Href,
            AttrId::CrossOrigin,
        ],
    },
    FeMerge {
        name: "feMerge",
        categories: ElementCategory::FilterPrimitive,
        permitted_categories: ElementCategory::Descriptive,
        permitted_elements: &[ElementId::FeMergeNode, ElementId::Script],
        expected_attribute_groups: AttributeGroup::Presentation
            .union(AttributeGroup::FilterPrimitive),
        expected_attributes: &[],
    },
    FeMergeNode {
        name: "feMergeNode",
        categories: ElementCategory::Uncategorised,
        permitted_categories: ElementCategory::Descriptive,
        permitted_elements: &[ElementId::Animate, ElementId::Script, ElementId::Set],
        expected_attribute_groups: AttributeGroup::Presentation
            .union(AttributeGroup::FilterPrimitive),
        expected_attributes: &[AttrId::In],
    },
    FeMorphology {
        name: "feMorphology",
        categories: ElementCategory::FilterPrimitive,
        permitted_categories: ElementCategory::Descriptive,
        permitted_elements: &[ElementId::Animate, ElementId::Script, ElementId::Set],
        expected_attribute_groups: AttributeGroup::Presentation
            .union(AttributeGroup::FilterPrimitive),
        expected_attributes: &[
            AttrId::In,
            AttrId::OperatorFeMorphology,
            AttrId::Radius,
        ],
    },
    FeOffset {
        name: "feOffset",
        categories: ElementCategory::FilterPrimitive,
        permitted_categories: ElementCategory::Descriptive,
        permitted_elements: &[ElementId::Animate, ElementId::Script, ElementId::Set],
        expected_attribute_groups: AttributeGroup::Presentation
            .union(AttributeGroup::FilterPrimitive),
        expected_attributes: &[
            AttrId::In,
            AttrId::DXFeOffset,
            AttrId::DYFeOffset,
        ],
    },
    FePointLight {
        name: "fePointLight",
        categories: ElementCategory::LightSource,
        permitted_categories: ElementCategory::Descriptive,
        permitted_elements: &[ElementId::Animate, ElementId::Script, ElementId::Set],
        expected_attribute_groups: AttributeGroup::empty(),
        expected_attributes: &[
            AttrId::XFe,
            AttrId::YFe,
            AttrId::ZFe,
        ],
    },
    FeSpecularLighting {
        name: "feSpecularLighting",
        categories: ElementCategory::FilterPrimitive,
        permitted_categories: ElementCategory::LightSource
            .union(ElementCategory::Descriptive),
        permitted_elements: &[ElementId::Script],
        expected_attribute_groups: AttributeGroup::Presentation
            .union(AttributeGroup::FilterPrimitive),
        expected_attributes: &[
            AttrId::In,
            AttrId::SurfaceScale,
            AttrId::SpecularConstant,
            AttrId::SpecularExponent,
            AttrId::KernelUnitLength,
        ],
    },
    FeSpotLight {
        name: "feSpotLight",
        categories: ElementCategory::LightSource,
        permitted_categories: ElementCategory::Descriptive,
        permitted_elements: &[ElementId::Animate, ElementId::Script, ElementId::Set],
        expected_attribute_groups: AttributeGroup::empty(),
        expected_attributes: &[
            AttrId::XFe,
            AttrId::YFe,
            AttrId::ZFe,
            AttrId::PointsAtX,
            AttrId::PointsAtY,
            AttrId::PointsAtZ,
            AttrId::SpecularExponent,
            AttrId::LimitingConeAngle,
        ],
    },
    FeTile {
        name: "feTile",
        categories: ElementCategory::FilterPrimitive,
        permitted_categories: ElementCategory::Descriptive,
        permitted_elements: &[ElementId::Animate, ElementId::Script, ElementId::Set],
        expected_attribute_groups: AttributeGroup::Presentation
            .union(AttributeGroup::FilterPrimitive),
        expected_attributes: &[AttrId::In],
    },
    FeTurbulence {
        name: "feTurbulence",
        categories: ElementCategory::FilterPrimitive,
        permitted_categories: ElementCategory::Descriptive,
        permitted_elements: &[ElementId::Animate, ElementId::Script, ElementId::Set],
        expected_attribute_groups: AttributeGroup::Presentation
            .union(AttributeGroup::FilterPrimitive),
        expected_attributes: &[
            AttrId::BaseFrequency,
            AttrId::NumOctaves,
            AttrId::Seed,
            AttrId::StitchTiles,
            AttrId::TypeFeTurbulence,
        ],
    },
    Filter {
        name: "filter",
        categories: ElementCategory::Uncategorised,
        permitted_categories: ElementCategory::Descriptive
            .union(ElementCategory::FilterPrimitive),
        permitted_elements: &[ElementId::Animate, ElementId::Script, ElementId::Set],
        expected_attribute_groups: AttributeGroup::empty()
            .union(AttributeGroup::Presentation)
            .union(AttributeGroup::XLink),
        expected_attributes: &[
            AttrId::ExternalResourcesRequired,
            AttrId::XFilter,
            AttrId::YFilter,
            AttrId::WidthFilter,
            AttrId::HeightFilter,
            AttrId::FilterRes,
            AttrId::FilterUnits,
            AttrId::PrimitiveUnits,
        ],
        info: ElementInfo::NonRendering.union(ElementInfo::Legacy),
    },
    Font {
        name: "font",
        categories: ElementCategory::Uncategorised,
        permitted_categories: ElementCategory::Descriptive,
        permitted_elements: &[
            ElementId::FontFace,
            ElementId::Glyph,
            ElementId::HKern,
            ElementId::MissingGlyph,
            ElementId::VKern,
        ],
        expected_attribute_groups: AttributeGroup::Presentation,
        expected_attributes: &[
            AttrId::ExternalResourcesRequired,
            AttrId::HorizOriginX,
            AttrId::HorizOriginY,
            AttrId::HorizAdvX,
            AttrId::VertOriginX,
            AttrId::VertOriginY,
            AttrId::VertAdvY,
        ],
        info: ElementInfo::Legacy,
    },
    FontFace {
        name: "font-face",
        categories: ElementCategory::Uncategorised,
        permitted_categories: ElementCategory::Descriptive,
        permitted_elements: &[ElementId::FontFaceSrc],
        expected_attribute_groups: AttributeGroup::empty(),
        expected_attributes: &[
            AttrId::FontFamily,
            AttrId::FontStyle,
            AttrId::FontVariant,
            AttrId::FontWeight,
            AttrId::FontStretch,
            AttrId::FontSize,
            AttrId::UnicodeRange,
            AttrId::UnitsPerEm,
            AttrId::Panose1,
            AttrId::Stemv,
            AttrId::Stemh,
            AttrId::SlopeFont,
            AttrId::CapHeight,
            AttrId::XHeight,
            AttrId::AccentHeight,
            AttrId::Ascent,
            AttrId::Descent,
            AttrId::Widths,
            AttrId::Bbox,
            AttrId::Ideographic,
            AttrId::Alphabetic,
            AttrId::Mathematical,
            AttrId::Hanging,
            AttrId::VIdeographic,
            AttrId::VAlphabetic,
            AttrId::VMathematical,
            AttrId::VHanging,
            AttrId::UnderlinePosition,
            AttrId::UnderlineThickness,
            AttrId::StrikethroughPosition,
            AttrId::StrikethroughThickness,
            AttrId::OverlinePosition,
            AttrId::OverlineThickness,
        ],
        info: ElementInfo::Legacy,
    },
    FontFaceFormat {
        name: "font-face-format",
        categories: ElementCategory::Uncategorised,
        permitted_categories: ElementCategory::empty(),
        permitted_elements: &[],
        expected_attribute_groups: AttributeGroup::empty(),
        expected_attributes: &[AttrId::String],
        info: ElementInfo::Legacy,
    },
    FontFaceName {
        name: "font-face-name",
        categories: ElementCategory::Uncategorised,
        permitted_categories: ElementCategory::empty(),
        permitted_elements: &[],
        expected_attribute_groups: AttributeGroup::empty(),
        expected_attributes: &[AttrId::NameFontFace],
        info: ElementInfo::Legacy,
    },
    FontFaceSrc {
        name: "font-face-src",
        categories: ElementCategory::Uncategorised,
        permitted_categories: ElementCategory::empty(),
        permitted_elements: &[ElementId::FontFaceName, ElementId::FontFaceURI],
        expected_attribute_groups: AttributeGroup::empty(),
        expected_attributes: &[],
        info: ElementInfo::Legacy,
    },
    FontFaceURI {
        name: "font-face-uri",
        categories: ElementCategory::Uncategorised,
        permitted_categories: ElementCategory::empty(),
        permitted_elements: &[ElementId::FontFaceFormat],
        expected_attribute_groups: AttributeGroup::XLink,
        expected_attributes: &[],
        info: ElementInfo::Legacy,
    },
    ForeignObject {
        name: "foreignObject",
        categories: ElementCategory::Graphics
            .union(ElementCategory::Renderable)
            .union(ElementCategory::StructurallyExternal),
        permitted_categories: ElementCategory::all(),
        permitted_elements: &[],
        expected_attribute_groups: AttributeGroup::Aria
            .union(AttributeGroup::ConditionalProcessing)
            .union(AttributeGroup::GlobalEvent)
            .union(AttributeGroup::DocumentElementEvent)
            .union(AttributeGroup::GraphicalEvent)
            .union(AttributeGroup::Presentation),
        expected_attributes: &[
            AttrId::ExternalResourcesRequired,
            AttrId::XGeometry,
            AttrId::YGeometry,
            AttrId::WidthForeignObject,
            AttrId::HeightForeignObject,
        ],
    },
    G {
        name: "g",
        categories: ElementCategory::Container
            .union(ElementCategory::Renderable)
            .union(ElementCategory::Structural),
        permitted_categories: ElementCategory::Animation
            .union(ElementCategory::Descriptive)
            .union(ElementCategory::PaintServer)
            .union(ElementCategory::Shape)
            .union(ElementCategory::Structural)
            .union(ElementCategory::Gradient),
        permitted_elements: &[
            ElementId::A,
            ElementId::AltGlyphDef,
            ElementId::ClipPath,
            ElementId::ColorProfile,
            ElementId::Cursor,
            ElementId::Filter,
            ElementId::Font,
            ElementId::FontFace,
            ElementId::ForeignObject,
            ElementId::Image,
            ElementId::Marker,
            ElementId::Mask,
            ElementId::Pattern,
            ElementId::Script,
            ElementId::Style,
            ElementId::Switch,
            ElementId::Text,
            ElementId::View,
        ],
        expected_attribute_groups: AttributeGroup::Aria
            .union(AttributeGroup::ConditionalProcessing)
            .union(AttributeGroup::GraphicalEvent)
            .union(AttributeGroup::GlobalEvent)
            .union(AttributeGroup::DocumentElementEvent)
            .union(AttributeGroup::Presentation),
        expected_attributes: &[
            AttrId::ExternalResourcesRequired,
        ],
    },
    Glyph {
        name: "glyph",
        categories: ElementCategory::Container,
        permitted_categories: ElementCategory::Animation
            .union(ElementCategory::Descriptive)
            .union(ElementCategory::Shape)
            .union(ElementCategory::Structural)
            .union(ElementCategory::Gradient),
        permitted_elements: &[
            ElementId::A,
            ElementId::AltGlyphDef,
            ElementId::ClipPath,
            ElementId::ColorProfile,
            ElementId::Cursor,
            ElementId::Filter,
            ElementId::Font,
            ElementId::FontFace,
            ElementId::ForeignObject,
            ElementId::Image,
            ElementId::Marker,
            ElementId::Mask,
            ElementId::Pattern,
            ElementId::Script,
            ElementId::Style,
            ElementId::Switch,
            ElementId::Text,
            ElementId::View,
        ],
        expected_attribute_groups: AttributeGroup::Presentation,
        expected_attributes: &[
            AttrId::D,
            AttrId::HorizAdvX,
            AttrId::VertOriginX,
            AttrId::VertOriginY,
            AttrId::VertAdvY,
            AttrId::Unicode,
            AttrId::GlyphName,
            AttrId::Orientation,
            AttrId::ArabicForm,
        ],
        info: ElementInfo::Legacy,
    },
    GlyphRef {
        name: "glyphRef",
        categories: ElementCategory::Uncategorised,
        permitted_categories: ElementCategory::empty(),
        permitted_elements: &[],
        expected_attribute_groups: AttributeGroup::Presentation
            .union(AttributeGroup::XLink),
        expected_attributes: &[
            AttrId::XGlyphRef,
            AttrId::YGlyphRef,
            AttrId::DXGlyphRef,
            AttrId::DYGlyphRef,
            AttrId::GlyphRef,
            AttrId::Format,
        ],
        info: ElementInfo::Legacy,
    },
    // https://docs.w3cub.com/svg/element/hatch.html
    // https://www.w3.org/TR/2015/WD-SVG2-20150915/pservers.html#Hatches
    Hatch {
        name: "hatch",
        categories: ElementCategory::Animation.union(ElementCategory::PaintServer),
        permitted_categories: ElementCategory::Animation.union(ElementCategory::Descriptive),
        permitted_elements: &[ElementId::Script, ElementId::Style, ElementId::HatchPath],
        expected_attribute_groups: AttributeGroup::GlobalEvent
            .union(AttributeGroup::Presentation),
        expected_attributes: &[
            AttrId::XHatch,
            AttrId::YHatch,
            AttrId::Pitch,
            AttrId::RotateHatch,
            AttrId::HatchUnits,
            AttrId::HatchContentUnits,
            AttrId::Href,
        ],
    },
    // https://docs.w3cub.com/svg/element/hatchpath
    HatchPath {
        name: "hatchpath",
        categories: ElementCategory::Uncategorised,
        permitted_categories: ElementCategory::Animation.union(ElementCategory::Descriptive),
        permitted_elements: &[ElementId::Script, ElementId::Style],
        expected_attribute_groups: AttributeGroup::GlobalEvent
            .union(AttributeGroup::Presentation),
        expected_attributes: &[
            AttrId::D,
            AttrId::OffsetHatchPath,
        ],
    },
    HKern {
        name: "hkern",
        categories: ElementCategory::Uncategorised,
        permitted_categories: ElementCategory::empty(),
        permitted_elements: &[],
        expected_attribute_groups: AttributeGroup::empty(),
        expected_attributes: &[
            AttrId::U1,
            AttrId::G1,
            AttrId::U2,
            AttrId::G2,
            AttrId::K,
        ],
        info: ElementInfo::Legacy,
    },
    Image {
        name: "image",
        categories: ElementCategory::Graphics
            .union(ElementCategory::GraphicsReferencing)
            .union(ElementCategory::Renderable)
            .union(ElementCategory::StructurallyExternal),
        permitted_categories: ElementCategory::Animation
            .union(ElementCategory::Descriptive),
        permitted_elements: &[
            ElementId::ClipPath,
            ElementId::Mask,
            ElementId::Script,
            ElementId::Style,
        ],
        expected_attribute_groups: AttributeGroup::Aria
            .union(AttributeGroup::ConditionalProcessing)
            .union(AttributeGroup::GlobalEvent)
            .union(AttributeGroup::DocumentElementEvent)
            .union(AttributeGroup::GraphicalEvent)
            .union(AttributeGroup::XLink)
            .union(AttributeGroup::Presentation),
        expected_attributes: &[
            AttrId::ExternalResourcesRequired,
            AttrId::PreserveAspectRatio,
            AttrId::Href,
            AttrId::CrossOrigin,
            AttrId::XGeometry,
            AttrId::YGeometry,
            AttrId::WidthImage,
            AttrId::HeightImage,
        ],
    },
    Line {
        name: "line",
        categories: ElementCategory::BasicShape
            .union(ElementCategory::Graphics)
            .union(ElementCategory::Renderable)
            .union(ElementCategory::Shape),
        permitted_categories: ElementCategory::Animation
            .union(ElementCategory::Descriptive)
            .union(ElementCategory::PaintServer),
        permitted_elements: &[
            ElementId::ClipPath,
            ElementId::Marker,
            ElementId::Mask,
            ElementId::Script,
            ElementId::Style,
        ],
        expected_attribute_groups: AttributeGroup::Aria
            .union(AttributeGroup::ConditionalProcessing)
            .union(AttributeGroup::GlobalEvent)
            .union(AttributeGroup::DocumentElementEvent)
            .union(AttributeGroup::GraphicalEvent)
            .union(AttributeGroup::Presentation),
        expected_attributes: &[
            AttrId::ExternalResourcesRequired,
            AttrId::PathLength,
            AttrId::X1Line,
            AttrId::X2Line,
            AttrId::Y1Line,
            AttrId::Y2Line,
        ],
    },
    LinearGradient {
        name: "linearGradient",
        categories: ElementCategory::Gradient
            .union(ElementCategory::NeverRendered)
            .union(ElementCategory::PaintServer),
        permitted_categories: ElementCategory::Descriptive,
        permitted_elements: &[
            ElementId::Animate,
            ElementId::AnimateTransform,
            ElementId::Script,
            ElementId::Set,
            ElementId::Stop,
            ElementId::Style,
        ],
        expected_attribute_groups: AttributeGroup::GlobalEvent
            .union(AttributeGroup::DocumentElementEvent)
            .union(AttributeGroup::Presentation)
            .union(AttributeGroup::XLink),
        expected_attributes: &[
            AttrId::ExternalResourcesRequired,
            AttrId::X1LinearGradient,
            AttrId::X2LinearGradient,
            AttrId::Y1LinearGradient,
            AttrId::Y2LinearGradient,
            AttrId::GradientUnits,
            AttrId::GradientTransform,
            AttrId::SpreadMethod,
            AttrId::Href,
        ],
        info: ElementInfo::NonRendering,
    },
    Marker {
        name: "marker",
        categories: ElementCategory::Container
            .union(ElementCategory::NeverRendered),
        permitted_categories: ElementCategory::Animation
            .union(ElementCategory::Descriptive)
            .union(ElementCategory::PaintServer)
            .union(ElementCategory::Shape)
            .union(ElementCategory::Structural)
            .union(ElementCategory::Gradient),
        permitted_elements: &[
            ElementId::A,
            ElementId::AltGlyphDef,
            ElementId::ClipPath,
            ElementId::ColorProfile,
            ElementId::Cursor,
            ElementId::Filter,
            ElementId::Font,
            ElementId::FontFace,
            ElementId::ForeignObject,
            ElementId::Image,
            ElementId::Marker,
            ElementId::Mask,
            ElementId::Pattern,
            ElementId::Script,
            ElementId::Style,
            ElementId::Switch,
            ElementId::Text,
            ElementId::View,
        ],
        expected_attribute_groups: AttributeGroup::GlobalEvent
            .union(AttributeGroup::DocumentElementEvent)
            .union(AttributeGroup::Presentation),
        expected_attributes: &[
            AttrId::ExternalResourcesRequired,
            AttrId::ViewBox,
            AttrId::PreserveAspectRatio,
            AttrId::RefX,
            AttrId::RefY,
            AttrId::MarkerUnits,
            AttrId::MarkerWidth,
            AttrId::MarkerHeight,
            AttrId::Orient,
        ],
        info: ElementInfo::NonRendering,
    },
    Mask {
        name: "mask",
        categories: ElementCategory::Container
            .union(ElementCategory::NeverRendered),
        permitted_categories: ElementCategory::Animation
            .union(ElementCategory::Descriptive)
            .union(ElementCategory::Shape)
            .union(ElementCategory::Structural)
            .union(ElementCategory::Gradient),
        permitted_elements: &[
            ElementId::A,
            ElementId::AltGlyphDef,
            ElementId::ClipPath,
            ElementId::ColorProfile,
            ElementId::Cursor,
            ElementId::Filter,
            ElementId::Font,
            ElementId::FontFace,
            ElementId::ForeignObject,
            ElementId::Image,
            ElementId::Marker,
            ElementId::Mask,
            ElementId::Pattern,
            ElementId::Script,
            ElementId::Style,
            ElementId::Switch,
            ElementId::View,
            ElementId::Text,
        ],
        expected_attribute_groups: AttributeGroup::ConditionalProcessing
            .union(AttributeGroup::Presentation),
        expected_attributes: &[
            AttrId::ExternalResourcesRequired,
            AttrId::XMask,
            AttrId::YMask,
            AttrId::WidthMask,
            AttrId::HeightMask,
            AttrId::MaskUnits,
            AttrId::MaskContentUnits,
        ],
        info: ElementInfo::NonRendering,
    },
    Metadata {
        name: "metadata",
        categories: ElementCategory::Descriptive
            .union(ElementCategory::NeverRendered),
        permitted_categories: ElementCategory::all(),
        permitted_elements: &[],
        expected_attribute_groups: AttributeGroup::GlobalEvent
            .union(AttributeGroup::DocumentElementEvent),
        expected_attributes: &[],
    },
    MissingGlyph {
        name: "missing-glyph",
        categories: ElementCategory::Container,
        permitted_categories: ElementCategory::Animation
            .union(ElementCategory::Descriptive)
            .union(ElementCategory::Shape)
            .union(ElementCategory::Structural)
            .union(ElementCategory::Gradient),
        permitted_elements: &[
            ElementId::A,
            ElementId::AltGlyphDef,
            ElementId::ClipPath,
            ElementId::ColorProfile,
            ElementId::Cursor,
            ElementId::Filter,
            ElementId::Font,
            ElementId::FontFace,
            ElementId::ForeignObject,
            ElementId::Image,
            ElementId::Marker,
            ElementId::Mask,
            ElementId::Pattern,
            ElementId::Script,
            ElementId::Style,
            ElementId::Switch,
            ElementId::Text,
            ElementId::View,
        ],
        expected_attribute_groups: AttributeGroup::Presentation,
        expected_attributes: &[
            AttrId::D,
            AttrId::HorizAdvX,
            AttrId::VertOriginX,
            AttrId::VertOriginY,
            AttrId::VertAdvY,
        ],
        info: ElementInfo::Legacy,
    },
    MPath {
        name: "mpath",
        categories: ElementCategory::Animation,
        permitted_categories: ElementCategory::Descriptive,
        permitted_elements: &[ElementId::Script],
        expected_attribute_groups: AttributeGroup::GlobalEvent
            .union(AttributeGroup::DocumentElementEvent)
            .union(AttributeGroup::XLink),
        expected_attributes: &[AttrId::ExternalResourcesRequired, AttrId::Href],
    },
    Path {
        name: "path",
        categories: ElementCategory::Graphics
            .union(ElementCategory::Shape)
            .union(ElementCategory::Renderable),
        permitted_categories: ElementCategory::Animation
            .union(ElementCategory::Descriptive)
            .union(ElementCategory::PaintServer),
        permitted_elements: &[
            ElementId::ClipPath,
            ElementId::Marker,
            ElementId::Mask,
            ElementId::Script,
            ElementId::Style,
        ],
        expected_attribute_groups: AttributeGroup::Aria
            .union(AttributeGroup::ConditionalProcessing)
            .union(AttributeGroup::GlobalEvent)
            .union(AttributeGroup::DocumentElementEvent)
            .union(AttributeGroup::GraphicalEvent)
            .union(AttributeGroup::Presentation),
        expected_attributes: &[
            AttrId::ExternalResourcesRequired,
            AttrId::D,
            AttrId::PathLength,
        ],
    },
    Pattern {
        name: "pattern",
        categories: ElementCategory::Container
            .union(ElementCategory::NeverRendered)
            .union(ElementCategory::PaintServer),
        permitted_categories: ElementCategory::Animation
            .union(ElementCategory::Descriptive)
            .union(ElementCategory::PaintServer)
            .union(ElementCategory::Shape)
            .union(ElementCategory::Structural)
            .union(ElementCategory::Gradient),
        permitted_elements: &[
            ElementId::A,
            ElementId::AltGlyphDef,
            ElementId::ClipPath,
            ElementId::ColorProfile,
            ElementId::Cursor,
            ElementId::Filter,
            ElementId::Font,
            ElementId::FontFace,
            ElementId::ForeignObject,
            ElementId::Image,
            ElementId::Marker,
            ElementId::Mask,
            ElementId::Pattern,
            ElementId::Script,
            ElementId::Style,
            ElementId::Switch,
            ElementId::Text,
            ElementId::View,
        ],
        expected_attribute_groups: AttributeGroup::GlobalEvent
            .union(AttributeGroup::DocumentElementEvent)
            .union(AttributeGroup::Presentation)
            .union(AttributeGroup::XLink),
        expected_attributes: &[
            AttrId::ExternalResourcesRequired,
            AttrId::ViewBox,
            AttrId::PreserveAspectRatio,
            AttrId::XPattern,
            AttrId::YPattern,
            AttrId::WidthPattern,
            AttrId::HeightPattern,
            AttrId::PatternUnits,
            AttrId::PatternContentUnits,
            AttrId::PatternTransform,
            AttrId::Href,
        ],
        info: ElementInfo::NonRendering,
    },
    Polygon {
        name: "polygon",
        categories: ElementCategory::Graphics
            .union(ElementCategory::Renderable)
            .union(ElementCategory::Shape)
            .union(ElementCategory::BasicShape),
        permitted_categories: ElementCategory::Animation
            .union(ElementCategory::Descriptive)
            .union(ElementCategory::PaintServer),
        permitted_elements: &[
            ElementId::ClipPath,
            ElementId::Marker,
            ElementId::Mask,
            ElementId::Script,
            ElementId::Style,
        ],
        expected_attribute_groups: AttributeGroup::Aria
            .union(AttributeGroup::ConditionalProcessing)
            .union(AttributeGroup::GlobalEvent)
            .union(AttributeGroup::DocumentElementEvent)
            .union(AttributeGroup::GraphicalEvent)
            .union(AttributeGroup::Presentation),
        expected_attributes: &[
            AttrId::ExternalResourcesRequired,
            AttrId::PathLength,
            AttrId::Points,
        ],
    },
    Polyline {
        name: "polyline",
        categories: ElementCategory::BasicShape
            .union(ElementCategory::Renderable)
            .union(ElementCategory::Graphics)
            .union(ElementCategory::Shape),
        permitted_categories: ElementCategory::Animation
            .union(ElementCategory::Descriptive)
            .union(ElementCategory::PaintServer),
        permitted_elements: &[
            ElementId::ClipPath,
            ElementId::Marker,
            ElementId::Mask,
            ElementId::Script,
            ElementId::Style,
        ],
        expected_attribute_groups: AttributeGroup::Aria
            .union(AttributeGroup::ConditionalProcessing)
            .union(AttributeGroup::GlobalEvent)
            .union(AttributeGroup::DocumentElementEvent)
            .union(AttributeGroup::GraphicalEvent)
            .union(AttributeGroup::Presentation),
        expected_attributes: &[
            AttrId::ExternalResourcesRequired,
            AttrId::PathLength,
            AttrId::Points,
        ],
    },
    RadialGradient {
        name: "radialGradient",
        categories: ElementCategory::Gradient
            .union(ElementCategory::NeverRendered)
            .union(ElementCategory::PaintServer),
        permitted_categories: ElementCategory::Descriptive,
        permitted_elements: &[
            ElementId::Animate,
            ElementId::AnimateTransform,
            ElementId::Script,
            ElementId::Set,
            ElementId::Stop,
            ElementId::Style,
        ],
        expected_attribute_groups: AttributeGroup::GlobalEvent
            .union(AttributeGroup::DocumentElementEvent)
            .union(AttributeGroup::Presentation)
            .union(AttributeGroup::XLink),
        expected_attributes: &[
            AttrId::ExternalResourcesRequired,
            AttrId::CXRadialGradient,
            AttrId::CYRadialGradient,
            AttrId::RRadialGradient,
            AttrId::FX,
            AttrId::FY,
            AttrId::FR,
            AttrId::GradientUnits,
            AttrId::GradientTransform,
            AttrId::SpreadMethod,
            AttrId::Href,
        ],
        info: ElementInfo::NonRendering,
    },
    Rect {
        name: "rect",
        categories: ElementCategory::BasicShape
            .union(ElementCategory::Graphics)
            .union(ElementCategory::Renderable)
            .union(ElementCategory::Shape),
        permitted_categories: ElementCategory::Animation
            .union(ElementCategory::Descriptive)
            .union(ElementCategory::PaintServer),
        permitted_elements: &[
            ElementId::ClipPath,
            ElementId::Marker,
            ElementId::Mask,
            ElementId::Script,
            ElementId::Style,
        ],
        expected_attribute_groups: AttributeGroup::Aria
            .union(AttributeGroup::ConditionalProcessing)
            .union(AttributeGroup::GlobalEvent)
            .union(AttributeGroup::DocumentElementEvent)
            .union(AttributeGroup::GraphicalEvent)
            .union(AttributeGroup::Presentation),
        expected_attributes: &[
            AttrId::ExternalResourcesRequired,
            AttrId::PathLength,
            AttrId::XGeometry,
            AttrId::YGeometry,
            AttrId::WidthRect,
            AttrId::HeightRect,
            AttrId::RX,
            AttrId::RY,
        ],
    },
    Script {
        name: "script",
        categories: ElementCategory::NeverRendered
            .union(ElementCategory::StructurallyExternal),
        permitted_categories: ElementCategory::empty(),
        permitted_elements: &[],
        expected_attribute_groups: AttributeGroup::GlobalEvent
            .union(AttributeGroup::DocumentElementEvent)
            .union(AttributeGroup::XLink),
        expected_attributes: &[
            AttrId::ExternalResourcesRequired,
            AttrId::TypeScript,
            AttrId::Href,
            AttrId::CrossOrigin
        ],
    },
    Set {
        name: "set",
        categories: ElementCategory::Animation,
        permitted_categories: ElementCategory::Descriptive,
        permitted_elements: &[ElementId::Script],
        expected_attribute_groups: AttributeGroup::AnimationEvent
            .union(AttributeGroup::AnimationTargetElement)
            .union(AttributeGroup::AnimationAttributeTarget)
            .union(AttributeGroup::AnimationTiming)
            .union(AttributeGroup::ConditionalProcessing)
            .union(AttributeGroup::GlobalEvent)
            .union(AttributeGroup::DocumentElementEvent)
            .union(AttributeGroup::XLink),
        expected_attributes: &[
            AttrId::ExternalResourcesRequired,
            AttrId::To,
        ],
    },
    SolidColor {
        // NOTE: Not added to SVG 2 yet
        // https://www.w3.org/TR/2012/WD-SVG2-20120828/pservers.html#SolidColorElement
        name: "solidColor",
        categories: ElementCategory::Uncategorised,
        permitted_categories: ElementCategory::empty(),
        permitted_elements: &[
            ElementId::Animate,
            ElementId::AnimateColor,
            ElementId::Set,
        ],
        expected_attribute_groups: AttributeGroup::Presentation,
        expected_attributes: &[
            AttrId::SolidColor,
            AttrId::SolidOpacity,
        ],
        info: ElementInfo::NonRendering,
    },
    Stop {
        name: "stop",
        categories: ElementCategory::empty(),
        permitted_categories: ElementCategory::empty(),
        permitted_elements: &[
            ElementId::Animate,
            ElementId::AnimateColor,
            ElementId::Script,
            ElementId::Set,
            ElementId::Style,
        ],
        expected_attribute_groups: AttributeGroup::GlobalEvent
            .union(AttributeGroup::DocumentElementEvent)
            .union(AttributeGroup::Presentation),
        expected_attributes: &[
            AttrId::OffsetStop
        ],
    },
    Style {
        name: "style",
        categories: ElementCategory::NeverRendered,
        permitted_categories: ElementCategory::empty(),
        permitted_elements: &[],
        expected_attribute_groups: AttributeGroup::GlobalEvent
            .union(AttributeGroup::DocumentElementEvent),
        expected_attributes: &[
            AttrId::TypeStyle,
            AttrId::Media,
            AttrId::Title,
        ],
    },
    Svg {
        name: "svg",
        categories: ElementCategory::Container
            .union(ElementCategory::Renderable)
            .union(ElementCategory::Structural),
        permitted_categories: ElementCategory::Animation
            .union(ElementCategory::Descriptive)
            .union(ElementCategory::PaintServer)
            .union(ElementCategory::Shape)
            .union(ElementCategory::Structural)
            .union(ElementCategory::Gradient),
        permitted_elements: &[
            ElementId::A,
            ElementId::AltGlyphDef,
            ElementId::ClipPath,
            ElementId::ColorProfile,
            ElementId::Cursor,
            ElementId::Filter,
            ElementId::Font,
            ElementId::FontFace,
            ElementId::ForeignObject,
            ElementId::Image,
            ElementId::Marker,
            ElementId::Mask,
            ElementId::Pattern,
            ElementId::Script,
            ElementId::Style,
            ElementId::Switch,
            ElementId::Text,
            ElementId::View,
        ],
        expected_attribute_groups: AttributeGroup::Aria
            .union(AttributeGroup::ConditionalProcessing)
            .union(AttributeGroup::DocumentEvent)
            .union(AttributeGroup::GlobalEvent)
            .union(AttributeGroup::DocumentElementEvent)
            .union(AttributeGroup::GraphicalEvent)
            .union(AttributeGroup::Presentation),
        expected_attributes: &[
            AttrId::ExternalResourcesRequired,
            AttrId::XGeometry,
            AttrId::YGeometry,
            AttrId::WidthSvg,
            AttrId::HeightSvg,
            AttrId::ViewBox,
            AttrId::PreserveAspectRatio,
            AttrId::ZoomAndPan,
            AttrId::Version,
            AttrId::BaseProfile,
            AttrId::ContentScriptType,
            AttrId::ContentStyleType,
            AttrId::Playbackorder,
            AttrId::Timelinebegin,
        ],
    },
    Switch {
        name: "switch",
        categories: ElementCategory::Container.union(ElementCategory::Renderable),
        permitted_categories: ElementCategory::Animation
            .union(ElementCategory::Shape),
        permitted_elements: &[
            ElementId::A,
            ElementId::ForeignObject,
            ElementId::G,
            ElementId::Image,
            ElementId::Svg,
            ElementId::Switch,
            ElementId::Text,
            ElementId::Use,
        ],
        expected_attribute_groups: AttributeGroup::Aria
            .union(AttributeGroup::ConditionalProcessing)
            .union(AttributeGroup::GlobalEvent)
            .union(AttributeGroup::DocumentElementEvent)
            .union(AttributeGroup::GraphicalEvent)
            .union(AttributeGroup::Presentation),
        expected_attributes: &[
            AttrId::ExternalResourcesRequired,
        ],
    },
    Symbol {
        name: "symbol",
        categories: ElementCategory::Container.union(ElementCategory::Structural),
        permitted_categories: ElementCategory::Animation
            .union(ElementCategory::Descriptive)
            .union(ElementCategory::PaintServer)
            .union(ElementCategory::Shape)
            .union(ElementCategory::Structural)
            .union(ElementCategory::Gradient),
        permitted_elements: &[
            ElementId::A,
            ElementId::AltGlyphDef,
            ElementId::ClipPath,
            ElementId::ColorProfile,
            ElementId::Cursor,
            ElementId::Filter,
            ElementId::Font,
            ElementId::FontFace,
            ElementId::ForeignObject,
            ElementId::Image,
            ElementId::Marker,
            ElementId::Mask,
            ElementId::Pattern,
            ElementId::Script,
            ElementId::Style,
            ElementId::Switch,
            ElementId::Text,
            ElementId::View,
        ],
        expected_attribute_groups: AttributeGroup::Aria
            .union(AttributeGroup::GlobalEvent)
            .union(AttributeGroup::DocumentElementEvent)
            .union(AttributeGroup::GraphicalEvent)
            .union(AttributeGroup::Presentation),
        expected_attributes: &[
            AttrId::ExternalResourcesRequired,
            AttrId::PreserveAspectRatio,
            AttrId::ViewBox,
            AttrId::RefX,
            AttrId::RefY,
            AttrId::XGeometry,
            AttrId::YGeometry,
            AttrId::WidthSymbol,
            AttrId::HeightSymbol,
        ],
        info: ElementInfo::NonRendering,
    },
    Text {
        name: "text",
        categories: ElementCategory::Graphics
            .union(ElementCategory::Renderable)
            .union(ElementCategory::TextContent),
        permitted_categories: ElementCategory::Animation
            .union(ElementCategory::Descriptive)
            .union(ElementCategory::PaintServer)
            .union(ElementCategory::TextContentChild),
        permitted_elements: &[
            ElementId::A,
            ElementId::ClipPath,
            ElementId::Marker,
            ElementId::Mask,
            ElementId::Script,
            ElementId::Style,
        ],
        expected_attribute_groups: AttributeGroup::Aria
            .union(AttributeGroup::ConditionalProcessing)
            .union(AttributeGroup::GlobalEvent)
            .union(AttributeGroup::DocumentElementEvent)
            .union(AttributeGroup::GraphicalEvent)
            .union(AttributeGroup::Presentation),
        expected_attributes: &[
            AttrId::ExternalResourcesRequired,
            AttrId::LengthAdjust,
            AttrId::XText,
            AttrId::YText,
            AttrId::DXText,
            AttrId::DYText,
            AttrId::RotateText,
            AttrId::TextLength,
        ],
    },
    TextPath {
        name: "textPath",
        categories: ElementCategory::Graphics
            .union(ElementCategory::Renderable)
            .union(ElementCategory::TextContent)
            .union(ElementCategory::TextContentChild),
        permitted_categories: ElementCategory::Descriptive.union(ElementCategory::PaintServer),
        permitted_elements: &[
            ElementId::A,
            ElementId::AltGlyph,
            ElementId::Animate,
            ElementId::AnimateColor,
            ElementId::ClipPath,
            ElementId::Marker,
            ElementId::Mask,
            ElementId::Script,
            ElementId::Set,
            ElementId::TRef,
            ElementId::Style,
            ElementId::TSpan,
        ],
        expected_attribute_groups: AttributeGroup::Aria
            .union(AttributeGroup::ConditionalProcessing)
            .union(AttributeGroup::GlobalEvent)
            .union(AttributeGroup::DocumentElementEvent)
            .union(AttributeGroup::GraphicalEvent)
            .union(AttributeGroup::Presentation)
            .union(AttributeGroup::XLink),
        expected_attributes: &[
            AttrId::ExternalResourcesRequired,
            AttrId::LengthAdjust,
            AttrId::TextLength,
            AttrId::Path,
            AttrId::Href,
            AttrId::StartOffset,
            AttrId::Method,
            AttrId::Spacing,
            AttrId::Side,
        ],
    },
    Title {
        name: "title",
        categories: ElementCategory::Descriptive.union(ElementCategory::NeverRendered),
        permitted_categories: ElementCategory::all(),
        permitted_elements: &[],
        expected_attribute_groups: AttributeGroup::GlobalEvent
            .union(AttributeGroup::DocumentElementEvent),
        expected_attributes: &[],
    },
    TRef {
        name: "tref",
        categories: ElementCategory::TextContent.union(ElementCategory::TextContentChild),
        permitted_categories: ElementCategory::Descriptive,
        permitted_elements: &[
            ElementId::Animate,
            ElementId::AnimateColor,
            ElementId::Set,
        ],
        expected_attribute_groups: AttributeGroup::ConditionalProcessing
            .union(AttributeGroup::GraphicalEvent)
            .union(AttributeGroup::Presentation)
            .union(AttributeGroup::XLink),
        expected_attributes: &[
            AttrId::ExternalResourcesRequired,
            AttrId::XTRef,
            AttrId::DXTSpan,
            AttrId::DYTSpan,
            AttrId::RotateText,
        ],
        info: ElementInfo::Legacy,
    },
    TSpan {
        name: "tspan",
        categories: ElementCategory::Graphics
            .union(ElementCategory::Renderable)
            .union(ElementCategory::TextContent)
            .union(ElementCategory::TextContentChild),
        permitted_categories: ElementCategory::Descriptive.union(ElementCategory::PaintServer),
        permitted_elements: &[
            ElementId::A,
            ElementId::AltGlyph,
            ElementId::Animate,
            ElementId::AnimateColor,
            ElementId::Script,
            ElementId::Set,
            ElementId::Style,
            ElementId::TRef,
            ElementId::TSpan,
        ],
        expected_attribute_groups: AttributeGroup::Aria
            .union(AttributeGroup::ConditionalProcessing)
            .union(AttributeGroup::GlobalEvent)
            .union(AttributeGroup::DocumentElementEvent)
            .union(AttributeGroup::GlobalEvent)
            .union(AttributeGroup::Presentation),
        expected_attributes: &[
            AttrId::ExternalResourcesRequired,
            AttrId::XTRef,
            AttrId::YTRef,
            AttrId::DXTSpan,
            AttrId::DYTSpan,
            AttrId::RotateText,
            AttrId::TextLength,
            AttrId::LengthAdjust,
        ],
    },
    Use {
        name: "use",
        categories: ElementCategory::Graphics
            .union(ElementCategory::GraphicsReferencing)
            .union(ElementCategory::Renderable)
            .union(ElementCategory::Structural)
            .union(ElementCategory::StructurallyExternal),
        permitted_categories: ElementCategory::Animation
            .union(ElementCategory::Descriptive),
        permitted_elements: &[
            ElementId::ClipPath,
            ElementId::Mask,
            ElementId::Script,
            ElementId::Style,
        ],
        expected_attribute_groups: AttributeGroup::Aria
            .union(AttributeGroup::ConditionalProcessing)
            .union(AttributeGroup::GlobalEvent)
            .union(AttributeGroup::DocumentElementEvent)
            .union(AttributeGroup::GraphicalEvent)
            .union(AttributeGroup::Presentation)
            .union(AttributeGroup::XLink),
        expected_attributes: &[
            AttrId::ExternalResourcesRequired,
            AttrId::Href,
            AttrId::XGeometry,
            AttrId::YGeometry,
            AttrId::WidthUse,
            AttrId::HeightUse,
        ],
    },
    View {
        name: "view",
        categories: ElementCategory::Uncategorised,
        permitted_categories: ElementCategory::Animation.union(ElementCategory::Descriptive),
        permitted_elements: &[ElementId::Script, ElementId::Style],
        expected_attribute_groups: AttributeGroup::Aria
            .union(AttributeGroup::GlobalEvent)
            .union(AttributeGroup::DocumentElementEvent),
        expected_attributes: &[
            AttrId::ExternalResourcesRequired,
            AttrId::ViewBox,
            AttrId::PreserveAspectRatio,
            AttrId::ZoomAndPan,
            AttrId::ViewTarget,
        ],
    },
    VKern {
        name: "vkern",
        categories: ElementCategory::Uncategorised,
        permitted_categories: ElementCategory::empty(),
        permitted_elements: &[],
        expected_attribute_groups: AttributeGroup::empty(),
        expected_attributes: &[
            AttrId::U1,
            AttrId::G1,
            AttrId::U2,
            AttrId::G2,
            AttrId::K,
        ],
    },
}
