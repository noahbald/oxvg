use crate::element::Element;

use crate::{
    atom::Atom,
    attribute::{data::AttrId, AttributeGroup},
    element::category::ElementCategory,
    name::{Prefix, QualName},
};
use std::{collections::VecDeque, fmt::Display};

/// An element's category.
type C = ElementCategory;
/// Permitted category of a child.
type PC = ElementCategory;
/// Permitted child.
type PE = &'static [ElementId<'static>];
/// Expected attributes.
type EA = &'static [AttrId<'static>];
/// Expected attribute groups.
type EAG = AttributeGroup;

pub type LocalName<'input> = Atom<'input>;

const UNKNOWN_PE: PE = &[];

macro_rules! define_elements {
    ($($element:ident {
        name: $name:literal,
        categories: $categories:expr,
        permitted_categories: $permitted_categories:expr,
        permitted_elements: $permitted_elements:expr,
        expected_attribute_groups: $expected_attribute_groups:expr,
        expected_attributes: $expected_attributes:expr,
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
        mod _qual_name {
            use crate::name::{Prefix, QualName};
            use crate::atom::Atom;
            $(pub const $element: &'static QualName<'static> = &QualName {
                prefix: Prefix::SVG,
                local: Atom::Static($name),
            };)+
        }
        #[allow(non_upper_case_globals)]
        mod _local_name {
            use crate::atom::Atom;
            use super::_qual_name;
            $(pub const $element: &'static Atom<'static> = &_qual_name::$element.local;)+
        }

        #[derive(Clone, Debug, Eq, PartialEq, Hash)]
        pub enum ElementId<'input> {
            $($element,)+
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
                    _ => &Prefix::SVG,
                }
            }

            /// Returns the local part of the qualified name.
            pub fn local_name(&self) -> &LocalName<'input> {
                match self {
                    $(Self::$element => _local_name::$element,)+
                    Self::Unknown(name) => &name.local,
                }
            }

            pub fn qual_name(&self) -> &QualName<'input> {
                match self {
                    $(Self::$element => _qual_name::$element,)+
                    Self::Unknown(name) => name,
                }
            }

            /// Returns the element's category
            pub fn categories(&self) -> C {
                match self {
                    $(Self::$element => _c::$element,)+
                    Self::Unknown(_) => ElementCategory::empty(),
                }
            }

            /// Returns element categories allowed as children
            pub fn permitted_categories(&self) -> PC {
                match self {
                    $(Self::$element => _pc::$element,)+
                    Self::Unknown(_) => ElementCategory::all(),
                }
            }

            /// Returns specific elements allowed as children
            pub fn permitted_elements(&self) -> PE {
                match self {
                    $(Self::$element => _pe::$element,)+
                    Self::Unknown(_) => UNKNOWN_PE,
                }
            }

            /// Whether the child is allowed within the SVG element.
            pub fn is_permitted_category(&self, child: &Self) -> bool {
                let permitted_categories = self.permitted_categories();
                if permitted_categories.contains(ElementCategory::all()) {
                    return true;
                }

                let child_categories = child.categories();
                if child_categories.contains(ElementCategory::all()) {
                    return true;
                }
                permitted_categories.contains(child_categories)
                    || self.permitted_elements().contains(child)
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

        impl<'input> Display for ElementId<'input> {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                let local_name: &str = &*self.local_name();
                match self.prefix() {
                    Prefix::SVG => Display::fmt(local_name, f),
                    p => f.write_fmt(format_args!("{p}:{local_name}")),
                }
            }
        }
    };
}

