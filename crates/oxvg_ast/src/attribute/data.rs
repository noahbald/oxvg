//! Types of data which can be assigned to an element's attributes
use animation::{BeginEnd, CalcMode, ControlPoint};
use animation_addition::{Accumulate, Additive};
use animation_timing::{Dur, Fill, MinMax, RepeatCount, RepeatDur, Restart};
use aria::{
    AriaAutocomplete, AriaCurrent, AriaDropEffect, AriaHasPopup, AriaInvalid, AriaLive,
    AriaOrientation, AriaRelevant, AriaSort, IDReference, Role, Tristate,
};
use core::{
    Angle, Anything, Boolean, Class, Color, Id, Integer, Length, Number, NumberOptionalNumber,
    Opacity, Paint, Style, TokenList, TransformList, Url,
};
use filter_effect::{
    ChannelSelector, EdgeModeFe, In, OperatorFeComposite, OperatorFeMorphology,
    StitchTilesFeTurbulence, TypeFeColorMatrix, TypeFeTurbulence,
};
use fonts::{ArabicForm, Orientation};
use inheritable::Inheritable;
use lightningcss::{
    properties::{
        font::{AbsoluteFontWeight, FontStretchKeyword},
        Property, PropertyId,
    },
    values::percentage::Percentage,
    vendor_prefix::VendorPrefix,
};
use list_of::{Comma, ListOf, Semicolon, Seperators, Space, SpaceOrComma};
use path::Path;
use presentation::{
    AlignmentBaseline, BaselineShift, Clip, ClipPath, ColorInterpolation, ColorProfile,
    ColorRendering, Cursor, Direction, Display, DominantBaseline, EnableBackground, FillRule,
    FilterList, FontFamily, FontSize, FontStretch, FontStyle, FontVariant, FontWeight,
    GlyphOrientationVertical, ImageRendering, Kerning, LengthOrNumber, LengthPercentage, Marker,
    Mask, Overflow, PointerEvents, ShapeRendering, Spacing, StrokeDasharray, StrokeLinecap,
    StrokeLinejoin, TextAnchor, TextDecoration, TextRendering, UnicodeBidi, Visibility,
    WritingMode,
};
use std::{cell::RefCell, collections::HashMap};
use transfer_function::TransferFunctionType;
use uncategorised::{
    BlendMode, ColorProfileName, CrossOrigin, LengthAdjust, LinkType, MarkerUnits, MediaQueryList,
    MediaType, NumberPercentage, Orient, Origin, PreserveAspectRatio, Radius, RefX, RefY,
    ReferrerPolicy, RenderingIntent, Rotate, SpreadMethod, Target, TextPathMethod, TextPathSpacing,
    TrueFalse, TrueFalseUndefined, TypeAnimateTransform, Units, ViewBox, ZoomAndPan,
};
use xlink::XLinkShow;
use xml::XmlSpace;

use super::content_type::{ContentType, ContentTypeId, ContentTypeRef};
use crate::{
    atom::Atom,
    attribute::{AttributeGroup, AttributeInfo},
    name::{Prefix, QualName},
    parse::Parse,
    serialize::ToAtom,
};

pub mod animation;
pub mod animation_addition;
pub mod animation_timing;
pub mod aria;
pub mod core;
pub mod filter_effect;
pub mod fonts;
pub mod inheritable;
pub mod list_of;
pub mod path;
pub mod presentation;
pub mod transfer_function;
pub mod transform;
pub mod uncategorised;
pub mod xlink;
pub mod xml;

/// An attribute's group.
type C = AttributeGroup;