define_elements! {
    A {
        name: "a",
        categories: ElementCategory::Container,
        permitted_categories: ElementCategory::Animation
            .union(ElementCategory::Descriptive)
            .union(ElementCategory::Shape)
            .union(ElementCategory::Structural)
            .union(ElementCategory::Gradient),
        permitted_elements: &[
            ElementId::A,
            ElementId::ClipPath,
            ElementId::Filter,
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
            .union(AttributeGroup::Core)
            .union(AttributeGroup::GlobalEvent)
            .union(AttributeGroup::DocumentElementEvent)
            .union(AttributeGroup::Presentation)
            .union(AttributeGroup::XLink),
        expected_attributes: &[
            AttrId::Download,
            AttrId::Href,
            AttrId::Hreflang,
            AttrId::Ping,
            AttrId::ReferrerPolicy,
            AttrId::Rel,
            AttrId::Target,
            AttrId::Type,
            AttrId::XLinkHref,
        ],
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
            .union(AttributeGroup::Core)
            .union(AttributeGroup::GlobalEvent)
            .union(AttributeGroup::DocumentElementEvent)
            .union(AttributeGroup::Presentation),
        expected_attributes: &[],
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
            .union(AttributeGroup::Core)
            .union(AttributeGroup::GlobalEvent)
            .union(AttributeGroup::DocumentElementEvent),
        expected_attributes: &[AttrId::Path, AttrId::KeyPoints, AttrId::Rotate, AttrId::Origin],
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
                .union(AttributeGroup::Core)
                .union(AttributeGroup::GlobalEvent)
                .union(AttributeGroup::DocumentElementEvent),
        expected_attributes: &[AttrId::Type],
    },
    Circle {
        name: "circle",
        categories: ElementCategory::Shape,
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
                .union(AttributeGroup::Core)
                .union(AttributeGroup::GlobalEvent)
                .union(AttributeGroup::DocumentElementEvent)
                .union(AttributeGroup::Presentation),
        expected_attributes: &[AttrId::PathLength, AttrId::CX, AttrId::CY, AttrId::R],
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
                .union(AttributeGroup::Core)
                .union(AttributeGroup::Presentation),
        expected_attributes: &[
            AttrId::Class,
            AttrId::Style,
            AttrId::ExternalResourcesRequired,
            AttrId::Transform,
            AttrId::ClipPathUnits,
        ],
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
            .union(ElementCategory::Structural),
        permitted_elements: &[
            ElementId::A,
            ElementId::ClipPath,
            ElementId::Filter,
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
        expected_attribute_groups: AttributeGroup::Core
            .union(AttributeGroup::GlobalEvent)
            .union(AttributeGroup::DocumentElementEvent)
            .union(AttributeGroup::Presentation),
        expected_attributes: &[],
    },
    Desc {
        name: "desc",
        categories: ElementCategory::Descriptive.union(ElementCategory::NeverRendered),
        permitted_categories: ElementCategory::all(),
        permitted_elements: &[],
        expected_attribute_groups: AttributeGroup::Core
            .union(AttributeGroup::GlobalEvent)
            .union(AttributeGroup::DocumentElementEvent),
        expected_attributes: &[],
    },
    Ellipse {
        name: "ellipse",
        categories: ElementCategory::BasicShape
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
            .union(AttributeGroup::Core)
            .union(AttributeGroup::GlobalEvent)
            .union(AttributeGroup::DocumentElementEvent)
            .union(AttributeGroup::Presentation),
        expected_attributes: &[AttrId::PathLength, AttrId::CX, AttrId::CY, AttrId::RX, AttrId::RY],
    },
    FeBlend {
        name: "feBlend",
        categories: ElementCategory::FilterPrimitive,
        permitted_categories: ElementCategory::Descriptive,
        permitted_elements: &[ElementId::Animate, ElementId::Script, ElementId::Set],
        expected_attribute_groups: AttributeGroup::Core
            .union(AttributeGroup::Presentation)
            .union(AttributeGroup::FilterPrimitive),
        expected_attributes: &[
            AttrId::Class,
            AttrId::Style,
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
        expected_attribute_groups: AttributeGroup::Core
            .union(AttributeGroup::Presentation)
            .union(AttributeGroup::FilterPrimitive),
        expected_attributes: &[
            AttrId::Class,
            AttrId::Style,
            AttrId::In,
            AttrId::FeColorMatrixType,
            AttrId::FeColorMatrixValues,
        ],
    },
    FeComponentTransfer {
        name: "feComponentTransfer",
        categories: ElementCategory::FilterPrimitive,
        permitted_categories: ElementCategory::Descriptive,
        permitted_elements: &[
            ElementId::FeFuncA,
            ElementId::FeFuncB,
            ElementId::FeFuncR,
            ElementId::FeFuncG,
            ElementId::Script,
        ],
        expected_attribute_groups: AttributeGroup::Core
            .union(AttributeGroup::Presentation)
            .union(AttributeGroup::FilterPrimitive),
        expected_attributes: &[AttrId::Class, AttrId::Style, AttrId::In],
    },
    FeComposite {
        name: "feComposite",
        categories: ElementCategory::FilterPrimitive,
        permitted_categories: ElementCategory::Descriptive,
        permitted_elements: &[ElementId::Animate, ElementId::Script, ElementId::Set],
        expected_attribute_groups: AttributeGroup::Core
            .union(AttributeGroup::Presentation)
            .union(AttributeGroup::FilterPrimitive),
        expected_attributes: &[
            AttrId::Class,
            AttrId::Style,
            AttrId::In,
            AttrId::In2,
            AttrId::FeCompositeOperator,
            AttrId::FeCompositeK1,
            AttrId::FeCompositeK2,
            AttrId::FeCompositeK3,
            AttrId::FeCompositeK4,
        ],
    },
    FeConvolveMatrix {
        name: "feConvolveMatrix",
        categories: ElementCategory::FilterPrimitive,
        permitted_categories: ElementCategory::Descriptive,
        permitted_elements: &[ElementId::Animate, ElementId::Script, ElementId::Set],
        expected_attribute_groups: AttributeGroup::Core
            .union(AttributeGroup::Presentation)
            .union(AttributeGroup::FilterPrimitive),
        expected_attributes: &[
            AttrId::Class,
            AttrId::Style,
            AttrId::In,
            AttrId::Order,
            AttrId::FeConvolveMatrixKernalMatrix,
            AttrId::FeConvolveMatrixDivisor,
            AttrId::FeConvolveMatrixBias,
            AttrId::FeConvolveMatrixTargetX,
            AttrId::FeConvolveMatrixTargetY,
            AttrId::FeEdgeMode,
            AttrId::FeKernelUnitLength,
            AttrId::FeConvolveMatrixPreserveAlpha,
        ],
    },
    FeDiffuseLighting {
        name: "feDiffuseLighting",
        categories: ElementCategory::FilterPrimitive,
        permitted_categories: ElementCategory::Descriptive.union(ElementCategory::LightSource),
        permitted_elements: &[ElementId::Script],
        expected_attribute_groups: AttributeGroup::Core
            .union(AttributeGroup::Presentation)
            .union(AttributeGroup::FilterPrimitive),
        expected_attributes: &[
            AttrId::Class,
            AttrId::Style,
            AttrId::In,
            AttrId::FeSurfaceScale,
            AttrId::FeDiffuseLightingDiffuseConstant,
            AttrId::FeKernelUnitLength,
        ],
    },
    FeDisplacementMap {
        name: "feDisplacementMap",
        categories: ElementCategory::FilterPrimitive,
        permitted_categories: ElementCategory::Descriptive,
        permitted_elements: &[ElementId::Animate, ElementId::Script, ElementId::Set],
        expected_attribute_groups: AttributeGroup::Core
            .union(AttributeGroup::Presentation)
            .union(AttributeGroup::FilterPrimitive),
        expected_attributes: &[
            AttrId::Class,
            AttrId::Style,
            AttrId::In,
            AttrId::In2,
            AttrId::FeDisplacementMapScale,
            AttrId::FeDisplacementMapXChannelSelector,
            AttrId::FeDisplacementMapYChannelSelector,
        ],
    },
    FeDistantLight {
        name: "feDistantLight",
        categories: ElementCategory::LightSource,
        permitted_categories: ElementCategory::Descriptive,
        permitted_elements: &[ElementId::Animate, ElementId::Set],
        expected_attribute_groups: AttributeGroup::Core,
        expected_attributes: &[AttrId::FeDistantLightAzimuth, AttrId::FeDistantLightElevation],
    },
    FeDropShadow {
        name: "feDropShadow",
        categories: ElementCategory::FilterPrimitive,
        permitted_categories: ElementCategory::Descriptive,
        permitted_elements: &[ElementId::Animate, ElementId::Script, ElementId::Set],
        expected_attribute_groups: AttributeGroup::Core
            .union(AttributeGroup::Presentation)
            .union(AttributeGroup::FilterPrimitive),
        expected_attributes: &[
            AttrId::Class,
            AttrId::Style,
            AttrId::In,
            AttrId::FeStdDeviation,
            AttrId::FeDx,
            AttrId::FeDy,
        ],
    },
    FeFlood {
        name: "feFlood",
        categories: ElementCategory::FilterPrimitive,
        permitted_categories: ElementCategory::Descriptive,
        permitted_elements: &[ElementId::Animate, ElementId::Script, ElementId::Set],
        expected_attribute_groups: AttributeGroup::Core
            .union(AttributeGroup::Presentation)
            .union(AttributeGroup::FilterPrimitive),
        expected_attributes: &[AttrId::Class, AttrId::Style],
    },
    FeFuncA {
        name: "feFuncA",
        categories: ElementCategory::TransferFunction,
        permitted_categories: ElementCategory::Descriptive,
        permitted_elements: &[ElementId::Animate, ElementId::Script, ElementId::Set],
        expected_attribute_groups: AttributeGroup::Core
            .union(AttributeGroup::TransferFunction),
        expected_attributes: &[],
    },
    FeFuncB {
        name: "feFuncB",
        categories: ElementCategory::TransferFunction,
        permitted_categories: ElementCategory::Descriptive,
        permitted_elements: &[ElementId::Animate, ElementId::Script, ElementId::Set],
        expected_attribute_groups: AttributeGroup::Core
            .union(AttributeGroup::TransferFunction),
        expected_attributes: &[],
    },
    FeFuncG {
        name: "feFuncG",
        categories: ElementCategory::TransferFunction,
        permitted_categories: ElementCategory::Descriptive,
        permitted_elements: &[ElementId::Animate, ElementId::Script, ElementId::Set],
        expected_attribute_groups: AttributeGroup::Core
            .union(AttributeGroup::TransferFunction),
        expected_attributes: &[],
    },
    FeFuncR {
        name: "feFuncR",
        categories: ElementCategory::TransferFunction,
        permitted_categories: ElementCategory::Descriptive,
        permitted_elements: &[ElementId::Animate, ElementId::Script, ElementId::Set],
        expected_attribute_groups: AttributeGroup::Core
            .union(AttributeGroup::TransferFunction),
        expected_attributes: &[],
    },
    FeGaussianBlur {
        name: "feGaussianBlur",
        categories: ElementCategory::FilterPrimitive,
        permitted_categories: ElementCategory::Descriptive,
        permitted_elements: &[ElementId::Animate, ElementId::Script, ElementId::Set],
        expected_attribute_groups: AttributeGroup::Core
            .union(AttributeGroup::Presentation)
            .union(AttributeGroup::FilterPrimitive),
        expected_attributes: &[
            AttrId::Class,
            AttrId::Style,
            AttrId::In,
            AttrId::FeStdDeviation,
            AttrId::FeEdgeMode,
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
        expected_attribute_groups: AttributeGroup::Core
            .union(AttributeGroup::Presentation)
            .union(AttributeGroup::FilterPrimitive),
        expected_attributes: &[
            AttrId::Class,
            AttrId::Style,
            AttrId::ExternalResourcesRequired,
            AttrId::PreserveAspectRatio,
            AttrId::XLinkHref,
            AttrId::Href,
            AttrId::CrossOrigin,
        ],
    },
    FeMerge {
        name: "feMerge",
        categories: ElementCategory::FilterPrimitive,
        permitted_categories: ElementCategory::Descriptive,
        permitted_elements: &[ElementId::FeMergeNode, ElementId::Script],
        expected_attribute_groups: AttributeGroup::Core
            .union(AttributeGroup::Presentation)
            .union(AttributeGroup::FilterPrimitive),
        expected_attributes: &[AttrId::Class, AttrId::Style],
    },
    FeMergeNode {
        name: "feMergeNode",
        categories: ElementCategory::Uncategorised,
        permitted_categories: ElementCategory::Descriptive,
        permitted_elements: &[ElementId::Animate, ElementId::Script, ElementId::Set],
        expected_attribute_groups: AttributeGroup::Core
            .union(AttributeGroup::Presentation)
            .union(AttributeGroup::FilterPrimitive),
        expected_attributes: &[
            AttrId::Class,
            AttrId::Style,
            AttrId::In,
            AttrId::FeOperator,
            AttrId::FeRadius,
        ],
    },
    FeMorphology {
        name: "feMorphology",
        categories: ElementCategory::FilterPrimitive,
        permitted_categories: ElementCategory::Descriptive,
        permitted_elements: &[ElementId::Animate, ElementId::Script, ElementId::Set],
        expected_attribute_groups: AttributeGroup::Core
            .union(AttributeGroup::Presentation)
            .union(AttributeGroup::FilterPrimitive),
        expected_attributes: &[
            AttrId::Class,
            AttrId::Style,
            AttrId::In,
            AttrId::FeOperator,
            AttrId::FeRadius,
        ],
    },
    FeOffset {
        name: "feOffset",
        categories: ElementCategory::FilterPrimitive,
        permitted_categories: ElementCategory::Descriptive,
        permitted_elements: &[ElementId::Animate, ElementId::Script, ElementId::Set],
        expected_attribute_groups: AttributeGroup::Core
            .union(AttributeGroup::Presentation)
            .union(AttributeGroup::FilterPrimitive),
        expected_attributes: &[
            AttrId::Class,
            AttrId::Style,
            AttrId::In,
            AttrId::FeDx,
            AttrId::FeDy,
        ],
    },
    FePointLight {
        name: "fePointLight",
        categories: ElementCategory::LightSource,
        permitted_categories: ElementCategory::Descriptive,
        permitted_elements: &[ElementId::Animate, ElementId::Script, ElementId::Set],
        expected_attribute_groups: AttributeGroup::Core,
        expected_attributes: &[
            AttrId::FeX,
            AttrId::FeY,
            AttrId::FeZ,
        ],
    },
    FeSpecularLighting {
        name: "feSpecularLighting",
        categories: ElementCategory::FilterPrimitive,
        permitted_categories: ElementCategory::LightSource
            .union(ElementCategory::Descriptive),
        permitted_elements: &[ElementId::Script],
        expected_attribute_groups: AttributeGroup::Core
            .union(AttributeGroup::Presentation)
            .union(AttributeGroup::FilterPrimitive),
        expected_attributes: &[
            AttrId::Class,
            AttrId::Style,
            AttrId::In,
            AttrId::FeSurfaceScale,
            AttrId::FeSpecularLightingSpecularConstant,
            AttrId::FeSpecularExponent,
            AttrId::FeKernelUnitLength,
        ],
    },
    FeSpotLight {
        name: "feSpotLight",
        categories: ElementCategory::LightSource,
        permitted_categories: ElementCategory::Descriptive,
        permitted_elements: &[ElementId::Animate, ElementId::Script, ElementId::Set],
        expected_attribute_groups: AttributeGroup::Core,
        expected_attributes: &[
            AttrId::FeX,
            AttrId::FeY,
            AttrId::FeZ,
            AttrId::FePointsAtX,
            AttrId::FePointsAtY,
            AttrId::FePointsAtZ,
            AttrId::FeSpecularExponent,
            AttrId::FeSpotLightLimitingConeAngle,
        ],
    },
    FeTile {
        name: "feTile",
        categories: ElementCategory::FilterPrimitive,
        permitted_categories: ElementCategory::Descriptive,
        permitted_elements: &[ElementId::Animate, ElementId::Script, ElementId::Set],
        expected_attribute_groups: AttributeGroup::Core
            .union(AttributeGroup::Presentation)
            .union(AttributeGroup::FilterPrimitive),
        expected_attributes: &[AttrId::Class, AttrId::Style, AttrId::In],
    },
    FeTurbulence {
        name: "feTurbulence",
        categories: ElementCategory::FilterPrimitive,
        permitted_categories: ElementCategory::Descriptive,
        permitted_elements: &[ElementId::Animate, ElementId::Script, ElementId::Set],
        expected_attribute_groups: AttributeGroup::Core
            .union(AttributeGroup::Presentation)
            .union(AttributeGroup::FilterPrimitive),
        expected_attributes: &[
            AttrId::Class,
            AttrId::Style,
            AttrId::FeTurbulenceBaseFrequency,
            AttrId::FeTurbulenceNumOctaves,
            AttrId::FeTurbulenceSeed,
            AttrId::FeTurbulenceStitchTiles,
            AttrId::FeTurbulenceType,
        ],
    },
    Filter {
        name: "filter",
        categories: ElementCategory::Uncategorised,
        permitted_categories: ElementCategory::Descriptive
            .union(ElementCategory::FilterPrimitive),
        permitted_elements: &[ElementId::Animate, ElementId::Script, ElementId::Set],
        expected_attribute_groups: AttributeGroup::Core
            .union(AttributeGroup::Presentation),
        expected_attributes: &[
            AttrId::Class,
            AttrId::Style,
            AttrId::ExternalResourcesRequired,
            AttrId::X,
            AttrId::Y,
            AttrId::Width,
            AttrId::Height,
            AttrId::FilterUnits,
            AttrId::PrimitiveUnits,
        ],
    },
    ForeignObject {
        name: "foreignObject",
        categories: ElementCategory::Graphics
            .union(ElementCategory::Renderable)
            .union(ElementCategory::StructurallyExternal),
        permitted_categories: ElementCategory::empty(),
        permitted_elements: &[],
        expected_attribute_groups: AttributeGroup::Aria
            .union(AttributeGroup::Core)
            .union(AttributeGroup::ConditionalProcessing)
            .union(AttributeGroup::GlobalEvent)
            .union(AttributeGroup::DocumentElementEvent)
            .union(AttributeGroup::Presentation),
        expected_attributes: &[
            AttrId::X,
            AttrId::Y,
            AttrId::Width,
            AttrId::Height,
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
            .union(ElementCategory::Structural),
        permitted_elements: &[
            ElementId::A,
            ElementId::ClipPath,
            ElementId::Filter,
            ElementId::ForeignObject,
            ElementId::Image,
            ElementId::Marker,
            ElementId::Mask,
            ElementId::Script,
            ElementId::Switch,
            ElementId::Text,
            ElementId::View,
        ],
        expected_attribute_groups: AttributeGroup::Aria
            .union(AttributeGroup::ConditionalProcessing)
            .union(AttributeGroup::Core)
            .union(AttributeGroup::GlobalEvent)
            .union(AttributeGroup::DocumentElementEvent)
            .union(AttributeGroup::Presentation),
        expected_attributes: &[],
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
            .union(AttributeGroup::Core)
            .union(AttributeGroup::ConditionalProcessing)
            .union(AttributeGroup::GlobalEvent)
            .union(AttributeGroup::DocumentElementEvent)
            .union(AttributeGroup::XLink)
            .union(AttributeGroup::Presentation),
        expected_attributes: &[
            AttrId::PreserveAspectRatio,
            AttrId::Href,
            AttrId::CrossOrigin,
            AttrId::X,
            AttrId::Y,
            AttrId::Width,
            AttrId::Height,
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
            .union(AttributeGroup::Core)
            .union(AttributeGroup::GlobalEvent)
            .union(AttributeGroup::DocumentElementEvent)
            .union(AttributeGroup::Presentation),
        expected_attributes: &[
            AttrId::PathLength,
            AttrId::X1,
            AttrId::X2,
            AttrId::Y1,
            AttrId::Y2,
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
        expected_attribute_groups: AttributeGroup::Core
            .union(AttributeGroup::GlobalEvent)
            .union(AttributeGroup::DocumentElementEvent)
            .union(AttributeGroup::Presentation)
            .union(AttributeGroup::XLink),
        expected_attributes: &[
            AttrId::X1,
            AttrId::X2,
            AttrId::Y1,
            AttrId::Y2,
            AttrId::GradientUnits,
            AttrId::GradientTransform,
            AttrId::SpreadMethod,
            AttrId::Href,
        ],
    },
    Marker {
        name: "marker",
        categories: ElementCategory::Container
            .union(ElementCategory::NeverRendered),
        permitted_categories: ElementCategory::Animation
            .union(ElementCategory::Descriptive)
            .union(ElementCategory::PaintServer)
            .union(ElementCategory::Shape)
            .union(ElementCategory::Structural),
        permitted_elements: &[
            ElementId::A,
            ElementId::ClipPath,
            ElementId::Filter,
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
        expected_attribute_groups: AttributeGroup::Core
            .union(AttributeGroup::GlobalEvent)
            .union(AttributeGroup::DocumentElementEvent)
            .union(AttributeGroup::Presentation),
        expected_attributes: &[
            AttrId::ViewBox,
            AttrId::PreserveAspectRatio,
            AttrId::RefX,
            AttrId::RefY,
            AttrId::MarkerUnits,
            AttrId::MarkerWidth,
            AttrId::MarkerHeight,
            AttrId::Orient,
        ],
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
            ElementId::ClipPath,
            ElementId::Filter,
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
            .union(AttributeGroup::Core)
            .union(AttributeGroup::Presentation),
        expected_attributes: &[
            AttrId::Class,
            AttrId::Style,
            AttrId::X,
            AttrId::Y,
            AttrId::Width,
            AttrId::Height,
            AttrId::MaskUnits,
            AttrId::MaskContentUnits,
        ],
    },
    Metadata {
        name: "metadata",
        categories: ElementCategory::Descriptive
            .union(ElementCategory::NeverRendered),
        permitted_categories: ElementCategory::all(),
        permitted_elements: &[],
        expected_attribute_groups: AttributeGroup::Core
            .union(AttributeGroup::GlobalEvent)
            .union(AttributeGroup::DocumentElementEvent),
        expected_attributes: &[],
    },
    MPath {
        name: "mpath",
        categories: ElementCategory::Animation,
        permitted_categories: ElementCategory::Descriptive,
        permitted_elements: &[ElementId::Script],
        expected_attribute_groups: AttributeGroup::Core
            .union(AttributeGroup::GlobalEvent)
            .union(AttributeGroup::DocumentElementEvent),
        expected_attributes: &[AttrId::Href],
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
            .union(AttributeGroup::Core)
            .union(AttributeGroup::GlobalEvent)
            .union(AttributeGroup::DocumentElementEvent)
            .union(AttributeGroup::Presentation),
        expected_attributes: &[AttrId::PathLength, AttrId::D],
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
            .union(ElementCategory::Structural),
        permitted_elements: &[
            ElementId::A,
            ElementId::ClipPath,
            ElementId::Filter,
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
        expected_attribute_groups: AttributeGroup::Core
            .union(AttributeGroup::GlobalEvent)
            .union(AttributeGroup::Presentation)
            .union(AttributeGroup::XLink),
        expected_attributes: &[
            AttrId::ViewBox,
            AttrId::PreserveAspectRatio,
            AttrId::X,
            AttrId::Y,
            AttrId::Width,
            AttrId::Height,
            AttrId::PatternUnits,
            AttrId::PatternContentUnits,
            AttrId::PatternTransform,
            AttrId::Href,
        ],
    },
    Polygon {
        name: "polygon",
        categories: ElementCategory::Graphics
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
            .union(AttributeGroup::Core)
            .union(AttributeGroup::GlobalEvent)
            .union(AttributeGroup::DocumentElementEvent)
            .union(AttributeGroup::Presentation),
        expected_attributes: &[AttrId::PathLength, AttrId::Points],
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
            .union(AttributeGroup::Core)
            .union(AttributeGroup::GlobalEvent)
            .union(AttributeGroup::DocumentElementEvent)
            .union(AttributeGroup::Presentation),
        expected_attributes: &[AttrId::PathLength, AttrId::Points],
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
        expected_attribute_groups: AttributeGroup::Core
            .union(AttributeGroup::GlobalEvent)
            .union(AttributeGroup::DocumentElementEvent)
            .union(AttributeGroup::Presentation)
            .union(AttributeGroup::XLink),
        expected_attributes: &[
            AttrId::CX,
            AttrId::CY,
            AttrId::R,
            AttrId::FX,
            AttrId::FY,
            AttrId::FR,
            AttrId::GradientUnits,
            AttrId::GradientTransform,
            AttrId::SpreadMethod,
            AttrId::Href,
        ],
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
            .union(AttributeGroup::Core)
            .union(AttributeGroup::GlobalEvent)
            .union(AttributeGroup::DocumentElementEvent)
            .union(AttributeGroup::Presentation),
        expected_attributes: &[
            AttrId::PathLength,
            AttrId::X,
            AttrId::Y,
            AttrId::Width,
            AttrId::Height,
            AttrId::RX,
            AttrId::RY,
        ],
    },
    Script {
        name: "script",
        categories: ElementCategory::NeverRendered.union(ElementCategory::StructurallyExternal),
        permitted_categories: ElementCategory::empty(),
        permitted_elements: &[],
        expected_attribute_groups: AttributeGroup::Core
            .union(AttributeGroup::GlobalEvent)
            .union(AttributeGroup::DocumentElementEvent)
            .union(AttributeGroup::XLink),
        expected_attributes: &[AttrId::ScriptType, AttrId::Href, AttrId::CrossOrigin],
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
            .union(AttributeGroup::Core)
            .union(AttributeGroup::GlobalEvent)
            .union(AttributeGroup::DocumentElementEvent),
        expected_attributes: &[AttrId::To],
    },
    Stop {
        name: "stop",
        categories: ElementCategory::empty(),
        permitted_categories: ElementCategory::empty(),
        permitted_elements: &[
            ElementId::Animate,
            ElementId::Script,
            ElementId::Set,
            ElementId::Style,
        ],
        expected_attribute_groups: AttributeGroup::Core
            .union(AttributeGroup::GlobalEvent)
            .union(AttributeGroup::DocumentElementEvent)
            .union(AttributeGroup::Presentation),
        expected_attributes: &[AttrId::StopOffset],
    },
    Style {
        name: "style",
        categories: ElementCategory::NeverRendered,
        permitted_categories: ElementCategory::empty(),
        permitted_elements: &[],
        expected_attribute_groups: AttributeGroup::Core
            .union(AttributeGroup::GlobalEvent)
            .union(AttributeGroup::DocumentElementEvent),
        expected_attributes: &[
            AttrId::StyleType,
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
            .union(ElementCategory::Structural),
        permitted_elements: &[
            ElementId::A,
            ElementId::ClipPath,
            ElementId::Filter,
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
        expected_attribute_groups: AttributeGroup::Aria
            .union(AttributeGroup::ConditionalProcessing)
            .union(AttributeGroup::Core)
            .union(AttributeGroup::DocumentEvent)
            .union(AttributeGroup::GlobalEvent)
            .union(AttributeGroup::DocumentElementEvent)
            .union(AttributeGroup::Presentation),
        expected_attributes: &[
            AttrId::ViewBox,
            AttrId::PreserveAspectRatio,
            AttrId::Transform,
            AttrId::X,
            AttrId::Y,
            AttrId::Width,
            AttrId::Height,
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
            .union(AttributeGroup::Core)
            .union(AttributeGroup::GlobalEvent)
            .union(AttributeGroup::DocumentElementEvent)
            .union(AttributeGroup::Presentation),
        expected_attributes: &[],
    },
    Symbol {
        name: "symbol",
        categories: ElementCategory::Container.union(ElementCategory::Structural),
        permitted_categories: ElementCategory::Animation
            .union(ElementCategory::Descriptive)
            .union(ElementCategory::PaintServer)
            .union(ElementCategory::Shape)
            .union(ElementCategory::Structural),
        permitted_elements: &[
            ElementId::A,
            ElementId::ClipPath,
            ElementId::Filter,
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
        expected_attribute_groups: AttributeGroup::Aria
            .union(AttributeGroup::Core)
            .union(AttributeGroup::GlobalEvent)
            .union(AttributeGroup::DocumentElementEvent)
            .union(AttributeGroup::Presentation),
        expected_attributes: &[
            AttrId::PreserveAspectRatio,
            AttrId::ViewBox,
            AttrId::RefX,
            AttrId::RefY,
            AttrId::X,
            AttrId::Y,
            AttrId::Width,
            AttrId::Height,
        ],
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
            .union(AttributeGroup::Core)
            .union(AttributeGroup::GlobalEvent)
            .union(AttributeGroup::DocumentElementEvent)
            .union(AttributeGroup::Presentation),
        expected_attributes: &[
            AttrId::LengthAdjust,
            AttrId::X,
            AttrId::Y,
            AttrId::DX,
            AttrId::DY,
            AttrId::Rotate,
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
            ElementId::Animate,
            ElementId::ClipPath,
            ElementId::Marker,
            ElementId::Mask,
            ElementId::Script,
            ElementId::Set,
            ElementId::Style,
            ElementId::TSpan,
        ],
        expected_attribute_groups: AttributeGroup::Aria
            .union(AttributeGroup::ConditionalProcessing)
            .union(AttributeGroup::Core)
            .union(AttributeGroup::GlobalEvent)
            .union(AttributeGroup::DocumentElementEvent)
            .union(AttributeGroup::Presentation)
            .union(AttributeGroup::XLink),
        expected_attributes: &[
            AttrId::LengthAdjust,
            AttrId::TextLength,
            AttrId::Path,
            AttrId::Href,
            AttrId::StartOffset,
            AttrId::TextPathMethod,
            AttrId::TextPathSpacing,
            AttrId::TextPathSide,
        ],
    },
    Title {
        name: "title",
        categories: ElementCategory::Descriptive.union(ElementCategory::NeverRendered),
        permitted_categories: ElementCategory::all(),
        permitted_elements: &[],
        expected_attribute_groups: AttributeGroup::Core
            .union(AttributeGroup::GlobalEvent)
            .union(AttributeGroup::DocumentElementEvent),
        expected_attributes: &[],
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
            ElementId::Animate,
            ElementId::Script,
            ElementId::Set,
            ElementId::Style,
            ElementId::TSpan,
        ],
        expected_attribute_groups: AttributeGroup::Aria
            .union(AttributeGroup::ConditionalProcessing)
            .union(AttributeGroup::Core)
            .union(AttributeGroup::GlobalEvent)
            .union(AttributeGroup::DocumentElementEvent)
            .union(AttributeGroup::Presentation),
        expected_attributes: &[
            AttrId::X,
            AttrId::Y,
            AttrId::DX,
            AttrId::DY,
            AttrId::Rotate,
            AttrId::TextLength,
            AttrId::LengthAdjust,
        ],
    },
    Use {
        name: "use",
        categories: ElementCategory::Graphics
            .union(ElementCategory::GraphicsReferencing)
            .union(ElementCategory::Structural)
            .union(ElementCategory::StructurallyExternal),
        permitted_categories: ElementCategory::Animation.union(ElementCategory::Descriptive),
        permitted_elements: &[
            ElementId::ClipPath,
            ElementId::Mask,
            ElementId::Script,
            ElementId::Style,
        ],
        expected_attribute_groups: AttributeGroup::Aria
            .union(AttributeGroup::Core)
            .union(AttributeGroup::ConditionalProcessing)
            .union(AttributeGroup::GlobalEvent)
            .union(AttributeGroup::DocumentElementEvent)
            .union(AttributeGroup::Presentation)
            .union(AttributeGroup::XLink),
        expected_attributes: &[
            AttrId::Href,
            AttrId::X,
            AttrId::Y,
            AttrId::Width,
            AttrId::Height,
        ],
    },
    View {
        name: "view",
        categories: ElementCategory::Uncategorised,
        permitted_categories: ElementCategory::Animation.union(ElementCategory::Descriptive),
        permitted_elements: &[ElementId::Script, ElementId::Style],
        expected_attribute_groups: AttributeGroup::Aria
            .union(AttributeGroup::Core)
            .union(AttributeGroup::GlobalEvent)
            .union(AttributeGroup::DocumentElementEvent),
        expected_attributes: &[AttrId::ViewBox, AttrId::PreserveAspectRatio],
    },
}

#[derive(Debug)]
/// An iterator that goes over an element and it's descendants in a breadth-first fashion
pub struct Iterator<'arena> {
    queue: VecDeque<Element<'arena>>,
}

impl<'arena> Iterator<'arena> {
    /// Returns a breadth-first iterator starting at the given element
    pub fn new(element: &Element<'arena>) -> Self {
        let mut queue = VecDeque::new();
        element.child_elements_iter().for_each(|e| {
            queue.push_back(e);
        });

        Self { queue }
    }
}

impl<'arena> std::iter::Iterator for Iterator<'arena> {
    type Item = Element<'arena>;

    fn next(&mut self) -> Option<Self::Item> {
        let current = self.queue.pop_front()?;
        current.child_elements_iter().for_each(|e| {
            self.queue.push_back(e);
        });
        Some(current)
    }
}