#[macro_export]
/// Creates an attribute type that consists of enumerable idents
macro_rules! enum_attr {
    (
        $(#[$outer:meta])*
        $attr:ident { $(
            $(#[$meta:meta])*
            $name:ident: $value:literal$(,)?
        )+}
    ) => {
        #[derive(Clone, Debug, PartialEq, Eq)]
        $(#[$outer])*
        pub enum $attr {
            $(
                $(#[$meta])*
                $name,
            )+
        }

        impl<'input> $crate::attribute::data::Parse<'input> for $attr {
            fn parse<'t>(
                input: &mut $crate::parse::Parser<'input, 't>,
            ) -> Result<Self, $crate::error::ParseError<'input>> {
                let location = input.current_source_location();
                let ident = input.expect_ident()?;
                let str: &str = &*ident;
                match str {
                    $($value => Ok($attr::$name),)+
                    _ => Err(location.new_unexpected_token_error(
                        cssparser_lightningcss::Token::Ident(ident.clone())
                    ))
                }
            }
        }

        impl $crate::serialize::ToAtom for $attr {
            fn write_atom<W>(
                &self,
                dest: &mut $crate::serialize::Printer<W>,
            ) -> Result<(), $crate::error::PrinterError>
            where
                W: std::fmt::Write,
            {
                match self {
                    $(Self::$name => dest.write_str($value),)+
                }
            }
        }
    };
}

macro_rules! wrap_content_type {
    ($outer:ident($value:ident)) => {
        ContentType::$outer(ContentTypeRef::Ref($value))
    };
    ($outer:ident<$inner:ident$(, $and:ident)?>($value:ident)) => {
        ContentType::$outer(
            $value
                .map(|x| { Box::new(ContentType::$inner(ContentTypeRef::Ref(x))) })
                $(.map_sep(|_| Seperators::$and))?
        )
    };
}
macro_rules! wrap_content_type_mut {
    ($outer:ident($value:ident)) => {
        ContentType::$outer(ContentTypeRef::RefMut($value))
    };
    ($outer:ident<$inner:ident$(, $and:ident)?>($value:ident)) => {
        ContentType::$outer(
            $value
                .map_mut(|x| { Box::new(ContentType::$inner(ContentTypeRef::RefMut(x))) })
                $(.map_sep(|_| Seperators::$and))?
        )
    };
}
macro_rules! define_attrs {
    ($($attr:ident(
        $outer:ident$(<$outer_lt:lifetime>)?$(<
            $inner:ident$(<$inner_lt:lifetime>)?
            $(, $and:ident)?
        >)?
    ) {
        $(prefix: $prefix:ident,)?
        name: $name:tt,
        $(categories: $categories:expr,)?
        $(info: $info:expr,)?
        $(default: $default:expr,)?
    },)+) => {
        macro_rules! prefix_else {
            ($_prefix:ident) => { Prefix::$_prefix };
            () => { Prefix::SVG };
        }
        macro_rules! categories_else {
            ($_categories:expr) => { $_categories };
            () => { AttributeGroup::empty() };
        }

        #[allow(non_upper_case_globals)]
        mod _c {
            use super::{C, AttributeGroup};
            $(pub const $attr: C = categories_else!($($categories)?);)+
        }
        #[allow(non_upper_case_globals)]
        #[cfg(not(feature = "markup5ever"))]
        mod _qual_name {
            use crate::name::{Prefix, QualName};
            use crate::atom::Atom;
            $(pub const $attr: &'static QualName<'static> = &QualName {
                prefix: prefix_else!($($prefix)?),
                local: Atom::Static($name),
            };)+
        }
        #[allow(non_upper_case_globals)]
        #[cfg(feature = "markup5ever")]
        mod _qual_name {
            use crate::name::{Prefix, QualName};
            use crate::atom::Atom;
            $(pub const $attr: &'static QualName<'static> = &QualName {
                prefix: prefix_else!($($prefix)?),
                local: Atom::Local(xml5ever::local_name!($name)),
            };)+
        }
        #[allow(non_upper_case_globals)]
        mod _local_name {
            use crate::atom::Atom;
            use super::_qual_name;
            $(pub const $attr: &'static Atom<'static> = &_qual_name::$attr.local;)+
        }
        #[allow(non_upper_case_globals)]
        mod _prefix {
            use crate::name::Prefix;
            use super::_qual_name;
            $(pub const $attr: &'static Prefix<'static> = &_qual_name::$attr.prefix;)+
        }
        #[allow(non_upper_case_globals)]
        mod _attr_id {
            use super::AttrId;
            $(pub const $attr: &'static AttrId<'static> = &AttrId::$attr;)+
            pub const all: &'static [&'static AttrId<'static>] = &[
                $($attr,)+
            ];
        }

        #[derive(Eq, Clone, Debug, Hash)]
        /// Identifies one of an element's attributes.
        ///
        /// [MDN | SVG Attribute reference](https://developer.mozilla.org/en-US/docs/Web/SVG/Reference/Attribute)
        pub enum AttrId<'input> {
            $(
                #[doc=concat!("The `", $name, "` attribute")]
                $attr,
            )+
            /// A known attribute aliased by a different prefix
            Aliased {
                /// The prefix assigned to the attribute
                prefix: Prefix<'input>,
                /// The associated attribute
                attr_id: Box<AttrId<'input>>,
            },
            /// An attribute that doesn't match the expected type for a given `ElementId`
            Unknown(QualName<'input>),
        }

        #[derive(Debug, Clone)]
        /// Represents one of an element's attributes.
        ///
        /// [MDN | Attr](https://developer.mozilla.org/en-US/docs/Web/API/Attr)
        pub enum Attr<'input> {
            $(
                #[doc=concat!("The `", $name, "` attribute")]
                $attr($outer$(<$outer_lt>)?$(<
                    $inner$(<$inner_lt>)?
                    $(, $and)?
                >)?),
            )+
            /// A known attribute aliased by a different prefix
            Aliased {
                /// The prefix assigned to the attribute
                prefix: Prefix<'input>,
                /// The associated attribute
                value: Box<Attr<'input>>,
            },
            /// An attribute with an unknown name or invalid value
            Unparsed {
                /// The name of the attribute
                attr_id: AttrId<'input>,
                /// The unparsed string of the attribute
                value: Atom<'input>,
            },
            /// An attribute converted from lightningcss
            CSSUnknown {
                /// An unknown variant of [`AttrId`]
                attr_id: AttrId<'input>,
                /// A parsed list of CSS tokens
                value: TokenList<'input>,
            },
        }

        impl<'input> AttrId<'input> {
            /// Returns the prefix of the qualified name of an attribute.
            pub fn prefix<'a>(&'a self) -> &'a Prefix<'input> {
                match self {
                    $(Self::$attr => _prefix::$attr,)+
                    Self::Aliased { prefix, .. } => prefix,
                    Self::Unknown(QualName { prefix, .. }) => prefix,
                }
            }

            /// Returns the local part of the qualified name of an attribute.
            ///
            /// [MDN | localName](https://developer.mozilla.org/en-US/docs/Web/API/Attr/localName)
            pub fn local_name<'a>(&'a self) -> &'a Atom<'input> {
                match self {
                    $(Self::$attr => _local_name::$attr,)+
                    Self::Aliased { attr_id, .. } => attr_id.local_name(),
                    Self::Unknown(QualName { local, .. }) => local,
                }
            }

            /// Returns the attribute groups this attribute belongs to
            pub fn attribute_group(&self) -> C {
                match self {
                    $(Self::$attr => _c::$attr,)+
                    Self::Aliased { attr_id, .. } => attr_id.attribute_group(),
                    Self::Unknown(_) => AttributeGroup::empty(),
                }
            }

            /// Returns info flags for this attribute
            pub fn info(&self) -> AttributeInfo {
                match self {
                    $($(Self::$attr => $info,)?)+
                    _ => AttributeInfo::empty(),
                }
            }

            /// Returns default value for this attribute
            pub fn default(&self) -> Option<Attr<'input>> {
                match self {
                    $($(Self::$attr => Some(Attr::$attr($default)),)?)+
                    _ => None,
                }
            }

            /// Returns the expected content type for the attribute
            pub fn r#type(&self) -> ContentTypeId {
                match self {
                    $(Self::$attr => ContentTypeId::$outer$((
                        Box::new(ContentTypeId::$inner)
                        $(, Seperators::$and)?
                    ))?,)+
                    Self::Aliased { attr_id, .. } => attr_id.r#type(),
                    Self::Unknown(_) => ContentTypeId::Anything,
                }
            }
        }

        impl<'input> Attr<'input> {
            /// Creates a new attribute
            pub fn new(name: AttrId<'input>, value: &'input str) -> Self {
                match name {
                    $(AttrId::$attr => match $outer::parse_string(value) {
                        Ok(value) => Self::$attr(value),
                        Err(err) => {
                            log::debug!("failed to parse {}: {err:?}", stringify!($attr));
                            Self::Unparsed {
                                attr_id: AttrId::$attr,
                                value: value.into(),
                            }
                        }
                    })+
                    AttrId::Aliased { prefix, attr_id } => Self::Aliased {
                        prefix,
                        value: Box::new(Self::new(*attr_id, value)),
                    },
                    AttrId::Unknown(name) => Self::Unparsed {
                        attr_id: AttrId::Unknown(name),
                        value: value.into(),
                    },
                }
            }

            /// Returns the qualified name of an attribute.
            ///
            /// [MDN | name](https://developer.mozilla.org/en-US/docs/Web/API/Attr/name)
            pub fn name<'a>(&'a self) -> &'a AttrId<'input> {
                match self {
                    $(Self::$attr(_) => _attr_id::$attr,)+
                    Self::Aliased { value, .. } => value.name(),
                    Self::Unparsed { attr_id, .. } => attr_id,
                    Self::CSSUnknown { attr_id, .. } => attr_id,
                }
            }

            /// Returns the local part of the qualified name of an attribute.
            ///
            /// [MDN | localName](https://developer.mozilla.org/en-US/docs/Web/API/Attr/localName)
            pub fn local_name<'a>(&'a self) -> &'a Atom<'input> {
                self.name().local_name()
            }

            /// Returns the namespace prefix of the attribute.
            ///
            /// [MDN | prefix](https://developer.mozilla.org/en-US/docs/Web/API/Attr/prefix)
            pub fn prefix<'a>(&'a self) -> &'a Prefix<'input> {
                match self {
                    Self::Aliased { prefix, .. } => prefix,
                    Self::Unparsed { attr_id, .. } => attr_id.prefix(),
                    _ => self.name().prefix(),
                }
            }

            /// Returns the value of the attribute.
            ///
            /// [MDN | value](https://developer.mozilla.org/en-US/docs/Web/API/Attr/value)
            pub fn value<'a>(&'a self) -> ContentType<'a, 'input> {
                match self {
                    $(Self::$attr(value) => wrap_content_type!($outer$(<$inner$(, $and)?>)?(value)),)+
                    Self::Aliased { value, .. } => value.value(),
                    Self::Unparsed { value, .. } => ContentType::Anything(
                        ContentTypeRef::Ref(value)
                    ),
                    Self::CSSUnknown { value, .. } => ContentType::TokenList(
                        ContentTypeRef::Ref(value)
                    ),
                }
            }

            /// Mutably returns the value of the attribute.
            ///
            /// [MDN | value](https://developer.mozilla.org/en-US/docs/Web/API/Attr/value)
            pub fn value_mut<'a>(&'a mut self) -> ContentType<'a, 'input> {
                match self {
                    $(Self::$attr(value) => wrap_content_type_mut!($outer$(<$inner$(, $and)?>)?(value)),)+
                    Self::Aliased { value, .. } => value.value_mut(),
                    Self::Unparsed { value, .. } => ContentType::Anything(
                        ContentTypeRef::RefMut(value)
                    ),
                    Self::CSSUnknown { value, .. } => ContentType::TokenList(
                        ContentTypeRef::RefMut(value)
                    ),
                }
            }
        }

        impl ToAtom for Attr<'_> {
            fn write_atom<W>(&self, dest: &mut crate::serialize::Printer<W>) -> Result<(), crate::error::PrinterError>
                where
                    W: std::fmt::Write {
                match self {
                    $(Self::$attr(value) => value.write_atom(dest),)+
                    Self::Aliased { value, .. } => value.write_atom(dest),
                    Self::Unparsed { value, .. } => value.write_atom(dest),
                    Self::CSSUnknown { value, .. } => value.write_atom(dest),
                }
            }
        }

        impl PartialEq for AttrId<'_> {
            fn eq(&self, other: &Self) -> bool {
                match (self.unaliased(), other.unaliased()) {
                    $((Self::$attr, Self::$attr) => true,)+
                    (Self::Aliased { .. }, _) => unreachable!(),
                    (_, Self::Aliased { .. }) => unreachable!(),
                    (Self::Unknown(a), Self::Unknown(b)) => a == b,
                    _ => false,
                }
            }
        }

        impl PartialEq for Attr<'_> {
            fn eq(&self, other: &Self) -> bool {
                match (self.unaliased(), other.unaliased()) {
                    $((Self::$attr(a), Self::$attr(b)) => a == b,)+
                    (Self::Aliased { .. }, _) => unreachable!(),
                    (_, Self::Aliased { .. }) => unreachable!(),
                    (Self::Unparsed { attr_id, value }, Self::Unparsed { attr_id: attr_id_b, value: value_b }) => attr_id == attr_id_b && value == value_b,
                    _ => false,
                }
            }
        }
    };
}

define_attrs! {
    AccentHeight(Number) {
        name: "accent-height",
        info: AttributeInfo::DeprecatedUnsafe,
    },
    Accumulate(Accumulate) {
        name: "accumulate",
        categories: AttributeGroup::AnimationAddition,
    },
    Additive(Additive) {
        name: "additive",
        categories: AttributeGroup::AnimationAddition,
    },
    AlignmentBaseline(Inheritable<AlignmentBaseline>) {
        name: "alignment-baseline",
        categories: AttributeGroup::Presentation,
    },
    Alphabetic(Number) {
        name: "alphabetic",
        info: AttributeInfo::DeprecatedUnsafe,
    },
    Amplitude(Number) {
        name: "amplitude",
        categories: AttributeGroup::TransferFunction,
    },
    ArabicForm(ArabicForm) {
        name: "arabic-form",
        info: AttributeInfo::DeprecatedUnsafe,
        default: ArabicForm::Initial,
    },
    AriaActiveDescendant(IDReference<'input>) {
        name: "aria-activedescendant",
        categories: AttributeGroup::Aria,
    },
    AriaAtomic(TrueFalse) {
        name: "aria-atomic",
        categories: AttributeGroup::Aria,
    },
    AriaAutocomplete(AriaAutocomplete) {
        name: "aria-autocomplete",
        categories: AttributeGroup::Aria,
    },
    AriaBusy(TrueFalse) {
        name: "aria-busy",
        categories: AttributeGroup::Aria,
    },
    AriaChecked(Tristate) {
        name: "aria-checked",
        categories: AttributeGroup::Aria,
    },
    AriaColCount(Integer) {
        name: "aria-colcount",
        categories: AttributeGroup::Aria,
    },
    AriaColIndex(Integer) {
        name: "aria-colindex",
        categories: AttributeGroup::Aria,
    },
    AriaColSpan(Integer) {
        name: "aria-colspan",
        categories: AttributeGroup::Aria,
    },
    AriaControls(ListOf<IDReference<'input>, Space>) {
        name: "aria-controls",
        categories: AttributeGroup::Aria,
    },
    AriaCurrent(AriaCurrent) {
        name: "aria-current",
        categories: AttributeGroup::Aria,
    },
    AriaDescribedBy(ListOf<IDReference<'input>, Space>) {
        name: "aria-describedby",
        categories: AttributeGroup::Aria,
    },
    AriaDetails(IDReference<'input>) {
        name: "aria-details",
        categories: AttributeGroup::Aria,
    },
    AriaDisabled(TrueFalse) {
        name: "aria-disabled",
        categories: AttributeGroup::Aria,
    },
    AriaDropEffect(ListOf<AriaDropEffect, Space>) {
        name: "aria-dropeffect",
        categories: AttributeGroup::Aria,
    },
    AriaErrorMessage(IDReference<'input>) {
        name: "aria-errormessage",
        categories: AttributeGroup::Aria,
    },
    AriaExpanded(TrueFalseUndefined) {
        name: "aria-expanded",
        categories: AttributeGroup::Aria,
    },
    AriaFlowTo(ListOf<IDReference<'input>, Space>) {
        name: "aria-flowto",
        categories: AttributeGroup::Aria,
    },
    // TODO: Add when atoms included in xml5ever::LocalNameStaticSet
    // AriaGrabbed(TrueFalseUndefined) {
    //     name: "aria-grabbed",
    //     categories: AttributeGroup::Aria,
    // },
    AriaHasPopup(AriaHasPopup) {
        name: "aria-haspopup",
        categories: AttributeGroup::Aria,
    },
    AriaHidden(TrueFalseUndefined) {
        name: "aria-hidden",
        categories: AttributeGroup::Aria,
    },
    AriaInvalid(AriaInvalid) {
        name: "aria-invalid",
        categories: AttributeGroup::Aria,
    },
    AriaKeyshortcuts(Anything<'input>) {
        name: "aria-keyshortcuts",
        categories: AttributeGroup::Aria,
    },
    AriaLabel(Anything<'input>) {
        name: "aria-label",
        categories: AttributeGroup::Aria,
    },
    AriaLabelledBy(ListOf<IDReference<'input>, Space>) {
        name: "aria-labelledby",
        categories: AttributeGroup::Aria,
    },
    AriaLevel(Integer) {
        name: "aria-level",
        categories: AttributeGroup::Aria,
    },
    AriaLive(AriaLive) {
        name: "aria-live",
        categories: AttributeGroup::Aria,
    },
    AriaModal(TrueFalse) {
        name: "aria-modal",
        categories: AttributeGroup::Aria,
    },
    AriaMultiline(TrueFalse) {
        name: "aria-multiline",
        categories: AttributeGroup::Aria,
    },
    AriaMultiselectable(TrueFalse) {
        name: "aria-multiselectable",
        categories: AttributeGroup::Aria,
    },
    AriaOrientation(AriaOrientation) {
        name: "aria-orientation",
        categories: AttributeGroup::Aria,
    },
    AriaOwns(ListOf<IDReference<'input>, Space>) {
        name: "aria-owns",
        categories: AttributeGroup::Aria,
    },
    AriaPlaceholder(Anything<'input>) {
        name: "aria-placeholder",
        categories: AttributeGroup::Aria,
    },
    AriaPosInSet(Integer) {
        name: "aria-posinset",
        categories: AttributeGroup::Aria,
    },
    AriaPressed(Tristate) {
        name: "aria-pressed",
        categories: AttributeGroup::Aria,
    },
    AriaReadonly(TrueFalse) {
        name: "aria-readonly",
        categories: AttributeGroup::Aria,
    },
    AriaRelevant(ListOf<AriaRelevant, Space>) {
        name: "aria-relevant",
        categories: AttributeGroup::Aria,
    },
    AriaRequired(TrueFalse) {
        name: "aria-required",
        categories: AttributeGroup::Aria,
    },
    AriaRoleDescription(Anything<'input>) {
        name: "aria-roledescription",
        categories: AttributeGroup::Aria,
    },
    AriaRowCount(Integer) {
        name: "aria-rowcount",
        categories: AttributeGroup::Aria,
    },
    AriaRowIndex(Integer) {
        name: "aria-rowindex",
        categories: AttributeGroup::Aria,
    },
    AriaRowSpan(Integer) {
        name: "aria-rowspan",
        categories: AttributeGroup::Aria,
    },
    AriaSelected(TrueFalseUndefined) {
        name: "aria-selected",
        categories: AttributeGroup::Aria,
    },
    AriaSetSize(Integer) {
        name: "aria-setsize",
        categories: AttributeGroup::Aria,
    },
    AriaSort(AriaSort) {
        name: "aria-sort",
        categories: AttributeGroup::Aria,
    },
    AriaValueMax(Number) {
        name: "aria-valuemax",
        categories: AttributeGroup::Aria,
    },
    AriaValueMin(Number) {
        name: "aria-valuemin",
        categories: AttributeGroup::Aria,
    },
    AriaValueNow(Number) {
        name: "aria-valuenow",
        categories: AttributeGroup::Aria,
    },
    AriaValueText(Anything<'input>) {
        name: "aria-valuetext",
        categories: AttributeGroup::Aria,
    },
    Ascent(Number) {
        name: "ascent",
        info: AttributeInfo::DeprecatedUnsafe,
    },
    AttributeName(Anything<'input>) {
        name: "attributeName",
        categories: AttributeGroup::AnimationAttributeTarget,
    },
    AutoFocus(Boolean<'input>) {
        name: "autofocus",
        categories: AttributeGroup::Core,
    },
    AzimuthFeDistantLight(Number) {
        name: "azimuth",
        default: 0.0,
    },
    BaseFrequencyFeTurbulence(NumberOptionalNumber) {
        name: "baseFrequency",
        default: NumberOptionalNumber(0.0, None),
    },
    BaseProfile(Anything<'input>) {
        name: "baseProfile",
        info: AttributeInfo::DeprecatedUnsafe,
        default: Atom::Static("none"),
    },
    BaselineShift(Inheritable<BaselineShift>) {
        name: "baseline-shift",
        categories: AttributeGroup::Presentation,
    },
    Bbox(Anything<'input>) {
        name: "bbox",
        info: AttributeInfo::DeprecatedUnsafe,
    },
    Begin(ListOf<BeginEnd<'input>, Semicolon>) {
        name: "begin",
        categories: AttributeGroup::AnimationTiming,
    },
    BiasFeConvolveMatrix(Number) {
        name: "bias",
        default: 0.0,
    },
    By(Anything<'input>) {
        name: "by",
        categories: AttributeGroup::AnimationValue,
    },
    CX(LengthPercentage) {
        name: "cx",
        default: LengthPercentage::px(0.0),
    },
    CXRadialGradient(LengthPercentage) {
        name: "cx",
        default: LengthPercentage::px(50.0),
    },
    CY(LengthPercentage) {
        name: "cy",
        default: LengthPercentage::px(0.0),
    },
    CYRadialGradient(LengthPercentage) {
        name: "cy",
        default: LengthPercentage::px(50.0),
    },
    CalcMode(CalcMode) {
        name: "calcMode",
        categories: AttributeGroup::AnimationValue,
    },
    CapHeight(Number) {
        name: "cap-height",
        info: AttributeInfo::DeprecatedUnsafe,
    },
    Class(ListOf<Class<'input>, Space>) {
        name: "class",
        categories: AttributeGroup::Core,
    },
    Clip(Inheritable<Clip>) {
        name: "clip",
        categories: AttributeGroup::Presentation,
    },
    ClipPath(Inheritable<ClipPath<'input> >) {
        name: "clip-path",
        categories: AttributeGroup::Presentation,
    },
    ClipPathUnits(Units) {
        name: "clipPathUnits",
        default: Units::default(),
    },
    ClipRule(Inheritable<FillRule>) {
        name: "clip-rule",
        categories: AttributeGroup::Presentation,
        info: AttributeInfo::Inheritable,
    },
    Color(Inheritable<Color>) {
        name: "color",
        categories: AttributeGroup::Presentation,
        info: AttributeInfo::Inheritable,
    },
    ColorInterpolation(Inheritable<ColorInterpolation>) {
        name: "color-interpolation",
        categories: AttributeGroup::Presentation,
        info: AttributeInfo::Inheritable,
    },
    ColorInterpolationFilters(Inheritable<ColorInterpolation>) {
        name: "color-interpolation-filters",
        categories: AttributeGroup::Presentation,
        info: AttributeInfo::Inheritable,
    },
    ColorProfile(Inheritable<ColorProfile<'input> >) {
        name: "color-profile",
        categories: AttributeGroup::Presentation,
        info: AttributeInfo::Inheritable,
    },
    ColorRendering(Inheritable<ColorRendering>) {
        name: "color-rendering",
        categories: AttributeGroup::Presentation,
        info: AttributeInfo::Inheritable,
    },
    ContentScriptType(Anything<'input>) {
        name: "contentScriptType",
        info: AttributeInfo::Inheritable,
        default: Anything::Static("application/ecmascript"),
    },
    ContentStyleType(Anything<'input>) {
        name: "contentStyleType",
        info: AttributeInfo::Inheritable,
        default: Anything::Static("text/css"),
    },
    CrossOrigin(CrossOrigin) {
        name: "crossorigin",
    },
    Cursor(Inheritable<Cursor<'input> >) {
        name: "cursor",
        categories: AttributeGroup::Presentation,
        info: AttributeInfo::Inheritable,
    },
    D(Path) {
        name: "d",
    },
    DX(Length) {
        name: "dx",
    },
    DxFe(Number) {
        name: "dx",
        default: 0.0,
    },
    DY(Length) {
        name: "dy",
    },
    DyFe(Number) {
        name: "dy",
        default: 0.0,
    },
    Descent(Number) {
        name: "descent",
        info: AttributeInfo::DeprecatedUnsafe,
    },
    DiffuseConstantFeDiffuseLighting(Number) {
        name: "diffuseConstant",
        default: 1.0,
    },
    Direction(Inheritable<Direction>) {
        name: "direction",
        categories: AttributeGroup::Presentation,
        info: AttributeInfo::Inheritable,
    },
    Display(Inheritable<Display>) {
        name: "display",
        categories: AttributeGroup::Presentation,
    },
    DivisorFeConvolveMatrix(Number) {
        name: "divisor",
    },
    DominantBaseline(Inheritable<DominantBaseline>) {
        name: "dominant-baseline",
        categories: AttributeGroup::Presentation,
        info: AttributeInfo::Inheritable,
    },
    Download(Anything<'input>) {
        name: "download",
    },
    Dur(Dur) {
        name: "dur",
        categories: AttributeGroup::AnimationTiming,
    },
    EdgeModeFe(EdgeModeFe) {
        name: "edgeMode",
        default: EdgeModeFe::default(),
    },
    ElevationFeDistantLight(Number) {
        name: "elevation",
        default: 0.0,
    },
    EnableBackground(Inheritable<EnableBackground>) {
        name: "enable-background",
        categories: AttributeGroup::Presentation,
    },
    End(ListOf<BeginEnd<'input>, Semicolon>) {
        name: "end",
        categories: AttributeGroup::AnimationTiming,
    },
    Exponent(Number) {
        name: "exponent",
        categories: AttributeGroup::TransferFunction,
    },
    ExternalResourcesRequired(TrueFalse) {
        name: "externalResourcesRequired",
    },
    Fill(Paint<'input>) {
        name: "fill",
        categories: AttributeGroup::Presentation,
        info: AttributeInfo::Inheritable,
    },
    FillOpacity(Inheritable<Opacity>) {
        name: "fill-opacity",
        categories: AttributeGroup::Presentation,
        info: AttributeInfo::Inheritable,
    },
    FillRule(Inheritable<FillRule>) {
        name: "fill-rule",
        categories: AttributeGroup::Presentation,
        info: AttributeInfo::Inheritable,
    },
    FillTiming(Fill) {
        name: "fill",
        categories: AttributeGroup::AnimationTiming,
    },
    Filter(Inheritable<FilterList<'input> >) {
        name: "filter",
        categories: AttributeGroup::Presentation,
    },
    FilterRes(NumberOptionalNumber) {
        name: "filterRes",
        info: AttributeInfo::DeprecatedUnsafe,
    },
    FilterUnits(Units) {
        name: "filterUnits",
    },
    FloodColor(Inheritable<Color>) {
        name: "flood-color",
        categories: AttributeGroup::Presentation,
    },
    FloodOpacity(Inheritable<Opacity>) {
        name: "flood-opacity",
        categories: AttributeGroup::Presentation,
    },
    Font(Anything<'input>) {
        // NOTE: This isn't in the spec but is referenced by SVGO
        name: "font",
        info: AttributeInfo::Inheritable,
    },
    FontFamily(Inheritable<FontFamily<'input> >) {
        name: "font-family",
        categories: AttributeGroup::Presentation,
        info: AttributeInfo::Inheritable,
    },
    FontSize(Inheritable<FontSize>) {
        name: "font-size",
        categories: AttributeGroup::Presentation,
        info: AttributeInfo::Inheritable,
    },
    FontSizeAdjust(Inheritable<Number>) {
        name: "font-size-adjust",
        categories: AttributeGroup::Presentation,
        info: AttributeInfo::Inheritable,
    },
    FontStretch(Inheritable<FontStretch>) {
        name: "font-stretch",
        categories: AttributeGroup::Presentation,
        info: AttributeInfo::Inheritable,
        default: Inheritable::Defined(FontStretch::Keyword(FontStretchKeyword::Normal)),
    },
    FontStyle(Inheritable<FontStyle>) {
        name: "font-style",
        categories: AttributeGroup::Presentation,
        info: AttributeInfo::Inheritable,
        // NOTE: SVGO uses `all` but this is different to the spec
        default: Inheritable::Defined(FontStyle::Normal),
    },
    FontVariant(Inheritable<FontVariant>) {
        name: "font-variant",
        categories: AttributeGroup::Presentation,
        info: AttributeInfo::Inheritable,
        default: Inheritable::Defined(FontVariant::Normal),
    },
    FontWeight(Inheritable<FontWeight>) {
        name: "font-weight",
        categories: AttributeGroup::Presentation,
        info: AttributeInfo::Inheritable,
        // NOTE: SVGO uses `all` but this is different to the spec
        default: Inheritable::Defined(FontWeight::Absolute(AbsoluteFontWeight::Normal)),
    },
    Format(Anything<'input>) {
        name: "format",
    },
    PreserveAspectRatio(PreserveAspectRatio) {
        name: "preserveAspectRatio",
        default: PreserveAspectRatio::default(),
    },
    // TODO: Add when atoms included in xml5ever::LocalNameStaticSet
    // FR(Length) {
    //     name: "fr",
    // },
    FX(Length) {
        name: "fx",
    },
    FY(Length) {
        name: "fy",
    },
    From(Anything<'input>) {
        name: "from",
        categories: AttributeGroup::AnimationValue,
    },
    G1(Anything<'input>) {
        name: "g1",
        info: AttributeInfo::DeprecatedUnsafe,
    },
    G2(Anything<'input>) {
        name: "g2",
        info: AttributeInfo::DeprecatedUnsafe,
    },
    GlyphName(Anything<'input>) {
        name: "glyph-name",
        info: AttributeInfo::DeprecatedUnsafe,
    },
    GlyphOrientationHorizontal(Inheritable<Angle>) {
        name: "glyph-orientation-horizontal",
        categories: AttributeGroup::Presentation,
        info: AttributeInfo::Inheritable,
    },
    GlyphOrientationVertical(Inheritable<GlyphOrientationVertical>) {
        name: "glyph-orientation-vertical",
        categories: AttributeGroup::Presentation,
        info: AttributeInfo::Inheritable,
    },
    GlyphRef(Anything<'input>) {
        name: "glyphRef",
    },
    GradientTransform(TransformList) {
        name: "gradientTransform",
    },
    GradientUnits(Units) {
        name: "gradientUnits",
        default: Units::ObjectBoundingBox,
    },
    Hanging(Number) {
        name: "hanging",
        info: AttributeInfo::DeprecatedUnsafe,
    },
    // TODO: Add when atoms included in xml5ever::LocalNameStaticSet
    // HatchContentUnits(Units) {
    //     name: "hatchContentUnits",
    //     default: Units::UserSpaceOnUse,
    // },
    // TODO: Add when atoms included in xml5ever::LocalNameStaticSet
    // HatchUnits(Units) {
    //     name: "hatchUnits",
    //     default: Units::ObjectBoundingBox,
    // },
    Height(LengthPercentage) {
        name: "height",
        categories: AttributeGroup::FilterPrimitive,
    },
    HeightFilter(LengthPercentage) {
        name: "height",
        default: LengthPercentage::Percentage(Percentage(120.0)),
    },
    HeightMask(LengthPercentage) {
        name: "height",
        default: LengthPercentage::Percentage(Percentage(120.0)),
    },
    HeightPattern(LengthPercentage) {
        name: "height",
        default: LengthPercentage::Percentage(Percentage(120.0)),
    },
    HeightSvg(LengthPercentage) {
        name: "height",
        default: LengthPercentage::Percentage(Percentage(100.0)),
    },
    HorizAdvX(Number) {
        name: "horiz-adv-x",
        info: AttributeInfo::DeprecatedUnsafe,
    },
    HorizOriginX(Number) {
        name: "horiz-origin-x",
        info: AttributeInfo::DeprecatedUnsafe,
        default: 0.0,
    },
    HorizOriginY(Number) {
        name: "horiz-origin-y",
        info: AttributeInfo::DeprecatedUnsafe,
        default: 0.0,
    },
    Href(Url<'input>) {
        name: "href",
        categories: AttributeGroup::AnimationTargetElement,
    },
    Hreflang(Anything<'input>) {
        name: "hreflang",
    },
    Id(Id<'input>) {
        name: "id",
        categories: AttributeGroup::Core,
    },
    Ideographic(Number) {
        name: "ideographic",
        info: AttributeInfo::DeprecatedUnsafe,
    },
    ImageRendering(Inheritable<ImageRendering>) {
        name: "image-rendering",
        categories: AttributeGroup::Presentation,
        info: AttributeInfo::Inheritable,
    },
    In(In<'input>) {
        name: "in",
    },
    In2(In<'input>) {
        name: "in2",
    },
    Intercept(Number) {
        name: "intercept",
        categories: AttributeGroup::TransferFunction,
    },
    K(Number) {
        name: "k",
        info: AttributeInfo::DeprecatedUnsafe,
    },
    K1FeComposite(Number) {
        name: "k1",
        default: 0.0,
    },
    K2FeComposite(Number) {
        name: "k2",
        default: 0.0,
    },
    K3FeComposite(Number) {
        name: "k3",
        default: 0.0,
    },
    K4FeComposite(Number) {
        name: "k4",
        default: 0.0,
    },
    KernalMatrixFeConvolveMatrix(ListOf<Number, SpaceOrComma>) {
        name: "kernelMatrix",
    },
    KernelUnitLengthFe(NumberOptionalNumber) {
        name: "kernelUnitLength",
    },
    Kerning(Inheritable<Kerning>) {
        name: "kerning",
        categories: AttributeGroup::Presentation,
    },
    KeyPoints(ListOf<Number, Semicolon>) {
        name: "keyPoints",
    },
    KeySplines(ListOf<ControlPoint, Semicolon>) {
        name: "keySplines",
        categories: AttributeGroup::AnimationValue,
    },
    KeyTimes(ListOf<Number, Semicolon>) {
        name: "keyTimes",
        categories: AttributeGroup::AnimationValue,
    },
    Lang(Anything<'input>) {
        name: "lang",
        categories: AttributeGroup::Core,
    },
    LengthAdjust(LengthAdjust) {
        name: "lengthAdjust",
        default: LengthAdjust::Spacing,
    },
    LetterSpacing(Inheritable<Spacing>) {
        name: "letter-spacing",
        categories: AttributeGroup::Presentation,
        info: AttributeInfo::Inheritable,
    },
    LightingColor(Inheritable<Color>) {
        name: "lighting-color",
        categories: AttributeGroup::Presentation,
    },
    LimitingConeAngleFeSpotLight(Number) {
        name: "limitingConeAngle",
    },
    Local(Anything<'input>) {
        name: "local",
    },
    Marker(Inheritable<Marker<'input> >) {
        name: "marker",
        categories: AttributeGroup::Presentation,
        info: AttributeInfo::Inheritable,
    },
    MarkerEnd(Inheritable<Marker<'input> >) {
        name: "marker-end",
        categories: AttributeGroup::Presentation,
        info: AttributeInfo::Inheritable,
    },
    MarkerHeight(LengthOrNumber) {
        name: "markerHeight",
        default: LengthOrNumber::Number(3.0),
    },
    MarkerMid(Inheritable<Marker<'input> >) {
        name: "marker-mid",
        categories: AttributeGroup::Presentation,
        info: AttributeInfo::Inheritable,
    },
    MarkerStart(Inheritable<Marker<'input> >) {
        name: "marker-start",
        categories: AttributeGroup::Presentation,
        info: AttributeInfo::Inheritable,
    },
    MarkerUnits(MarkerUnits) {
        name: "markerUnits",
        default: MarkerUnits::StrokeWidth,
    },
    MarkerWidth(LengthOrNumber) {
        name: "markerWidth",
        default: LengthOrNumber::Number(3.0),
    },
    Mask(Inheritable<Mask<'input> >) {
        name: "mask",
        categories: AttributeGroup::Presentation,
    },
    MaskContentUnits(Units) {
        name: "maskContentUnits",
        default: Units::UserSpaceOnUse,
    },
    MaskUnits(Units) {
        name: "maskUnits",
        default: Units::ObjectBoundingBox,
    },
    Mathematical(Number) {
        name: "mathematical",
        info: AttributeInfo::DeprecatedUnsafe,
    },
    Max(MinMax) {
        name: "max",
        categories: AttributeGroup::AnimationTiming,
    },
    Media(MediaQueryList<'input>) {
        name: "media",
    },
    TextPathMethod(TextPathMethod) {
        name: "method",
        default: TextPathMethod::Align,
    },
    Min(MinMax) {
        name: "min",
        categories: AttributeGroup::AnimationTiming,
    },
    Mode(BlendMode) {
        name: "mode",
        default: BlendMode::default(),
    },
    Name(Anything<'input>) {
        name: "name",
        info: AttributeInfo::DeprecatedUnsafe,
    },
    NumOctavesFeTurbulence(Integer) {
        name: "numOctaves",
        default: 1,
    },
    ColorProfileName(ColorProfileName<'input>) {
        name: "name",
        info: AttributeInfo::DeprecatedUnsafe,
        default: ColorProfileName::default(),
    },
    Offset(Number) {
        name: "offset",
        categories: AttributeGroup::TransferFunction,
    },
    OffsetStop(NumberPercentage) {
        name: "offset",
    },
    OnBegin(BeginEnd<'input>) {
        name: "onbegin",
        categories: AttributeGroup::AnimationEvent,
    },
    // TODO: Add when atoms included in xml5ever::LocalNameStaticSet
    // OnCancel(Anything<'input>) {
    //     name: "oncancel",
    //     categories: AttributeGroup::GlobalEvent,
    // },
    // TODO: Add when atoms included in xml5ever::LocalNameStaticSet
    // OnCanplay(Anything<'input>) {
    //     name: "oncanplay",
    //     categories: AttributeGroup::GlobalEvent,
    // },
    // TODO: Add when atoms included in xml5ever::LocalNameStaticSet
    // OnCanplaythrough(Anything<'input>) {
    //     name: "oncanplaythrough",
    //     categories: AttributeGroup::GlobalEvent,
    // },
    OnChange(Anything<'input>) {
        name: "onchange",
        categories: AttributeGroup::GlobalEvent,
    },
    OnClick(Anything<'input>) {
        name: "onclick",
        categories: AttributeGroup::GlobalEvent,
    },
    // TODO: Add when atoms included in xml5ever::LocalNameStaticSet
    // OnClose(Anything<'input>) {
    //     name: "onclose",
    //     categories: AttributeGroup::GlobalEvent,
    // },
    OnCopy(Anything<'input>) {
        name: "oncopy",
        categories: AttributeGroup::DocumentElementEvent,
    },
    // TODO: Add when atoms included in xml5ever::LocalNameStaticSet
    // OnCuechange(Anything<'input>) {
    //     name: "oncuechange",
    //     categories: AttributeGroup::GlobalEvent,
    // },
    OnCut(Anything<'input>) {
        name: "oncut",
        categories: AttributeGroup::DocumentElementEvent,
    },
    OnDblclick(Anything<'input>) {
        name: "ondblclick",
        categories: AttributeGroup::GlobalEvent,
    },
    OnDrag(Anything<'input>) {
        name: "ondrag",
        categories: AttributeGroup::GlobalEvent,
    },
    OnDragend(Anything<'input>) {
        name: "ondragend",
        categories: AttributeGroup::GlobalEvent,
    },
    OnDragenter(Anything<'input>) {
        name: "ondragenter",
        categories: AttributeGroup::GlobalEvent,
    },
    // TODO: Add when atoms included in xml5ever::LocalNameStaticSet
    // OnDragexit(Anything<'input>) {
    //     name: "ondragexit",
    //     categories: AttributeGroup::GlobalEvent,
    // },
    OnDragleave(Anything<'input>) {
        name: "ondragleave",
        categories: AttributeGroup::GlobalEvent,
    },
    OnDragover(Anything<'input>) {
        name: "ondragover",
        categories: AttributeGroup::GlobalEvent,
    },
    OnDragstart(Anything<'input>) {
        name: "ondragstart",
        categories: AttributeGroup::GlobalEvent,
    },
    OnDrop(Anything<'input>) {
        name: "ondrop",
        categories: AttributeGroup::GlobalEvent,
    },
    // TODO: Add when atoms included in xml5ever::LocalNameStaticSet
    // OnDurationchange(Anything<'input>) {
    //     name: "ondurationchange",
    //     categories: AttributeGroup::GlobalEvent,
    // },
    // TODO: Add when atoms included in xml5ever::LocalNameStaticSet
    // OnEmptied(Anything<'input>) {
    //     name: "onemptied",
    //     categories: AttributeGroup::GlobalEvent,
    // },
    OnEnd(Anything<'input>) {
        name: "onend",
        categories: AttributeGroup::AnimationEvent,
    },
    // TODO: Add when atoms included in xml5ever::LocalNameStaticSet
    // OnEnded(Anything<'input>) {
    //     name: "onended",
    //     categories: AttributeGroup::GlobalEvent,
    // },
    OnError(Anything<'input>) {
        name: "onerror",
        categories: AttributeGroup::GlobalEvent,
    },
    OnFocus(Anything<'input>) {
        name: "onfocus",
        categories: AttributeGroup::GlobalEvent,
    },
    OnInput(Anything<'input>) {
        name: "oninput",
        categories: AttributeGroup::GlobalEvent,
    },
    OnInvalid(Anything<'input>) {
        name: "oninvalid",
        categories: AttributeGroup::GlobalEvent,
    },
    OnKeydown(Anything<'input>) {
        name: "onkeydown",
        categories: AttributeGroup::GlobalEvent,
    },
    OnKeypress(Anything<'input>) {
        name: "onkeypress",
        categories: AttributeGroup::GlobalEvent,
    },
    OnKeyup(Anything<'input>) {
        name: "onkeyup",
        categories: AttributeGroup::GlobalEvent,
    },
    OnLoad(Anything<'input>) {
        name: "onload",
        categories: AttributeGroup::GlobalEvent,
    },
    // TODO: Add when atoms included in xml5ever::LocalNameStaticSet
    // OnLoadeddata(Anything<'input>) {
    //     name: "onloadeddata",
    //     categories: AttributeGroup::GlobalEvent,
    // },
    // TODO: Add when atoms included in xml5ever::LocalNameStaticSet
    // OnLoadedmetadata(Anything<'input>) {
    //     name: "onloadedmetadata",
    //     categories: AttributeGroup::GlobalEvent,
    // },
    // TODO: Add when atoms included in xml5ever::LocalNameStaticSet
    // OnLoadstart(Anything<'input>) {
    //     name: "onloadstart",
    //     categories: AttributeGroup::GlobalEvent,
    // },
    OnMousedown(Anything<'input>) {
        name: "onmousedown",
        categories: AttributeGroup::GlobalEvent,
    },
    OnMouseenter(Anything<'input>) {
        name: "onmouseenter",
        categories: AttributeGroup::GlobalEvent,
    },
    OnMouseleave(Anything<'input>) {
        name: "onmouseleave",
        categories: AttributeGroup::GlobalEvent,
    },
    OnMousemove(Anything<'input>) {
        name: "onmousemove",
        categories: AttributeGroup::GlobalEvent,
    },
    OnMouseout(Anything<'input>) {
        name: "onmouseout",
        categories: AttributeGroup::GlobalEvent,
    },
    OnMouseover(Anything<'input>) {
        name: "onmouseover",
        categories: AttributeGroup::GlobalEvent,
    },
    OnMouseup(Anything<'input>) {
        name: "onmouseup",
        categories: AttributeGroup::GlobalEvent,
    },
    OnPaste(Anything<'input>) {
        name: "onpaste",
        categories: AttributeGroup::DocumentElementEvent,
    },
    // TODO: Add when atoms included in xml5ever::LocalNameStaticSet
    // OnPause(Anything<'input>) {
    //     name: "onpause",
    //     categories: AttributeGroup::GlobalEvent,
    // },
    // TODO: Add when atoms included in xml5ever::LocalNameStaticSet
    // OnPlay(Anything<'input>) {
    //     name: "onplay",
    //     categories: AttributeGroup::GlobalEvent,
    // },
    // TODO: Add when atoms included in xml5ever::LocalNameStaticSet
    // OnPlaying(Anything<'input>) {
    //     name: "onplaying",
    //     categories: AttributeGroup::GlobalEvent,
    // },
    // TODO: Add when atoms included in xml5ever::LocalNameStaticSet
    // OnProgress(Anything<'input>) {
    //     name: "onprogress",
    //     categories: AttributeGroup::GlobalEvent,
    // },
    // TODO: Add when atoms included in xml5ever::LocalNameStaticSet
    // OnRatechange(Anything<'input>) {
    //     name: "onratechange",
    //     categories: AttributeGroup::GlobalEvent,
    // },
    OnRepeat(Anything<'input>) {
        name: "onrepeat",
        categories: AttributeGroup::AnimationEvent,
    },
    OnReset(Anything<'input>) {
        name: "onreset",
        categories: AttributeGroup::GlobalEvent,
    },
    OnResize(Anything<'input>) {
        name: "onresize",
        categories: AttributeGroup::GlobalEvent,
    },
    OnScroll(Anything<'input>) {
        name: "onscroll",
        categories: AttributeGroup::GlobalEvent,
    },
    // TODO: Add when atoms included in xml5ever::LocalNameStaticSet
    // OnSeeked(Anything<'input>) {
    //     name: "onseeked",
    //     categories: AttributeGroup::GlobalEvent,
    // },
    // TODO: Add when atoms included in xml5ever::LocalNameStaticSet
    // OnSeeking(Anything<'input>) {
    //     name: "onseeking",
    //     categories: AttributeGroup::GlobalEvent,
    // },
    OnSelect(Anything<'input>) {
        name: "onselect",
        categories: AttributeGroup::GlobalEvent,
    },
    // TODO: Add when atoms included in xml5ever::LocalNameStaticSet
    // OnShow(Anything<'input>) {
    //     name: "onshow",
    //     categories: AttributeGroup::GlobalEvent,
    // },
    // TODO: Add when atoms included in xml5ever::LocalNameStaticSet
    // OnStalled(Anything<'input>) {
    //     name: "onstalled",
    //     categories: AttributeGroup::GlobalEvent,
    // },
    OnSubmit(Anything<'input>) {
        name: "onsubmit",
        categories: AttributeGroup::GlobalEvent,
    },
    // TODO: Add when atoms included in xml5ever::LocalNameStaticSet
    // OnSuspend(Anything<'input>) {
    //     name: "onsuspend",
    //     categories: AttributeGroup::GlobalEvent,
    // },
    // TODO: Add when atoms included in xml5ever::LocalNameStaticSet
    // OnTimeupdate(Anything<'input>) {
    //     name: "ontimeupdate",
    //     categories: AttributeGroup::GlobalEvent,
    // },
    // TODO: Add when atoms included in xml5ever::LocalNameStaticSet
    // OnToggle(Anything<'input>) {
    //     name: "ontoggle",
    //     categories: AttributeGroup::GlobalEvent,
    // },
    // TODO: Add when atoms included in xml5ever::LocalNameStaticSet
    // OnVolumechange(Anything<'input>) {
    //     name: "onvolumechange",
    //     categories: AttributeGroup::GlobalEvent,
    // },
    // TODO: Add when atoms included in xml5ever::LocalNameStaticSet
    // OnWaiting(Anything<'input>) {
    //     name: "onwaiting",
    //     categories: AttributeGroup::GlobalEvent,
    // },
    // TODO: Add when atoms included in xml5ever::LocalNameStaticSet
    // OnWheel(Anything<'input>) {
    //     name: "onwheel",
    //     categories: AttributeGroup::GlobalEvent,
    // },
    Opacity(Inheritable<Opacity>) {
        name: "opacity",
        categories: AttributeGroup::Presentation,
    },
    OperatorFeComposite(OperatorFeComposite) {
        name: "operator",
        default: OperatorFeComposite::default(),
    },
    OperatorFeMorphology(OperatorFeMorphology) {
        name: "operator",
        default: OperatorFeMorphology::default(),
    },
    Order(NumberOptionalNumber) {
        name: "order",
        default: NumberOptionalNumber(3.0, None),
    },
    Orient(Orient) {
        name: "orient",
    },
    Orientation(Orientation) {
        name: "orientation",
        info: AttributeInfo::DeprecatedUnsafe,
    },
    Origin(Origin) {
        name: "origin",
    },
    Overflow(Inheritable<Overflow>) {
        name: "overflow",
        categories: AttributeGroup::Presentation,
    },
    OverlinePosition(Number) {
        name: "overline-position",
    },
    OverlineThickness(Number) {
        name: "overline-thickness",
    },
    // TODO: Add when atoms included in xml5ever::LocalNameStaticSet
    // PaintOrder(PaintOrder) {
    //     name: "paint-order",
    //     categories: AttributeGroup::Presentation,
    //     info: AttributeInfo::Inheritable,
    // },
    Panose1(ListOf<Integer, Space>) {
        name: "panose-1",
        info: AttributeInfo::DeprecatedUnsafe,
        default: ListOf {
            list: vec![0, 0, 0, 0, 0, 0, 0, 0, 0, 0],
            seperator: Space,
        },
    },
    Path(Path) {
        name: "path",
    },
    PathLength(Number) {
        name: "pathLength",
    },
    PatternUnits(Units) {
        name: "patternUnits",
        default: Units::ObjectBoundingBox,
    },
    PatternContentUnits(Units) {
        name: "patternContentUnits",
        default: Units::UserSpaceOnUse,
    },
    PatternTransform(TransformList) {
        name: "patternTransform",
    },
    // TODO: Add when atoms included in xml5ever::LocalNameStaticSet
    // Pitch(LengthPercentage) {
    //     name: "pitch",
    //     default: LengthPercentage::px(0.0),
    // },
    PointerEvents(Inheritable<PointerEvents>) {
        name: "pointer-events",
        categories: AttributeGroup::Presentation,
        info: AttributeInfo::Inheritable,
    },
    Points(ListOf<Number, SpaceOrComma>) {
        name: "points",
    },
    PointsAtXFe(Number) {
        name: "pointsAtX",
        default: 0.0,
    },
    PointsAtYFe(Number) {
        name: "pointsAtY",
        default: 0.0,
    },
    PointsAtZFe(Number) {
        name: "pointsAtZ",
        default: 0.0,
    },
    PreserveAlphaFeConvolveMatrix(TrueFalse) {
        name: "preserveAlpha",
        default: TrueFalse(false),
    },
    PrimitiveUnits(Units) {
        name: "primitiveUnits",
        default: Units::default(),
    },
    RCircle(LengthPercentage) {
        name: "r",
        default: LengthPercentage::px(0.0),
    },
    RRadialGradient(LengthPercentage) {
        name: "r",
        default: LengthPercentage::Percentage(Percentage(50.0)),
    },
    RadiusFe(NumberOptionalNumber) {
        name: "radius",
        default: NumberOptionalNumber(0.0, None),
    },
    ReferrerPolicy(ReferrerPolicy) {
        name: "referrerpolicy",
    },
    RefX(RefX) {
        name: "refX",
        default: RefX::LengthOrNumber(LengthOrNumber::Number(0.0)),
    },
    RefY(RefY) {
        name: "refY",
        default: RefY::LengthOrNumber(LengthOrNumber::Number(0.0)),
    },
    Rel(ListOf<LinkType, Space>) {
        name: "rel",
    },
    RenderingIntent(RenderingIntent) {
        name: "rendering-intent",
        default: RenderingIntent::default(),
    },
    RepeatCount(RepeatCount) {
        name: "repeatCount",
        categories: AttributeGroup::AnimationTiming,
    },
    RepeatDur(RepeatDur) {
        name: "repeatDur",
        categories: AttributeGroup::AnimationTiming,
    },
    RequiredExtensions(Anything<'input>) {
        name: "requiredExtensions",
        categories: AttributeGroup::ConditionalProcessing,
    },
    RequiredFeatures(Anything<'input>) {
        name: "requiredFeatures",
        categories: AttributeGroup::ConditionalProcessing,
    },
    Restart(Restart) {
        name: "restart",
        categories: AttributeGroup::AnimationTiming,
    },
    Result(Anything<'input>) {
        name: "result",
        categories: AttributeGroup::FilterPrimitive,
    },
    Rotate(Rotate) {
        name: "rotate",
        default: Rotate::Number(0.0),
    },
    RotateHatch(Angle) {
        name: "rotate",
        default: Angle::Deg(0.0),
    },
    ListOfRotate(ListOf<Number, SpaceOrComma>) {
        name: "rotate",
    },
    Ping(ListOf<Url<'input>, Space>) {
        name: "ping",
    },
    Role(ListOf<Role, Space>) {
        name: "role",
        categories: AttributeGroup::Aria,
    },
    RX(Radius) {
        name: "rx",
    },
    RY(Radius) {
        name: "ry",
    },
    ScaleFeDisplacementMap(Number) {
        name: "scale",
        default: 0.0,
    },
    SeedFeTurbulence(Number) {
        name: "seed",
        default: 0.0,
    },
    ShapeRendering(Inheritable<ShapeRendering>) {
        name: "shape-rendering",
        categories: AttributeGroup::Presentation,
        info: AttributeInfo::Inheritable,
    },
    Slope(Number) {
        name: "slope",
        categories: AttributeGroup::TransferFunction,
        info: AttributeInfo::DeprecatedUnsafe,
        default: 0.0,
    },
    // TODO: Add when atoms included in xml5ever::LocalNameStaticSet
    // SolidColor(Paint<'input>) {
    //     name: "solid-color",
    // },
    // TODO: Add when atoms included in xml5ever::LocalNameStaticSet
    // SolidOpacity(Opacity) {
    //     name: "solid-opacity",
    // },
    TextPathSpacing(TextPathSpacing) {
        name: "spacing",
        default: TextPathSpacing::Exact,
    },
    SpecularExponentFe(Number) {
        name: "specularExponent",
        default: 1.0,
    },
    SpecularConstantFeSpecularLighting(Number) {
        name: "specularConstant",
        default: 1.0,
    },
    SpreadMethod(SpreadMethod) {
        name: "spreadMethod",
    },
    StartOffset(LengthOrNumber) {
        name: "startOffset",
        default: LengthOrNumber::Number(0.0),
    },
    StdDeviationFe(NumberOptionalNumber) {
        name: "stdDeviation",
        default: NumberOptionalNumber(0.0, None),
    },
    Stemh(Number) {
        name: "stemh",
        info: AttributeInfo::DeprecatedUnsafe,
    },
    Stemv(Number) {
        name: "stemv",
        info: AttributeInfo::DeprecatedUnsafe,
    },
    StitchTilesFeTurbulence(StitchTilesFeTurbulence) {
        name: "stitchTiles",
        default: StitchTilesFeTurbulence::default(),
    },
    StopColor(Inheritable<Color>) {
        name: "stop-color",
        categories: AttributeGroup::Presentation,
    },
    StopOpacity(Inheritable<Opacity>) {
        name: "stop-opacity",
        categories: AttributeGroup::Presentation,
    },
    String(Anything<'input>) {
        name: "string",
        info: AttributeInfo::DeprecatedUnsafe,
    },
    StrikethroughPosition(Number) {
        name: "strikethrough-position",
    },
    StrikethroughThickness(Number) {
        name: "strikethrough-thickness",
    },
    Stroke(Paint<'input>) {
        name: "stroke",
        categories: AttributeGroup::Presentation,
        info: AttributeInfo::Inheritable,
    },
    StrokeDasharray(Inheritable<StrokeDasharray>) {
        name: "stroke-dasharray",
        categories: AttributeGroup::Presentation,
        info: AttributeInfo::Inheritable,
    },
    StrokeDashoffset(Inheritable<LengthPercentage>) {
        name: "stroke-dashoffset",
        categories: AttributeGroup::Presentation,
        info: AttributeInfo::Inheritable,
    },
    StrokeLinecap(Inheritable<StrokeLinecap>) {
        name: "stroke-linecap",
        categories: AttributeGroup::Presentation,
        info: AttributeInfo::Inheritable,
    },
    StrokeLinejoin(Inheritable<StrokeLinejoin>) {
        name: "stroke-linejoin",
        categories: AttributeGroup::Presentation,
        info: AttributeInfo::Inheritable,
    },
    StrokeMiterlimit(Inheritable<Number>) {
        name: "stroke-miterlimit",
        categories: AttributeGroup::Presentation,
        info: AttributeInfo::Inheritable,
    },
    StrokeOpacity(Inheritable<Opacity>) {
        name: "stroke-opacity",
        categories: AttributeGroup::Presentation,
        info: AttributeInfo::Inheritable,
    },
    StrokeWidth(Inheritable<LengthPercentage>) {
        name: "stroke-width",
        categories: AttributeGroup::Presentation,
        info: AttributeInfo::Inheritable,
    },
    Style(Style<'input>) {
        name: "style",
        categories: AttributeGroup::Core,
    },
    SurfaceScaleFe(Number) {
        name: "surfaceScale",
        default: 1.0,
    },
    SystemLanguage(Anything<'input>) {
        name: "systemLanguage",
        categories: AttributeGroup::ConditionalProcessing,
    },
    Tabindex(Integer) {
        name: "tabindex",
        categories: AttributeGroup::Core,
    },
    TableValues(ListOf<Number, SpaceOrComma>) {
        name: "tableValues",
        categories: AttributeGroup::TransferFunction,
    },
    Target(Target<'input>) {
        name: "target",
        default: Target::_Self,
    },
    TargetXFeConvolveMatrix(Integer) {
        name: "targetX",
    },
    TargetYFeConvolveMatrix(Integer) {
        name: "targetY",
    },
    TextAnchor(Inheritable<TextAnchor>) {
        name: "text-anchor",
        categories: AttributeGroup::Presentation,
        info: AttributeInfo::Inheritable,
    },
    TextDecoration(Inheritable<TextDecoration>) {
        name: "text-decoration",
        categories: AttributeGroup::Presentation,
    },
    TextLength(LengthOrNumber) {
        name: "textLength",
    },
    // TODO: Add when atoms included in xml5ever::LocalNameStaticSet
    // TextPathSide(TextPathSide) {
    //     name: "side",
    // },
    TextRendering(Inheritable<TextRendering>) {
        name: "text-rendering",
        categories: AttributeGroup::Presentation,
        info: AttributeInfo::Inheritable,
    },
    Title(Anything<'input>) {
        name: "title",
    },
    To(Anything<'input>) {
        name: "to",
        categories: AttributeGroup::AnimationValue,
    },
    Transform(TransformList) {
        name: "transform",
        info: AttributeInfo::Inheritable,
    },
    // TODO: Add when atoms included in xml5ever::LocalNameStaticSet
    // TransformOrigin(Position) {
    //     name: "transform-origin",
    //     categories: AttributeGroup::Presentation,
    // },
    Type(TransferFunctionType) {
        name: "type",
        categories: AttributeGroup::TransferFunction,
    },
    TypeAnimateTransform(TypeAnimateTransform) {
        name: "type",
    },
    TypeFeColorMatrix(TypeFeColorMatrix) {
        name: "type",
        default: TypeFeColorMatrix::default(),
    },
    TypeFeTurbulence(TypeFeTurbulence) {
        name: "type",
        default: TypeFeTurbulence::default(),
    },
    TypeScript(MediaType<'input>) {
        name: "type",
    },
    TypeStyle(MediaType<'input>) {
        name: "type",
        default: MediaType::Static("text/css"),
    },
    U1(Anything<'input>) {
        name: "u1",
        info: AttributeInfo::DeprecatedUnsafe,
    },
    U2(Anything<'input>) {
        name: "u2",
        info: AttributeInfo::DeprecatedUnsafe,
    },
    UnderlinePosition(Number) {
        name: "underline-position",
    },
    UnderlineThickness(Number) {
        name: "underline-thickness",
    },
    Unicode(Anything<'input>) {
        name: "unicode",
        info: AttributeInfo::DeprecatedUnsafe,
    },
    UnicodeBidi(Inheritable<UnicodeBidi>) {
        name: "unicode-bidi",
        categories: AttributeGroup::Presentation,
    },
    UnicodeRange(Anything<'input>) {
        name: "unicode-range",
        info: AttributeInfo::DeprecatedUnsafe,
        default: Atom::Static("U+0-10FFFF"),
    },
    UnitsPerEm(Number) {
        name: "units-per-em",
        info: AttributeInfo::DeprecatedUnsafe,
        default: 1000.0,
    },
    VAlphabetic(Number) {
        name: "v-alphabetic",
        info: AttributeInfo::DeprecatedUnsafe,
    },
    Values(ListOf<Anything<'input>, Semicolon>) {
        name: "values",
        categories: AttributeGroup::AnimationValue,
    },
    // TODO: Add when atoms included in xml5ever::LocalNameStaticSet
    // VectorEffect(VectorEffect) {
    //     name: "vector-effect",
    //     categories: AttributeGroup::Presentation,
    // },
    Version(Number) {
        name: "version",
        info: AttributeInfo::DeprecatedSafe,
        default: 1.1,
    },
    VertAdvY(Number) {
        name: "vert-adv-y",
        info: AttributeInfo::DeprecatedUnsafe,
    },
    VertOriginX(Number) {
        name: "vert-origin-x",
        info: AttributeInfo::DeprecatedUnsafe,
    },
    VertOriginY(Number) {
        name: "vert-origin-y",
        info: AttributeInfo::DeprecatedUnsafe,
    },
    VHanging(Number) {
        name: "v-hanging",
        info: AttributeInfo::DeprecatedUnsafe,
    },
    VIdeographic(Number) {
        name: "v-ideographic",
        info: AttributeInfo::DeprecatedUnsafe,
    },
    ValuesFeColorMatrix(ListOf<Number, SpaceOrComma>) {
        name: "values",
    },
    ViewBox(ViewBox) {
        name: "viewBox",
    },
    ViewTarget(Anything<'input>) {
        name: "viewTarget",
        info: AttributeInfo::DeprecatedUnsafe,
    },
    Visibility(Inheritable<Visibility>) {
        name: "visibility",
        categories: AttributeGroup::Presentation,
        info: AttributeInfo::Inheritable,
    },
    VMathematical(Number) {
        name: "v-mathematical",
        info: AttributeInfo::DeprecatedUnsafe,
    },
    Width(LengthPercentage) {
        name: "width",
        categories: AttributeGroup::FilterPrimitive,
    },
    WidthFilter(LengthPercentage) {
        name: "width",
        default: LengthPercentage::Percentage(Percentage(120.0)),
    },
    WidthMask(LengthPercentage) {
        name: "width",
        default: LengthPercentage::Percentage(Percentage(120.0)),
    },
    WidthPattern(LengthPercentage) {
        name: "width",
        default: LengthPercentage::Percentage(Percentage(120.0)),
    },
    WidthSvg(LengthPercentage) {
        name: "width",
        default: LengthPercentage::Percentage(Percentage(100.0)),
    },
    Widths(Anything<'input>) {
        name: "widths",
        info: AttributeInfo::DeprecatedUnsafe,
    },
    WordSpacing(Inheritable<Spacing>) {
        name: "word-spacing",
        categories: AttributeGroup::Presentation,
        info: AttributeInfo::Inheritable,
    },
    WritingMode(Inheritable<WritingMode>) {
        name: "writing-mode",
        categories: AttributeGroup::Presentation,
        info: AttributeInfo::Inheritable,
    },
    X(LengthPercentage) {
        name: "x",
        default: LengthPercentage::px(0.0),
    },
    XFe(Number) {
        name: "x",
        categories: AttributeGroup::FilterPrimitive,
        default: 0.0,
    },
    XFilter(LengthPercentage) {
        name: "x",
        default: LengthPercentage::Percentage(Percentage(-10.0)),
    },
    XMask(LengthPercentage) {
        name: "x",
        default: LengthPercentage::Percentage(Percentage(-10.0)),
    },
    X1(LengthOrNumber) {
        name: "x1",
    },
    X1LinearGradient(LengthPercentage) {
        name: "x1",
        default: LengthPercentage::Percentage(Percentage(0.0)),
    },
    X2(LengthOrNumber) {
        name: "x2",
    },
    X2LinearGradient(LengthPercentage) {
        name: "x2",
        default: LengthPercentage::Percentage(Percentage(100.0)),
    },
    XChannelSelectorFeDisplacementMap(ChannelSelector) {
        name: "xChannelSelector",
        default: ChannelSelector::default(),
    },
    XHeight(Number) {
        name: "x-height",
        info: AttributeInfo::DeprecatedUnsafe,
    },
    XLinkHref(Url<'input>) {
        prefix: XLink,
        name: "href",
        categories: AttributeGroup::XLink,
    },
    XLinkShow(XLinkShow) {
        prefix: XLink,
        name: "show",
        categories: AttributeGroup::XLink,
    },
    XLinkTitle(Anything<'input>) {
        prefix: XLink,
        name: "title",
        categories: AttributeGroup::XLink,
    },
    XMLNS(Anything<'input>) {
        name: "xmlns",
    },
    XmlBase(Anything<'input>) {
        prefix: XML,
        name: "base",
        categories: AttributeGroup::Core,
    },
    XmlLang(Anything<'input>) {
        prefix: XML,
        name: "lang",
    },
    XmlSpace(XmlSpace) {
        prefix: XML,
        name: "space",
        categories: AttributeGroup::Core,
    },
    Y(LengthPercentage) {
        name: "y",
        categories: AttributeGroup::FilterPrimitive,
        default: LengthPercentage::px(0.0),
    },
    YFe(Number) {
        name: "y",
        default: 0.0,
    },
    YFilter(LengthPercentage) {
        name: "y",
        default: LengthPercentage::Percentage(Percentage(-10.0)),
    },
    YMask(LengthPercentage) {
        name: "y",
        default: LengthPercentage::Percentage(Percentage(-10.0)),
    },
    Y1(LengthOrNumber) {
        name: "y1",
    },
    Y1LinearGradient(LengthPercentage) {
        name: "y1",
        default: LengthPercentage::Percentage(Percentage(-10.0)),
    },
    Y2(LengthOrNumber) {
        name: "y2",
    },
    Y2LinearGradient(LengthPercentage) {
        name: "y2",
        default: LengthPercentage::Percentage(Percentage(0.0)),
    },
    YChannelSelectorFeDisplacementMap(ChannelSelector) {
        name: "yChannelSelector",
        default: ChannelSelector::default(),
    },
    ZFe(Number) {
        name: "z",
        default: 0.0,
    },
    ZoomAndPan(ZoomAndPan) {
        name: "zoomAndPan",
        info: AttributeInfo::DeprecatedUnsafe,
        default: ZoomAndPan::Magnify,
    },
}

macro_rules! try_from_into_property {
    ($($name:ident($value:ident$(, $vp:tt)?) => $to_css:expr => $to_attr:expr,)+) => {
        impl<'input> TryFrom<Property<'input>> for Attr<'input> {
            type Error = ();

            fn try_from(value: Property<'input>) -> Result<Self, Self::Error> {
                use lightningcss::properties::custom::{
                    CustomProperty, CustomPropertyName
                };
                Ok(match value {
                    $(Property::$name($value$(, $vp)?) $(if $vp == VendorPrefix::None)? => {
                        Attr::$name($to_css)
                    })+
                    Property::Custom(CustomProperty {
                        name: CustomPropertyName::Unknown(name),
                        value,
                    }) => Attr::CSSUnknown {
                        attr_id: AttrId::Unknown(QualName {
                            prefix: Prefix::SVG,
                            local: name.0.into(),
                        }),
                        value: TokenList(value),
                    },
                    _ => return Err(())
                })
            }
        }
        impl<'input> TryFrom<&PropertyId<'input>> for AttrId<'input> {
            type Error = ();

            fn try_from(value: &PropertyId<'input>) -> Result<Self, Self::Error> {
                Ok(match value {
                    $(PropertyId::$name$(($vp))? $(if *$vp == VendorPrefix::None)? => {
                        AttrId::$name
                    })+
                    _ => return Err(())
                })
            }
        }
        macro_rules! vp_default {
            ($_vp:tt) => { VendorPrefix::None };
        }
        impl<'input> From<&AttrId<'input>> for PropertyId<'input> {
            fn from(value: &AttrId<'input>) -> Self {
                match value {
                    $(AttrId::$name => PropertyId::$name$((vp_default!($vp)))?,)+
                    _ => PropertyId::Custom(
                        lightningcss::properties::custom::CustomPropertyName::Unknown(
                            value.local_name().to_string().into()
                        )
                    )
                }
            }
        }
        impl<'input> TryFrom<Attr<'input>> for Property<'input> {
            type Error = ();

            fn try_from(value: Attr<'input>) -> Result<Self, Self::Error> {
                Ok(match value {
                    $(Attr::$name($value) => Property::$name($to_attr$(, vp_default!($vp))?),)+
                    _ => return Err(())
                })
            }
        }
    };
}
try_from_into_property! {
    ClipPath(value, vp) => Inheritable::Defined(value) => value.option().ok_or(())?,
    ClipRule(value) => Inheritable::Defined(value) => value.option().ok_or(())?,
    Color(value) => Inheritable::Defined(value) => value.option().ok_or(())?,
    ColorInterpolation(value) => Inheritable::Defined(value) => value.option().ok_or(())?,
    ColorInterpolationFilters(value) => Inheritable::Defined(value) => value.option().ok_or(())?,
    ColorRendering(value) => Inheritable::Defined(value) => value.option().ok_or(())?,
    Cursor(value) => Inheritable::Defined(value) => value.option().ok_or(())?,
    Display(value) => Inheritable::Defined(value) => value.option().ok_or(())?,
    Fill(value) => value => value,
    FillOpacity(value) => Inheritable::Defined(value) => value.option().ok_or(())?,
    FillRule(value) => Inheritable::Defined(value) => value.option().ok_or(())?,
    Filter(value, vp) => Inheritable::Defined(value) => value.option().ok_or(())?,
    FontFamily(value) => Inheritable::Defined(FontFamily(ListOf {
        list: value,
        seperator: Comma,
    })) => value.option().ok_or(())?.0.list,
    FontSize(value) => Inheritable::Defined(value) => value.option().ok_or(())?,
    FontStretch(value) => Inheritable::Defined(value) => value.option().ok_or(())?,
    FontStyle(value) => Inheritable::Defined(value) => value.option().ok_or(())?,
    FontWeight(value) => Inheritable::Defined(value) => value.option().ok_or(())?,
    ImageRendering(value) => Inheritable::Defined(value) => value.option().ok_or(())?,
    LetterSpacing(value) => Inheritable::Defined(value) => value.option().ok_or(())?,
    Marker(value) => Inheritable::Defined(value) => value.option().ok_or(())?,
    MarkerEnd(value) => Inheritable::Defined(value) => value.option().ok_or(())?,
    MarkerMid(value) => Inheritable::Defined(value) => value.option().ok_or(())?,
    MarkerStart(value) => Inheritable::Defined(value) => value.option().ok_or(())?,
    Mask(value, vp) => Inheritable::Defined(Mask(ListOf {
        list: value.to_vec(),
        seperator: Comma,
    })) => value.option().ok_or(())?.0.list.into(),
    Opacity(value) => Inheritable::Defined(value) => value.option().ok_or(())?,
    Overflow(value) => Inheritable::Defined(value) => value.option().ok_or(())?,
    ShapeRendering(value) => Inheritable::Defined(value) => value.option().ok_or(())?,
    Stroke(value) => value => value,
    StrokeDasharray(value) => Inheritable::Defined(value) => value.option().ok_or(())?,
    StrokeDashoffset(value) => Inheritable::Defined(LengthPercentage(value)) => value.option().ok_or(())?.0,
    StrokeLinecap(value) => Inheritable::Defined(value) => value.option().ok_or(())?,
    StrokeLinejoin(value) => Inheritable::Defined(value) => value.option().ok_or(())?,
    StrokeMiterlimit(value) => Inheritable::Defined(value) => value.option().ok_or(())?,
    StrokeOpacity(value) => Inheritable::Defined(value) => value.option().ok_or(())?,
    StrokeWidth(value) => Inheritable::Defined(LengthPercentage(value)) => value.option().ok_or(())?.0,
    TextDecoration(value, vp) => Inheritable::Defined(value) => value.option().ok_or(())?,
    TextRendering(value) => Inheritable::Defined(value) => value.option().ok_or(())?,
    // TODO: When xml5ever supports
    // TransformOrigin(value, vp) => value => value,
    UnicodeBidi(value) => Inheritable::Defined(value) => value.option().ok_or(())?,
    Visibility(value) => Inheritable::Defined(value) => value.option().ok_or(())?,
}

impl Attr<'_> {
    /// Returns an `AttrId` that may be prefixed as `AttrId::Aliased` as the inner id
    /// that's aliased.
    pub fn unaliased(&self) -> &Self {
        match self {
            Self::Aliased { value, .. } => {
                let result = value.as_ref();
                debug_assert!(!matches!(result, Self::Aliased { .. }));
                result
            }
            _ => self,
        }
    }

    /// Returns an `AttrId` that may be prefixed as `AttrId::Aliased` as the inner id
    /// that's aliased.
    pub fn unaliased_mut(&mut self) -> &mut Self {
        match self {
            Self::Aliased { value, .. } => {
                let result = value.as_mut();
                debug_assert!(!matches!(result, Self::Aliased { .. }));
                result
            }
            _ => self,
        }
    }
}

impl AttrId<'_> {
    /// Returns an `AttrId` that may be prefixed as `AttrId::Aliased` as the inner id
    /// that's aliased.
    pub fn unaliased(&self) -> &Self {
        match self {
            Self::Aliased { attr_id, .. } => {
                let result = attr_id.as_ref();
                debug_assert!(!matches!(result, Self::Aliased { .. }));
                result
            }
            _ => self,
        }
    }
}

impl std::fmt::Display for AttrId<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if let Some(prefix) = self.prefix().value() {
            f.write_fmt(format_args!("{prefix}:"))?;
        }
        self.local_name().fmt(f)
    }
}

thread_local! {
    static ATTRIBUTE_GROUP_ATTRIBUTES_MEMO: RefCell<HashMap<AttributeGroup, &'static [&'static AttrId<'static>]>> = RefCell::default();
}
impl AttributeGroup {
    /// Returns the list of attributes associated with the attribute group
    pub fn attributes(&self) -> &'static [&'static AttrId<'static>] {
        if let Some(attributes) =
            ATTRIBUTE_GROUP_ATTRIBUTES_MEMO.with(|memo| memo.borrow().get(self).copied())
        {
            return attributes;
        }
        let attributes = _attr_id::all
            .iter()
            .filter(|attr| self.intersects(attr.attribute_group()))
            .copied()
            .collect();
        let attributes: &'static [_] = Vec::leak(attributes);
        ATTRIBUTE_GROUP_ATTRIBUTES_MEMO.with(move |memo| {
            let mut memo = memo.borrow_mut();
            let result = memo.insert(*self, attributes);
            debug_assert!(result.is_none());
        });
        self.attributes()
    }

    /// Returns an `AttrId` that matches the attribute groups.
    /// Returns an unknown `AttrId` otherwise
    pub fn parse_attr_id<'input>(
        &self,
        prefix: Prefix<'input>,
        local: Atom<'input>,
    ) -> AttrId<'input> {
        self.attributes()
            .iter()
            .find(|attr| *attr.prefix() == prefix && *attr.local_name() == local)
            .copied()
            .cloned()
            .unwrap_or_else(|| AttrId::Unknown(QualName { prefix, local }))
    }
}
