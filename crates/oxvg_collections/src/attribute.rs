//! Types of data which can be assigned to an element's attributes
use animation::{AttributeType, BeginEnd, CalcMode, ControlPoint};
use animation_addition::{Accumulate, Additive};
use animation_timing::{Dur, Fill, MinMax, RepeatCount, RepeatDur, Restart};
use aria::{
    AriaAutocomplete, AriaCurrent, AriaDropEffect, AriaHasPopup, AriaInvalid, AriaLive,
    AriaOrientation, AriaRelevant, AriaSort, IDReference, Role, Tristate,
};
use core::{
    Angle, Anything, Boolean, Class, Color, Id, Integer, Length, Number, NumberOptionalNumber,
    Opacity, Paint, SVGTransformList, Style, TokenList, Url,
};
use dashmap::DashMap;
use filter_effect::{
    ChannelSelector, EdgeMode, In, OperatorFeComposite, OperatorFeMorphology,
    StitchTilesFeTurbulence, TypeFeColorMatrix, TypeFeTurbulence,
};
use fonts::{ArabicForm, Orientation};
use inheritable::Inheritable;
use lightningcss::{
    media_query::{MediaList, MediaQuery},
    properties::{
        font::{AbsoluteFontSize, AbsoluteFontWeight, FontStretchKeyword},
        ui::CursorKeyword,
        Property, PropertyId,
    },
    values::{
        alpha::AlphaValue,
        percentage::{DimensionPercentage, Percentage},
        position::{HorizontalPosition, VerticalPosition},
    },
    vendor_prefix::VendorPrefix,
};
use list_of::{Comma, ListOf, Semicolon, Seperators, Space, SpaceOrComma};
use path::{Path, Points};
use presentation::{
    AlignmentBaseline, BaselineShift, Clip, ClipPath, ColorInterpolation, ColorProfile,
    ColorRendering, Cursor, Direction, Display, DominantBaseline, EnableBackground, FillRule,
    FilterList, Font, FontFamily, FontSize, FontSizeAdjust, FontStretch, FontStyle, FontVariant,
    FontWeight, GlyphOrientationVertical, ImageRendering, Kerning, LengthOrNumber,
    LengthPercentage, Marker, Mask, Overflow, PaintOrder, PointerEvents, Position, ShapeRendering,
    Spacing, StrokeDasharray, StrokeLinecap, StrokeLinejoin, TextAnchor, TextDecoration,
    TextRendering, UnicodeBidi, VectorEffect, Visibility, WritingMode,
};
use smallvec::SmallVec;
use transfer_function::TransferFunctionType;
use uncategorised::{
    BlendMode, ColorProfileName, CrossOrigin, LengthAdjust, LinkType, MarkerUnits, MediaQueryList,
    MediaType, NumberPercentage, Orient, Origin, Playbackorder, PreserveAspectRatio, Radius, RefX,
    RefY, ReferrerPolicy, RenderingIntent, Rotate, SpreadMethod, Target, TextPathMethod,
    TextPathSide, TextPathSpacing, Timelinebegin, TrueFalse, TrueFalseUndefined,
    TypeAnimateTransform, Units, ViewBox, ZoomAndPan,
};
use xlink::{XLinkActuate, XLinkShow, XLinkType};
use xml::XmlSpace;

use super::{
    atom::Atom,
    content_type::{ContentType, ContentTypeId, ContentTypeRef},
    name::{Prefix, QualName},
};

#[cfg(feature = "parse")]
use oxvg_parse::Parse;

pub use group::{AttributeGroup, AttributeInfo};

mod group;

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

        #[cfg(feature = "parse")]
        impl<'input> $crate::attribute::Parse<'input> for $attr {
            fn parse<'t>(
                input: &mut oxvg_parse::Parser<'input>,
            ) -> Result<Self, oxvg_parse::error::Error<'input>> {
                let str = input.expect_ident()?;
                match str {
                    $($value => Ok($attr::$name),)+
                    received => Err(oxvg_parse::error::Error::ExpectedIdent {
                        expected: concat!("one of", $(" `", $value, "`"),+),
                        received,
                    })
                }
            }
        }

        #[cfg(feature = "serialize")]
        impl oxvg_serialize::ToValue for $attr {
            fn write_value<W>(
                &self,
                dest: &mut oxvg_serialize::Printer<W>,
            ) -> Result<(), oxvg_serialize::error::PrinterError>
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
    ($($(#[$meta:meta])*
    $attr:ident(
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
                $(#[$meta:meta])*
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
                $(#[$meta:meta])*
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

            // For use in macros interoperable with `Attr`
            #[doc(hidden)]
            pub fn name<'a>(&'a self) -> &'a AttrId<'input> {
                self
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
            #[cfg(feature = "parse")]
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
            /// Note that the prefix will be unaliased.
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

        #[cfg(feature = "serialize")]
        impl oxvg_serialize::ToValue for Attr<'_> {
            fn write_value<W>(&self, dest: &mut oxvg_serialize::Printer<W>) -> Result<(), oxvg_serialize::error::PrinterError>
                where
                    W: std::fmt::Write {
                match self {
                    $(Self::$attr(value) => value.write_value(dest),)+
                    Self::Aliased { value, .. } => value.write_value(dest),
                    Self::Unparsed { value, .. } => value.write_value(dest),
                    Self::CSSUnknown { value, .. } => value.write_value(dest),
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
        impl PartialOrd for AttrId<'_> {
            fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
                Some(self.cmp(other))
            }
        }
impl Ord for AttrId<'_> {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.prefix()
            .cmp(other.prefix())
            .then_with(|| self.local_name().cmp(other.local_name()))
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

// NOTE: Attributes are ordered according to the spec's attribute index
// NOTE: Attributes with conflicting qual-names must be qualified in their
//       Rust type (e.g. "d" -> DPath | DGlyph)
// NOTE: Presentation attributes are not qualified, since the must match
//       lightningcss (e.g. "fill" -> FillAnimate | Fill)
// NOTE: Where the spec specifies "This attribute has the same meaning
//       as the ‘...’ attribute on the ‘...’ element." or are seemingly
//       identical are not considered conflicting.
// NOTE: Presentation attributes are at the end of the list
//
// [w3 | SVG 1.1](https://www.w3.org/TR/2011/REC-SVG11-20110816/attindex.html)
// [w3 | SVG 1.1 (raw)](https://github.com/w3c/svgwg/blob/main/specs/integration/master/definitions-SVG11.xml)
// [w3 | SVG 2](https://svgwg.org/svg2-draft/attindex.html)
// [w3 | SVG 2 (raw)](https://github.com/w3c/svgwg/blob/main/master/definitions.xml#L13)
// [w3 | SVG Properties](https://svgwg.org/svg2-draft/propidx.html)
define_attrs! {
    AccentHeight(Number) {
        name: "accent-height",
        info: AttributeInfo::DeprecatedUnsafe,
    },
    Accumulate(Accumulate) {
        name: "accumulate",
        categories: AttributeGroup::AnimationAddition,
        default: Accumulate::None,
    },
    Additive(Additive) {
        name: "additive",
        categories: AttributeGroup::AnimationAddition,
        default: Additive::Replace,
    },
    Alphabetic(Number) {
        name: "alphabetic",
        info: AttributeInfo::DeprecatedUnsafe,
    },
    Amplitude(Number) {
        name: "amplitude",
        categories: AttributeGroup::TransferFunction,
        default: 1.0,
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
        default: AriaAutocomplete::None,
    },
    AriaBusy(TrueFalse) {
        name: "aria-busy",
        categories: AttributeGroup::Aria,
        default: TrueFalse(false),
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
        default: AriaCurrent::False,
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
        default: TrueFalse(false),
    },
    AriaDropEffect(ListOf<AriaDropEffect, Space>) {
        name: "aria-dropeffect",
        categories: AttributeGroup::Aria,
        default: ListOf {
            list: vec![AriaDropEffect::None],
            seperator: Space,
        },
    },
    AriaErrorMessage(IDReference<'input>) {
        name: "aria-errormessage",
        categories: AttributeGroup::Aria,
    },
    AriaExpanded(TrueFalseUndefined) {
        name: "aria-expanded",
        categories: AttributeGroup::Aria,
        default: TrueFalseUndefined(None),
    },
    AriaFlowTo(ListOf<IDReference<'input>, Space>) {
        name: "aria-flowto",
        categories: AttributeGroup::Aria,
    },
    AriaGrabbed(TrueFalseUndefined) {
        name: "aria-grabbed",
        categories: AttributeGroup::Aria,
        default: TrueFalseUndefined(None),
    },
    AriaHasPopup(AriaHasPopup) {
        name: "aria-haspopup",
        categories: AttributeGroup::Aria,
        default: AriaHasPopup::False,
    },
    AriaHidden(TrueFalseUndefined) {
        name: "aria-hidden",
        categories: AttributeGroup::Aria,
        default: TrueFalseUndefined(None),
    },
    AriaInvalid(AriaInvalid) {
        name: "aria-invalid",
        categories: AttributeGroup::Aria,
        default: AriaInvalid::False,
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
        default: AriaLive::Off,
    },
    AriaModal(TrueFalse) {
        name: "aria-modal",
        categories: AttributeGroup::Aria,
        default: TrueFalse(false),
    },
    AriaMultiline(TrueFalse) {
        name: "aria-multiline",
        categories: AttributeGroup::Aria,
        default: TrueFalse(false),
    },
    AriaMultiselectable(TrueFalse) {
        name: "aria-multiselectable",
        categories: AttributeGroup::Aria,
        default: TrueFalse(false),
    },
    AriaOrientation(AriaOrientation) {
        name: "aria-orientation",
        categories: AttributeGroup::Aria,
        default: AriaOrientation::Undefined,
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
        default: TrueFalse(false),
    },
    AriaRelevant(ListOf<AriaRelevant, Space>) {
        name: "aria-relevant",
        categories: AttributeGroup::Aria,
    },
    AriaRequired(TrueFalse) {
        name: "aria-required",
        categories: AttributeGroup::Aria,
        default: TrueFalse(false),
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
        default: TrueFalseUndefined(None),
    },
    AriaSetSize(Integer) {
        name: "aria-setsize",
        categories: AttributeGroup::Aria,
    },
    AriaSort(AriaSort) {
        name: "aria-sort",
        categories: AttributeGroup::Aria,
        default: AriaSort::None,
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
    AttributeType(AttributeType) {
        name: "attributeType",
        categories: AttributeGroup::AnimationAttributeTarget,
        info: AttributeInfo::DeprecatedUnsafe,
        default: AttributeType::Auto,
    },
    AutoFocus(Boolean<'input>) {
        name: "autofocus",
        categories: AttributeGroup::Core,
    },
    Azimuth(Number) {
        name: "azimuth",
        default: 0.0,
    },
    BaseFrequency(NumberOptionalNumber) {
        name: "baseFrequency",
        default: NumberOptionalNumber(0.0, None),
    },
    BaseProfile(Anything<'input>) {
        name: "baseProfile",
        info: AttributeInfo::DeprecatedUnsafe,
        default: Atom::Static("none"),
    },
    Bbox(ListOf<Number, Comma>) {
        name: "bbox",
        info: AttributeInfo::DeprecatedUnsafe,
    },
    Begin(ListOf<BeginEnd<'input>, Semicolon>) {
        name: "begin",
        categories: AttributeGroup::AnimationTiming,
    },
    Bias(Number) {
        name: "bias",
        default: 0.0,
    },
    By(Anything<'input>) {
        name: "by",
        categories: AttributeGroup::AnimationValue,
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
    ClipPathUnits(Units) {
        name: "clipPathUnits",
        default: Units::UserSpaceOnUse,
    },
    ContentScriptType(Anything<'input>) {
        name: "contentScriptType",
        info: AttributeInfo::DeprecatedUnsafe,
        default: Anything::Static("application/ecmascript"),
    },
    ContentStyleType(Anything<'input>) {
        name: "contentStyleType",
        info: AttributeInfo::DeprecatedUnsafe,
        default: Anything::Static("text/css"),
    },
    CrossOrigin(CrossOrigin) {
        name: "crossorigin",
    },
    CXGeometry(LengthPercentage) {
        name: "cx",
        default: LengthPercentage::px(0.0),
    },
    CXRadialGradient(LengthPercentage) {
        name: "cx",
        default: LengthPercentage::px(50.0),
    },
    CYGeometry(LengthPercentage) {
        name: "cy",
        default: LengthPercentage::px(0.0),
    },
    CYRadialGradient(LengthPercentage) {
        name: "cy",
        default: LengthPercentage::px(50.0),
    },
    D(Path) {
        name: "d",
    },
    Descent(Number) {
        name: "descent",
        info: AttributeInfo::DeprecatedUnsafe,
    },
    DiffuseConstant(Number) {
        name: "diffuseConstant",
        default: 1.0,
    },
    Divisor(Number) {
        name: "divisor",
        default: 1.0,
    },
    Download(Anything<'input>) {
        name: "download",
    },
    Dur(Dur) {
        name: "dur",
        categories: AttributeGroup::AnimationTiming,
    },
    DXAltGlyph(ListOf<Length, SpaceOrComma>) {
        name: "dx",
        info: AttributeInfo::DeprecatedUnsafe,
    },
    DXFeDropShadow(Length) {
        name: "dx",
        default: Length::Number(2.0),
    },
    DXFeOffset(Number) {
        name: "dx",
        default: 0.0,
    },
    DXGlyphRef(Number) {
        name: "dx",
        info: AttributeInfo::DeprecatedUnsafe,
        default: 0.0,
    },
    DXText(ListOf<Length, SpaceOrComma>) {
        name: "dx",
    },
    DXTSpan(ListOf<Length, SpaceOrComma>) {
        name: "dx",
    },
    DYAltGlyph(ListOf<Length, SpaceOrComma>) {
        name: "dy",
        info: AttributeInfo::DeprecatedUnsafe,
    },
    DYFeDropShadow(Number) {
        name: "dy",
        default: 2.0,
    },
    DYFeOffset(Number) {
        name: "dy",
        default: 0.0,
    },
    DYGlyphRef(ListOf<Length, SpaceOrComma>) {
        name: "dy",
        info: AttributeInfo::DeprecatedUnsafe,
    },
    DYText(ListOf<Length, SpaceOrComma>) {
        name: "dy",
    },
    DYTSpan(ListOf<Length, SpaceOrComma>) {
        name: "dy",
    },
    EdgeModeFeConvolveMatrix(EdgeMode) {
        name: "edgeMode",
        default: EdgeMode::Duplicate,
    },
    EdgeModeFeGaussianBlur(EdgeMode) {
        name: "edgeMode",
        default: EdgeMode::None,
    },
    Elevation(Number) {
        name: "elevation",
        default: 0.0,
    },
    End(ListOf<BeginEnd<'input>, Semicolon>) {
        name: "end",
        categories: AttributeGroup::AnimationTiming,
    },
    Exponent(Number) {
        name: "exponent",
        categories: AttributeGroup::TransferFunction,
        default: 1.0,
    },
    ExternalResourcesRequired(TrueFalse) {
        name: "externalResourcesRequired",
        info: AttributeInfo::DeprecatedUnsafe,
        default: TrueFalse(false),
    },
    FillAnimate(Fill) {
        name: "fill",
        categories: AttributeGroup::AnimationTiming,
        default: Fill::Remove,
    },
    FilterRes(NumberOptionalNumber) {
        name: "filterRes",
        info: AttributeInfo::DeprecatedUnsafe,
    },
    FilterUnits(Units) {
        name: "filterUnits",
    },
    Format(Anything<'input>) {
        name: "format",
        info: AttributeInfo::DeprecatedUnsafe,
    },
    FR(Length) {
        name: "fr",
        default: Length::Percentage(Percentage(0.0)),
    },
    From(Anything<'input>) {
        name: "from",
        categories: AttributeGroup::AnimationValue,
    },
    FX(Length) {
        name: "fx",
    },
    FY(Length) {
        name: "fy",
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
    GlyphRef(Anything<'input>) {
        name: "glyphRef",
        info: AttributeInfo::DeprecatedUnsafe,
    },
    GradientTransform(SVGTransformList) {
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
    // https://docs.w3cub.com/svg/element/hatch.html
    HatchContentUnits(Units) {
        name: "hatchContentUnits",
        info: AttributeInfo::DeprecatedUnsafe,
        default: Units::UserSpaceOnUse,
    },
    // https://docs.w3cub.com/svg/element/hatch.html
    HatchUnits(Units) {
        name: "hatchUnits",
        info: AttributeInfo::DeprecatedUnsafe,
        default: Units::ObjectBoundingBox,
    },
    HeightFilter(LengthPercentage) {
        name: "height",
        default: LengthPercentage::Percentage(Percentage(120.0)),
    },
    HeightForeignObject(LengthPercentage) {
        name: "height",
    },
    HeightImage(LengthPercentage) {
        name: "height",
    },
    HeightPattern(LengthPercentage) {
        name: "height",
        default: LengthPercentage::Percentage(Percentage(0.0)),
    },
    HeightRect(LengthPercentage) {
        name: "height",
    },
    HeightSvg(LengthPercentage) {
        name: "height",
        default: LengthPercentage::Percentage(Percentage(100.0)),
    },
    // NOTE: Missing from index (https://github.com/w3c/svgwg/issues/1027)
    HeightSymbol(LengthPercentage) {
        name: "height",
    },
    HeightUse(LengthPercentage) {
        name: "height",
    },
    HeightFe(LengthPercentage) {
        name: "height",
        categories: AttributeGroup::FilterPrimitive,
    },
    HeightMask(LengthPercentage) {
        name: "height",
        default: LengthPercentage::Percentage(Percentage(120.0)),
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
    In(In<'input>) {
        name: "in",
    },
    In2(In<'input>) {
        name: "in2",
    },
    Intercept(Number) {
        name: "intercept",
        categories: AttributeGroup::TransferFunction,
        default: 0.0,
    },
    K(Number) {
        name: "k",
        info: AttributeInfo::DeprecatedUnsafe,
    },
    K1(Number) {
        name: "k1",
        default: 0.0,
    },
    K2(Number) {
        name: "k2",
        default: 0.0,
    },
    K3(Number) {
        name: "k3",
        default: 0.0,
    },
    K4(Number) {
        name: "k4",
        default: 0.0,
    },
    KernalMatrix(ListOf<Number, SpaceOrComma>) {
        name: "kernelMatrix",
    },
    KernelUnitLength(NumberOptionalNumber) {
        name: "kernelUnitLength",
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
    LimitingConeAngle(Number) {
        name: "limitingConeAngle",
    },
    Local(Anything<'input>) {
        name: "local",
        info: AttributeInfo::DeprecatedUnsafe,
    },
    MarkerHeight(LengthOrNumber) {
        name: "markerHeight",
        default: LengthOrNumber::Number(3.0),
    },
    MarkerUnits(MarkerUnits) {
        name: "markerUnits",
        default: MarkerUnits::StrokeWidth,
    },
    MarkerWidth(LengthOrNumber) {
        name: "markerWidth",
        default: LengthOrNumber::Number(3.0),
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
        default: MediaQueryList(MediaList {
            media_queries: vec![MediaQuery {
                qualifier: None,
                media_type: lightningcss::media_query::MediaType::All,
                condition: None,
            }],
        }),
    },
    Method(TextPathMethod) {
        name: "method",
        default: TextPathMethod::Align,
    },
    Min(MinMax) {
        name: "min",
        categories: AttributeGroup::AnimationTiming,
    },
    Mode(BlendMode) {
        name: "mode",
        default: BlendMode::Normal,
    },
    NameColorProfile(ColorProfileName<'input>) {
        name: "name",
        info: AttributeInfo::DeprecatedUnsafe,
        default: ColorProfileName::default(),
    },
    NameFontFace(Anything<'input>) {
        name: "name",
        info: AttributeInfo::DeprecatedUnsafe,
    },
    NumOctaves(Integer) {
        name: "numOctaves",
        default: 1,
    },
    OffsetStop(NumberPercentage) {
        name: "offset",
    },
    OffsetFe(Number) {
        name: "offset",
        categories: AttributeGroup::TransferFunction,
        default: 0.0,
    },
    OffsetHatchPath(NumberPercentage) {
        name: "offset",
        info: AttributeInfo::DeprecatedUnsafe,
    },
    OnAbort(Anything<'input>) {
        name: "onabort",
        categories: AttributeGroup::DocumentEvent,
    },
    OnActivate(Anything<'input>) {
        name: "onactivate",
        categories: AttributeGroup::GraphicalEvent,
    },
    OnBegin(Anything<'input>) {
        name: "onbegin",
        categories: AttributeGroup::AnimationEvent,
    },
    OnCancel(Anything<'input>) {
        name: "oncancel",
        categories: AttributeGroup::GlobalEvent,
    },
    OnCanplay(Anything<'input>) {
        name: "oncanplay",
        categories: AttributeGroup::GlobalEvent,
    },
    OnCanplaythrough(Anything<'input>) {
        name: "oncanplaythrough",
        categories: AttributeGroup::GlobalEvent,
    },
    OnChange(Anything<'input>) {
        name: "onchange",
        categories: AttributeGroup::GlobalEvent,
    },
    OnClick(Anything<'input>) {
        name: "onclick",
        categories: AttributeGroup::GlobalEvent
            .union(AttributeGroup::GraphicalEvent),
    },
    OnClose(Anything<'input>) {
        name: "onclose",
        categories: AttributeGroup::GlobalEvent,
    },
    OnCopy(Anything<'input>) {
        name: "oncopy",
        categories: AttributeGroup::DocumentElementEvent
            .union(AttributeGroup::GlobalEvent),
    },
    OnCuechange(Anything<'input>) {
        name: "oncuechange",
        categories: AttributeGroup::GlobalEvent,
    },
    OnCut(Anything<'input>) {
        name: "oncut",
        categories: AttributeGroup::DocumentElementEvent
            .union(AttributeGroup::GlobalEvent),
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
    OnDragexit(Anything<'input>) {
        name: "ondragexit",
        categories: AttributeGroup::GlobalEvent,
    },
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
    OnDurationchange(Anything<'input>) {
        name: "ondurationchange",
        categories: AttributeGroup::GlobalEvent,
    },
    OnEmptied(Anything<'input>) {
        name: "onemptied",
        categories: AttributeGroup::GlobalEvent,
    },
    OnEnd(Anything<'input>) {
        name: "onend",
        categories: AttributeGroup::AnimationEvent,
    },
    OnEnded(Anything<'input>) {
        name: "onended",
        categories: AttributeGroup::GlobalEvent,
    },
    OnError(Anything<'input>) {
        name: "onerror",
        categories: AttributeGroup::GlobalEvent
            .union(AttributeGroup::DocumentEvent),
    },
    OnFocusIn(Anything<'input>) {
        name: "onfocusin",
        categories: AttributeGroup::GraphicalEvent,
    },
    OnFocusOut(Anything<'input>) {
        name: "onfocusout",
        categories: AttributeGroup::GraphicalEvent,
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
        categories: AttributeGroup::GlobalEvent
            .union(AttributeGroup::GraphicalEvent)
            .union(AttributeGroup::AnimationEvent),
    },
    OnLoadeddata(Anything<'input>) {
        name: "onloadeddata",
        categories: AttributeGroup::GlobalEvent,
    },
    OnLoadedmetadata(Anything<'input>) {
        name: "onloadedmetadata",
        categories: AttributeGroup::GlobalEvent,
    },
    OnLoadstart(Anything<'input>) {
        name: "onloadstart",
        categories: AttributeGroup::GlobalEvent,
    },
    OnMousedown(Anything<'input>) {
        name: "onmousedown",
        categories: AttributeGroup::GlobalEvent
            .union(AttributeGroup::GraphicalEvent),
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
        categories: AttributeGroup::GlobalEvent
            .union(AttributeGroup::GraphicalEvent),
    },
    OnMouseout(Anything<'input>) {
        name: "onmouseout",
        categories: AttributeGroup::GlobalEvent
            .union(AttributeGroup::GraphicalEvent),
    },
    OnMouseover(Anything<'input>) {
        name: "onmouseover",
        categories: AttributeGroup::GlobalEvent
            .union(AttributeGroup::GraphicalEvent),
    },
    OnMouseup(Anything<'input>) {
        name: "onmouseup",
        categories: AttributeGroup::GlobalEvent
            .union(AttributeGroup::GraphicalEvent),
    },
    OnPaste(Anything<'input>) {
        name: "onpaste",
        categories: AttributeGroup::DocumentElementEvent
            .union(AttributeGroup::GlobalEvent),
    },
    OnPause(Anything<'input>) {
        name: "onpause",
        categories: AttributeGroup::GlobalEvent,
    },
    OnPlay(Anything<'input>) {
        name: "onplay",
        categories: AttributeGroup::GlobalEvent,
    },
    OnPlaying(Anything<'input>) {
        name: "onplaying",
        categories: AttributeGroup::GlobalEvent,
    },
    OnProgress(Anything<'input>) {
        name: "onprogress",
        categories: AttributeGroup::GlobalEvent,
    },
    OnRatechange(Anything<'input>) {
        name: "onratechange",
        categories: AttributeGroup::GlobalEvent,
    },
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
        categories: AttributeGroup::GlobalEvent
            .union(AttributeGroup::DocumentEvent),
    },
    OnScroll(Anything<'input>) {
        name: "onscroll",
        categories: AttributeGroup::GlobalEvent
            .union(AttributeGroup::DocumentEvent),
    },
    OnSeeked(Anything<'input>) {
        name: "onseeked",
        categories: AttributeGroup::GlobalEvent,
    },
    OnSeeking(Anything<'input>) {
        name: "onseeking",
        categories: AttributeGroup::GlobalEvent,
    },
    OnSelect(Anything<'input>) {
        name: "onselect",
        categories: AttributeGroup::GlobalEvent,
    },
    OnShow(Anything<'input>) {
        name: "onshow",
        categories: AttributeGroup::GlobalEvent,
    },
    OnStalled(Anything<'input>) {
        name: "onstalled",
        categories: AttributeGroup::GlobalEvent,
    },
    OnSubmit(Anything<'input>) {
        name: "onsubmit",
        categories: AttributeGroup::GlobalEvent,
    },
    OnSuspend(Anything<'input>) {
        name: "onsuspend",
        categories: AttributeGroup::GlobalEvent,
    },
    OnTimeupdate(Anything<'input>) {
        name: "ontimeupdate",
        categories: AttributeGroup::GlobalEvent,
    },
    OnToggle(Anything<'input>) {
        name: "ontoggle",
        categories: AttributeGroup::GlobalEvent,
    },
    OnUnload(Anything<'input>) {
        name: "onunload",
        categories: AttributeGroup::DocumentEvent,
    },
    OnVolumechange(Anything<'input>) {
        name: "onvolumechange",
        categories: AttributeGroup::GlobalEvent,
    },
    OnWaiting(Anything<'input>) {
        name: "onwaiting",
        categories: AttributeGroup::GlobalEvent,
    },
    OnWheel(Anything<'input>) {
        name: "onwheel",
        categories: AttributeGroup::GlobalEvent,
    },
    OnZoom(Anything<'input>) {
        name: "onzoom",
        categories: AttributeGroup::DocumentEvent,
    },
    OperatorFeComposite(OperatorFeComposite) {
        name: "operator",
        default: OperatorFeComposite::Over,
    },
    OperatorFeMorphology(OperatorFeMorphology) {
        name: "operator",
        default: OperatorFeMorphology::Erode,
    },
    Order(NumberOptionalNumber) {
        name: "order",
        default: NumberOptionalNumber(3.0, None),
    },
    Orient(Orient) {
        name: "orient",
        default: Orient::Number(0.0),
    },
    Orientation(Orientation) {
        name: "orientation",
        info: AttributeInfo::DeprecatedUnsafe,
    },
    Origin(Origin) {
        name: "origin",
        default: Origin::Default,
    },
    OverlinePosition(Number) {
        name: "overline-position",
        info: AttributeInfo::DeprecatedUnsafe,
    },
    OverlineThickness(Number) {
        name: "overline-thickness",
        info: AttributeInfo::DeprecatedUnsafe,
    },
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
    PatternContentUnits(Units) {
        name: "patternContentUnits",
        default: Units::UserSpaceOnUse,
    },
    PatternTransform(SVGTransformList) {
        name: "patternTransform",
    },
    PatternUnits(Units) {
        name: "patternUnits",
        default: Units::ObjectBoundingBox,
    },
    Ping(ListOf<Url<'input>, Space>) {
        name: "ping",
    },
    Pitch(LengthPercentage) {
        name: "pitch",
        info: AttributeInfo::DeprecatedUnsafe,
        default: LengthPercentage::px(0.0),
    },
    Playbackorder(Playbackorder) {
        name: "playbackorder",
        default: Playbackorder::All,
    },
    Points(Points) {
        name: "points",
    },
    PointsAtX(Number) {
        name: "pointsAtX",
        default: 0.0,
    },
    PointsAtY(Number) {
        name: "pointsAtY",
        default: 0.0,
    },
    PointsAtZ(Number) {
        name: "pointsAtZ",
        default: 0.0,
    },
    PreserveAlpha(TrueFalse) {
        name: "preserveAlpha",
        default: TrueFalse(false),
    },
    PreserveAspectRatio(PreserveAspectRatio) {
        name: "preserveAspectRatio",
        default: PreserveAspectRatio::default(),
    },
    PrimitiveUnits(Units) {
        name: "primitiveUnits",
        default: Units::UserSpaceOnUse,
    },
    Radius(NumberOptionalNumber) {
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
        info: AttributeInfo::DeprecatedUnsafe,
        default: RenderingIntent::Auto,
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
        info: AttributeInfo::DeprecatedUnsafe,
    },
    Restart(Restart) {
        name: "restart",
        categories: AttributeGroup::AnimationTiming,
        default: Restart::Always,
    },
    Result(Anything<'input>) {
        name: "result",
        categories: AttributeGroup::FilterPrimitive,
    },
    Role(ListOf<Role, Space>) {
        name: "role",
        categories: AttributeGroup::Aria,
    },
    RotateText(ListOf<Number, SpaceOrComma>) {
        name: "rotate",
    },
    RotateAnimate(Rotate) {
        name: "rotate",
        default: Rotate::Number(0.0),
    },
    RotateHatch(Angle) {
        name: "rotate",
        default: Angle::Deg(0.0),
    },
    RX(Radius) {
        name: "rx",
    },
    RY(Radius) {
        name: "ry",
    },
    RGeometry(LengthPercentage) {
        name: "r",
        default: LengthPercentage::px(0.0),
    },
    RRadialGradient(LengthPercentage) {
        name: "r",
        default: LengthPercentage::Percentage(Percentage(50.0)),
    },
    Scale(Number) {
        name: "scale",
        default: 0.0,
    },
    Seed(Number) {
        name: "seed",
        default: 0.0,
    },
    Side(TextPathSide) {
        name: "side",
        default: TextPathSide::Left,
    },
    SlopeFont(Number) {
        name: "slope",
        info: AttributeInfo::DeprecatedUnsafe,
        default: 0.0,
    },
    SlopeFe(Number) {
        name: "slope",
        categories: AttributeGroup::TransferFunction,
        default: 1.0,
    },
    // https://udn.realityripple.com/docs/Web/SVG/Element/solidColor
    SolidColor(Paint<'input>) {
        name: "solid-color",
    },
    // https://udn.realityripple.com/docs/Web/SVG/Element/solidColor
    SolidOpacity(Opacity) {
        name: "solid-opacity",
    },
    Spacing(TextPathSpacing) {
        name: "spacing",
        default: TextPathSpacing::Exact,
    },
    SpecularConstant(Number) {
        name: "specularConstant",
        default: 1.0,
    },
    SpecularExponent(Number) {
        name: "specularExponent",
        default: 1.0,
    },
    SpreadMethod(SpreadMethod) {
        name: "spreadMethod",
        default: SpreadMethod::Pad,
    },
    StartOffset(LengthOrNumber) {
        name: "startOffset",
        default: LengthOrNumber::Number(0.0),
    },
    StdDeviationFeDropShadow(NumberOptionalNumber) {
        name: "stdDeviation",
        default: NumberOptionalNumber(2.0, None),
    },
    StdDeviationFeGaussianBlur(NumberOptionalNumber) {
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
    StitchTiles(StitchTilesFeTurbulence) {
        name: "stitchTiles",
        default: StitchTilesFeTurbulence::NoStitch,
    },
    StrikethroughPosition(Number) {
        name: "strikethrough-position",
        info: AttributeInfo::DeprecatedUnsafe,
    },
    StrikethroughThickness(Number) {
        name: "strikethrough-thickness",
        info: AttributeInfo::DeprecatedUnsafe,
    },
    String(Anything<'input>) {
        name: "string",
        info: AttributeInfo::DeprecatedUnsafe,
    },
    Style(Style<'input>) {
        name: "style",
        categories: AttributeGroup::Core,
    },
    SurfaceScale(Number) {
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
        default: ListOf {
            list: vec![],
            seperator: SpaceOrComma,
        },
    },
    Target(Target<'input>) {
        name: "target",
        default: Target::_Self,
    },
    TargetX(Integer) {
        name: "targetX",
    },
    TargetY(Integer) {
        name: "targetY",
    },
    TextLength(LengthOrNumber) {
        name: "textLength",
    },
    Timelinebegin(Timelinebegin) {
        name: "timelinebegin",
        default: Timelinebegin::Loadend,
    },
    Title(Anything<'input>) {
        name: "title",
    },
    To(Anything<'input>) {
        name: "to",
        categories: AttributeGroup::AnimationValue,
    },
    TypeA(Anything<'input>) {
        name: "type",
    },
    TypeAnimateTransform(TypeAnimateTransform) {
        name: "type",
        default: TypeAnimateTransform::Translate,
    },
    TypeFeColorMatrix(TypeFeColorMatrix) {
        name: "type",
        default: TypeFeColorMatrix::Matrix,
    },
    TypeFeFunc(TransferFunctionType) {
        name: "type",
        categories: AttributeGroup::TransferFunction,
        default: TransferFunctionType::Identity,
    },
    TypeFeTurbulence(TypeFeTurbulence) {
        name: "type",
        default: TypeFeTurbulence::Turbulence,
    },
    TypeScript(MediaType<'input>) {
        name: "type",
        default: MediaType::Static("application/ecmascript"),
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
        info: AttributeInfo::DeprecatedUnsafe,
    },
    UnderlineThickness(Number) {
        name: "underline-thickness",
        info: AttributeInfo::DeprecatedUnsafe,
    },
    Unicode(Anything<'input>) {
        name: "unicode",
        info: AttributeInfo::DeprecatedUnsafe,
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
    VHanging(Number) {
        name: "v-hanging",
        info: AttributeInfo::DeprecatedUnsafe,
    },
    VIdeographic(Number) {
        name: "v-ideographic",
        info: AttributeInfo::DeprecatedUnsafe,
    },
    VMathematical(Number) {
        name: "v-mathematical",
        info: AttributeInfo::DeprecatedUnsafe,
    },
    ValuesFeColorMatrix(ListOf<Number, SpaceOrComma>) {
        name: "values",
    },
    ValuesAnimate(ListOf<Anything<'input>, Semicolon>) {
        name: "values",
        categories: AttributeGroup::AnimationValue,
    },
    // NOTE: Spec says `<number>` -- but decimal should be present (e.g. "1.0")
    Version(Anything<'input>) {
        name: "version",
        info: AttributeInfo::DeprecatedSafe,
        default: Anything::Static("1.1"),
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
    ViewBox(ViewBox) {
        name: "viewBox",
    },
    ViewTarget(Anything<'input>) {
        name: "viewTarget",
        info: AttributeInfo::DeprecatedUnsafe,
    },
    WidthFilter(LengthPercentage) {
        name: "width",
        default: LengthPercentage::Percentage(Percentage(120.0)),
    },
    WidthForeignObject(LengthPercentage) {
        name: "width",
    },
    WidthImage(LengthPercentage) {
        name: "width",
    },
    WidthPattern(LengthPercentage) {
        name: "width",
        default: LengthPercentage::Percentage(Percentage(0.0)),
    },
    WidthRect(LengthPercentage) {
        name: "width",
    },
    WidthSvg(LengthPercentage) {
        name: "width",
        default: LengthPercentage::Percentage(Percentage(100.0)),
    },
    // NOTE: Missing from index (https://github.com/w3c/svgwg/issues/1027)
    WidthSymbol(LengthPercentage) {
        name: "width",
    },
    WidthUse(LengthPercentage) {
        name: "width",
    },
    WidthFe(LengthPercentage) {
        name: "width",
        categories: AttributeGroup::FilterPrimitive,
    },
    WidthMask(LengthPercentage) {
        name: "width",
        default: LengthPercentage::Percentage(Percentage(120.0)),
    },
    Widths(Anything<'input>) {
        name: "widths",
        info: AttributeInfo::DeprecatedUnsafe,
    },
    XAltGlyph(ListOf<LengthPercentage, SpaceOrComma>) {
        name: "x",
        info: AttributeInfo::DeprecatedUnsafe,
    },
    XCursor(LengthPercentage) {
        name: "x",
        info: AttributeInfo::DeprecatedUnsafe,
        default: LengthPercentage::px(0.0),
    },
    XFe(LengthPercentage) {
        name: "x",
        categories: AttributeGroup::FilterPrimitive,
        default: LengthPercentage::Percentage(Percentage(0.0)),
    },
    XFePointLight(Number) {
        name: "x",
        default: 0.0,
    },
    XFeSpotLight(Number) {
        name: "x",
        default: 0.0,
    },
    XFilter(LengthPercentage) {
        name: "x",
        default: LengthPercentage::Percentage(Percentage(-10.0)),
    },
    XGeometry(LengthPercentage) {
        name: "x",
        default: LengthPercentage::px(0.0),
    },
    XGlyphRef(Number) {
        name: "x",
        info: AttributeInfo::DeprecatedUnsafe,
    },
    XHatch(LengthPercentage) {
        name: "x",
        info: AttributeInfo::DeprecatedUnsafe,
        default: LengthPercentage::Percentage(Percentage(0.0)),
    },
    XMask(LengthPercentage) {
        name: "x",
        default: LengthPercentage::Percentage(Percentage(-10.0)),
    },
    XPattern(LengthPercentage) {
        name: "x",
        default: LengthPercentage::px(0.0),
    },
    XText(ListOf<LengthPercentage, SpaceOrComma>) {
        name: "x",
        default: ListOf {
            list: vec![LengthPercentage::px(0.0)],
            seperator: SpaceOrComma,
        },
    },
    XTRef(ListOf<LengthPercentage, SpaceOrComma>) {
        name: "x",
    },
    XHeight(Number) {
        name: "x-height",
        info: AttributeInfo::DeprecatedUnsafe,
    },
    X1Line(LengthPercentage) {
        name: "x1",
        default: LengthPercentage::px(0.0),
    },
    X1LinearGradient(LengthPercentage) {
        name: "x1",
        default: LengthPercentage::Percentage(Percentage(0.0)),
    },
    X2Line(LengthPercentage) {
        name: "x2",
        default: LengthPercentage::px(0.0),
    },
    X2LinearGradient(LengthPercentage) {
        name: "x2",
        default: LengthPercentage::Percentage(Percentage(100.0)),
    },
    XChannelSelector(ChannelSelector) {
        name: "xChannelSelector",
        default: ChannelSelector::default(),
    },
    XLinkActuate(XLinkActuate) {
        prefix: XLink,
        name: "actuate",
        categories: AttributeGroup::XLink,
    },
    XLinkArcrole(Url<'input>) {
        prefix: XLink,
        name: "arcrole",
        categories: AttributeGroup::XLink,
    },
    XLinkHref(Url<'input>) {
        prefix: XLink,
        name: "href",
        categories: AttributeGroup::XLink,
    },
    XLinkRole(Url<'input>) {
        prefix: XLink,
        name: "role",
        categories: AttributeGroup::XLink,
    },
    XLinkTitle(Anything<'input>) {
        prefix: XLink,
        name: "title",
        categories: AttributeGroup::XLink,
    },
    XLinkType(XLinkType) {
        prefix: XLink,
        name: "type",
        categories: AttributeGroup::XLink,
    },
    XLinkShow(XLinkShow) {
        prefix: XLink,
        name: "show",
        categories: AttributeGroup::XLink,
    },
    XMLNS(Anything<'input>) {
        name: "xmlns",
        categories: AttributeGroup::Core,
    },
    XmlBase(Anything<'input>) {
        prefix: XML,
        name: "base",
        categories: AttributeGroup::Core,
        info: AttributeInfo::DeprecatedUnsafe,
    },
    XmlLang(Anything<'input>) {
        prefix: XML,
        name: "lang",
        categories: AttributeGroup::Core,
        info: AttributeInfo::DeprecatedUnsafe,
    },
    XmlSpace(XmlSpace) {
        prefix: XML,
        name: "space",
        categories: AttributeGroup::Core,
        info: AttributeInfo::DeprecatedUnsafe,
    },
    YAltGlyph(ListOf<LengthPercentage, SpaceOrComma>) {
        name: "y",
        info: AttributeInfo::DeprecatedUnsafe,
    },
    YCursor(LengthPercentage) {
        name: "y",
        info: AttributeInfo::DeprecatedUnsafe,
        default: LengthPercentage::px(0.0),
    },
    YFe(LengthPercentage) {
        name: "y",
        categories: AttributeGroup::FilterPrimitive,
        default: LengthPercentage::Percentage(Percentage(0.0)),
    },
    YFePointLight(Number) {
        name: "y",
        default: 0.0,
    },
    YFeSpotLight(Number) {
        name: "y",
        default: 0.0,
    },
    YFilter(LengthPercentage) {
        name: "y",
        default: LengthPercentage::Percentage(Percentage(-10.0)),
    },
    YGeometry(LengthPercentage) {
        name: "y",
        default: LengthPercentage::px(0.0),
    },
    YGlyphRef(Number) {
        name: "y",
        info: AttributeInfo::DeprecatedUnsafe,
    },
    YHatch(LengthPercentage) {
        name: "y",
        info: AttributeInfo::DeprecatedUnsafe,
        default: LengthPercentage::Percentage(Percentage(0.0)),
    },
    YMask(LengthPercentage) {
        name: "y",
        default: LengthPercentage::Percentage(Percentage(-10.0)),
    },
    YPattern(LengthPercentage) {
        name: "y",
        default: LengthPercentage::px(0.0),
    },
    YText(ListOf<LengthPercentage, SpaceOrComma>) {
        name: "y",
        default: ListOf {
            list: vec![LengthPercentage::px(0.0)],
            seperator: SpaceOrComma,
        },
    },
    YTRef(ListOf<LengthPercentage, SpaceOrComma>) {
        name: "y",
    },
    Y1Line(LengthPercentage) {
        name: "y1",
        default: LengthPercentage::Percentage(Percentage(0.0)),
    },
    Y1LinearGradient(LengthPercentage) {
        name: "y1",
        default: LengthPercentage::Percentage(Percentage(-10.0)),
    },
    Y2Line(LengthPercentage) {
        name: "y2",
        default: LengthPercentage::Percentage(Percentage(0.0)),
    },
    Y2LinearGradient(LengthPercentage) {
        name: "y2",
        default: LengthPercentage::Percentage(Percentage(0.0)),
    },
    YChannelSelector(ChannelSelector) {
        name: "yChannelSelector",
        default: ChannelSelector::A,
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

    // Presentation Attributes
    AlignmentBaseline(Inheritable<AlignmentBaseline>) {
        name: "alignment-baseline",
        categories: AttributeGroup::Presentation,
    },
    BaselineShift(Inheritable<BaselineShift>) {
        name: "baseline-shift",
        categories: AttributeGroup::Presentation,
    },
    Clip(Inheritable<Clip>) {
        name: "clip",
        categories: AttributeGroup::Presentation,
        default: Inheritable::Defined(Clip::Auto),
    },
    ClipPath(Inheritable<ClipPath<'input> >) {
        name: "clip-path",
        categories: AttributeGroup::Presentation,
        info: AttributeInfo::PresentationNonInheritableGroupAttrs,
        default: Inheritable::Defined(ClipPath::None),
    },
    ClipRule(Inheritable<FillRule>) {
        name: "clip-rule",
        categories: AttributeGroup::Presentation,
        info: AttributeInfo::Inheritable,
        default: Inheritable::Defined(FillRule::Nonzero),
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
        default: Inheritable::Defined(ColorInterpolation::SRGB),
    },
    ColorInterpolationFilters(Inheritable<ColorInterpolation>) {
        name: "color-interpolation-filters",
        categories: AttributeGroup::Presentation,
        info: AttributeInfo::Inheritable,
        default: Inheritable::Defined(ColorInterpolation::LinearRGB),
    },
    ColorProfile(Inheritable<ColorProfile<'input> >) {
        name: "color-profile",
        categories: AttributeGroup::Presentation,
        info: AttributeInfo::DeprecatedUnsafe
            .union(AttributeInfo::Inheritable),
        default: Inheritable::Defined(ColorProfile::Auto),
    },
    ColorRendering(Inheritable<ColorRendering>) {
        name: "color-rendering",
        categories: AttributeGroup::Presentation,
        info: AttributeInfo::DeprecatedUnsafe
            .union(AttributeInfo::Inheritable),
        default: Inheritable::Defined(ColorRendering::Auto),
    },
    Cursor(Inheritable<Cursor<'input> >) {
        name: "cursor",
        categories: AttributeGroup::Presentation,
        info: AttributeInfo::Inheritable,
        default: Inheritable::Defined(Cursor {
            images: SmallVec::default(),
            keyword: CursorKeyword::Auto,
        }),
    },
    Direction(Inheritable<Direction>) {
        name: "direction",
        categories: AttributeGroup::Presentation,
        info: AttributeInfo::Inheritable,
    },
    Display(Inheritable<Display>) {
        name: "display",
        categories: AttributeGroup::Presentation,
        info: AttributeInfo::PresentationNonInheritableGroupAttrs,
    },
    DominantBaseline(Inheritable<DominantBaseline>) {
        name: "dominant-baseline",
        categories: AttributeGroup::Presentation,
        info: AttributeInfo::Inheritable,
        default: Inheritable::Defined(DominantBaseline::Auto),
    },
    EnableBackground(Inheritable<EnableBackground>) {
        name: "enable-background",
        categories: AttributeGroup::Presentation,
        info: AttributeInfo::DeprecatedUnsafe,
    },
    Fill(Inheritable<Paint<'input> >) {
        name: "fill",
        categories: AttributeGroup::Presentation,
        info: AttributeInfo::Inheritable,
    },
    FillOpacity(Inheritable<Opacity>) {
        name: "fill-opacity",
        categories: AttributeGroup::Presentation,
        info: AttributeInfo::Inheritable,
        default: Inheritable::Defined(AlphaValue(1.0)),
    },
    FillRule(Inheritable<FillRule>) {
        name: "fill-rule",
        categories: AttributeGroup::Presentation,
        info: AttributeInfo::Inheritable,
        default: Inheritable::Defined(FillRule::Nonzero),
    },
    Filter(Inheritable<FilterList<'input> >) {
        name: "filter",
        categories: AttributeGroup::Presentation,
        info: AttributeInfo::PresentationNonInheritableGroupAttrs,
    },
    FloodColor(Inheritable<Color>) {
        name: "flood-color",
        categories: AttributeGroup::Presentation,
    },
    FloodOpacity(Inheritable<Opacity>) {
        name: "flood-opacity",
        categories: AttributeGroup::Presentation,
        default: Inheritable::Defined(AlphaValue(1.0)),
    },
    Font(Inheritable<Font<'input> >) {
        // NOTE: This isn't in the spec but is referenced by SVGO
        name: "font",
        categories: AttributeGroup::Presentation,
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
        default: Inheritable::Defined(FontSize::Absolute(AbsoluteFontSize::Medium)),
    },
    FontSizeAdjust(Inheritable<FontSizeAdjust>) {
        name: "font-size-adjust",
        categories: AttributeGroup::Presentation,
        info: AttributeInfo::Inheritable,
        default: Inheritable::Defined(FontSizeAdjust::None),
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
    GlyphOrientationHorizontal(Inheritable<Angle>) {
        name: "glyph-orientation-horizontal",
        categories: AttributeGroup::Presentation,
        info: AttributeInfo::DeprecatedUnsafe
            .union(AttributeInfo::Inheritable),
        default: Inheritable::Defined(Angle::Deg(0.0)),
    },
    GlyphOrientationVertical(Inheritable<GlyphOrientationVertical>) {
        name: "glyph-orientation-vertical",
        categories: AttributeGroup::Presentation,
        info: AttributeInfo::DeprecatedUnsafe
            .union(AttributeInfo::Inheritable),
    },
    ImageRendering(Inheritable<ImageRendering>) {
        name: "image-rendering",
        categories: AttributeGroup::Presentation,
        info: AttributeInfo::Inheritable,
        default: Inheritable::Defined(ImageRendering::Auto),
    },
    Kerning(Inheritable<Kerning>) {
        name: "kerning",
        categories: AttributeGroup::Presentation,
        info: AttributeInfo::DeprecatedUnsafe,
        default: Inheritable::Defined(Kerning::Auto),
    },
    LetterSpacing(Inheritable<Spacing>) {
        name: "letter-spacing",
        categories: AttributeGroup::Presentation,
        info: AttributeInfo::Inheritable,
        default: Inheritable::Defined(Spacing::Normal),
    },
    LightingColor(Inheritable<Color>) {
        name: "lighting-color",
        categories: AttributeGroup::Presentation,
    },
    MarkerEnd(Inheritable<Marker<'input> >) {
        name: "marker-end",
        categories: AttributeGroup::Presentation,
        info: AttributeInfo::Inheritable,
        default: Inheritable::Defined(Marker::None),
    },
    MarkerMid(Inheritable<Marker<'input> >) {
        name: "marker-mid",
        categories: AttributeGroup::Presentation,
        info: AttributeInfo::Inheritable,
        default: Inheritable::Defined(Marker::None),
    },
    MarkerStart(Inheritable<Marker<'input> >) {
        name: "marker-start",
        categories: AttributeGroup::Presentation,
        info: AttributeInfo::Inheritable,
        default: Inheritable::Defined(Marker::None),
    },
    Marker(Inheritable<Marker<'input> >) {
        name: "marker",
        categories: AttributeGroup::Presentation,
        info: AttributeInfo::Inheritable,
    },
    Mask(Inheritable<Mask<'input> >) {
        name: "mask",
        categories: AttributeGroup::Presentation,
        info: AttributeInfo::PresentationNonInheritableGroupAttrs,
    },
    // TODO: Add when atoms included in xml5ever::LocalNameStaticSet
    // MaskType(MaskType) {
    //     name: "mask-type",
    //     categories: AttributeGroup::Presentation,
    //     default: MaskType::Luminance,
    // },
    Opacity(Inheritable<Opacity>) {
        name: "opacity",
        categories: AttributeGroup::Presentation,
        info: AttributeInfo::PresentationNonInheritableGroupAttrs,
        default: Inheritable::Defined(AlphaValue(1.0)),
    },
    Overflow(Inheritable<Overflow>) {
        name: "overflow",
        categories: AttributeGroup::Presentation,
    },
    PaintOrder(Inheritable<PaintOrder>) {
        name: "paint-order",
        categories: AttributeGroup::Presentation,
        info: AttributeInfo::Inheritable,
        default: Inheritable::Defined(PaintOrder::normal()),
    },
    PointerEvents(Inheritable<PointerEvents>) {
        name: "pointer-events",
        categories: AttributeGroup::Presentation,
        info: AttributeInfo::Inheritable,
        default: Inheritable::Defined(PointerEvents::VisiblePainted),
    },
    ShapeRendering(Inheritable<ShapeRendering>) {
        name: "shape-rendering",
        categories: AttributeGroup::Presentation,
        info: AttributeInfo::Inheritable,
        default: Inheritable::Defined(ShapeRendering::Auto),
    },
    StopColor(Inheritable<Color>) {
        name: "stop-color",
        categories: AttributeGroup::Presentation,
    },
    StopOpacity(Inheritable<Opacity>) {
        name: "stop-opacity",
        categories: AttributeGroup::Presentation,
        default: Inheritable::Defined(AlphaValue(1.0)),
    },
    Stroke(Inheritable<Paint<'input> >) {
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
        default: Inheritable::Defined(LengthPercentage::px(0.0)),
    },
    StrokeLinecap(Inheritable<StrokeLinecap>) {
        name: "stroke-linecap",
        categories: AttributeGroup::Presentation,
        info: AttributeInfo::Inheritable,
        default: Inheritable::Defined(StrokeLinecap::Butt),
    },
    StrokeLinejoin(Inheritable<StrokeLinejoin>) {
        name: "stroke-linejoin",
        categories: AttributeGroup::Presentation,
        info: AttributeInfo::Inheritable,
        default: Inheritable::Defined(StrokeLinejoin::Miter),
    },
    StrokeMiterlimit(Inheritable<Number>) {
        name: "stroke-miterlimit",
        categories: AttributeGroup::Presentation,
        info: AttributeInfo::Inheritable,
        default: Inheritable::Defined(4.0),
    },
    StrokeOpacity(Inheritable<Opacity>) {
        name: "stroke-opacity",
        categories: AttributeGroup::Presentation,
        info: AttributeInfo::Inheritable,
        default: Inheritable::Defined(AlphaValue(1.0)),
    },
    StrokeWidth(Inheritable<LengthPercentage>) {
        name: "stroke-width",
        categories: AttributeGroup::Presentation,
        info: AttributeInfo::Inheritable,
        default: Inheritable::Defined(LengthPercentage::px(1.0)),
    },
    TextAnchor(Inheritable<TextAnchor>) {
        name: "text-anchor",
        categories: AttributeGroup::Presentation,
        info: AttributeInfo::Inheritable,
        default: Inheritable::Defined(TextAnchor::Start),
    },
    TextDecoration(Inheritable<TextDecoration>) {
        name: "text-decoration",
        categories: AttributeGroup::Presentation,
        info: AttributeInfo::PresentationNonInheritableGroupAttrs,
    },
    TextRendering(Inheritable<TextRendering>) {
        name: "text-rendering",
        categories: AttributeGroup::Presentation,
        info: AttributeInfo::Inheritable,
        default: Inheritable::Defined(TextRendering::Auto),
    },
    Transform(Inheritable<SVGTransformList>) {
        name: "transform",
        categories: AttributeGroup::Presentation,
        info: AttributeInfo::PresentationNonInheritableGroupAttrs,
    },
    TransformOrigin(Position) {
        name: "transform-origin",
        categories: AttributeGroup::Presentation,
        default: Position {
            x: HorizontalPosition::Length(DimensionPercentage::Percentage(Percentage(50.0))),
            y: VerticalPosition::Length(DimensionPercentage::Percentage(Percentage(50.0))),
        },
    },
    UnicodeBidi(Inheritable<UnicodeBidi>) {
        name: "unicode-bidi",
        categories: AttributeGroup::Presentation,
        info: AttributeInfo::PresentationNonInheritableGroupAttrs,
        default: Inheritable::Defined(UnicodeBidi::Normal),
    },
    VectorEffect(VectorEffect) {
        name: "vector-effect",
        categories: AttributeGroup::Presentation,
        default: VectorEffect::None,
    },
    Visibility(Inheritable<Visibility>) {
        name: "visibility",
        categories: AttributeGroup::Presentation,
        info: AttributeInfo::Inheritable,
        default: Inheritable::Defined(Visibility::Visible),
    },
    WordSpacing(Inheritable<Spacing>) {
        name: "word-spacing",
        categories: AttributeGroup::Presentation,
        info: AttributeInfo::Inheritable,
        default: Inheritable::Defined(Spacing::Normal),
    },
    WritingMode(Inheritable<WritingMode>) {
        name: "writing-mode",
        categories: AttributeGroup::Presentation,
        info: AttributeInfo::Inheritable,
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
    Fill(value) => Inheritable::Defined(value) => value.option().ok_or(())?,
    FillOpacity(value) => Inheritable::Defined(value) => value.option().ok_or(())?,
    FillRule(value) => Inheritable::Defined(value) => value.option().ok_or(())?,
    Filter(value, vp) => Inheritable::Defined(value) => value.option().ok_or(())?,
    Font(value) => Inheritable::Defined(value) => value.option().ok_or(())?,
    FontFamily(value) => Inheritable::Defined(FontFamily(ListOf {
        list: value,
        seperator: SpaceOrComma,
    })) => value.option().ok_or(())?.0.list,
    FontSize(value) => Inheritable::Defined(value) => value.option().ok_or(())?,
    FontStretch(value) => Inheritable::Defined(value) => value.option().ok_or(())?,
    FontStyle(value) => Inheritable::Defined(value) => value.option().ok_or(())?,
    FontWeight(value) => Inheritable::Defined(value) => value.option().ok_or(())?,
    ImageRendering(value) => Inheritable::Defined(value) => value.option().ok_or(())?,
    LetterSpacing(value) => Inheritable::Defined(value) => value.option().ok_or(())?,
    MarkerEnd(value) => Inheritable::Defined(value) => value.option().ok_or(())?,
    MarkerMid(value) => Inheritable::Defined(value) => value.option().ok_or(())?,
    MarkerStart(value) => Inheritable::Defined(value) => value.option().ok_or(())?,
    Marker(value) => Inheritable::Defined(value) => value.option().ok_or(())?,
    Mask(value, vp) => Inheritable::Defined(Mask(ListOf {
        list: value.to_vec(),
        seperator: SpaceOrComma,
    })) => value.option().ok_or(())?.0.list.into(),
    Opacity(value) => Inheritable::Defined(value) => value.option().ok_or(())?,
    Overflow(value) => Inheritable::Defined(value) => value.option().ok_or(())?,
    ShapeRendering(value) => Inheritable::Defined(value) => value.option().ok_or(())?,
    Stroke(value) => Inheritable::Defined(value) => value.option().ok_or(())?,
    StrokeDasharray(value) => Inheritable::Defined(value) => value.option().ok_or(())?,
    StrokeDashoffset(value) => Inheritable::Defined(LengthPercentage(value)) => value.option().ok_or(())?.0,
    StrokeLinecap(value) => Inheritable::Defined(value) => value.option().ok_or(())?,
    StrokeLinejoin(value) => Inheritable::Defined(value) => value.option().ok_or(())?,
    StrokeMiterlimit(value) => Inheritable::Defined(value) => value.option().ok_or(())?,
    StrokeOpacity(value) => Inheritable::Defined(value) => value.option().ok_or(())?,
    StrokeWidth(value) => Inheritable::Defined(LengthPercentage(value)) => value.option().ok_or(())?.0,
    TextDecoration(value, vp) => Inheritable::Defined(value) => value.option().ok_or(())?,
    TextRendering(value) => Inheritable::Defined(value) => value.option().ok_or(())?,
    Transform(value, vp) => Inheritable::Defined((&value).try_into()?) => value.option().map(Into::into).ok_or(())?,
    TransformOrigin(value, vp) => value => value,
    UnicodeBidi(value) => Inheritable::Defined(value) => value.option().ok_or(())?,
    Visibility(value) => Inheritable::Defined(value) => value.option().ok_or(())?,
    WordSpacing(value) => Inheritable::Defined(value) => value.option().ok_or(())?,
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

#[cfg(not(feature = "markup5ever"))]
static ATTRIBUTE_GROUP_ATTRIBUTES_MEMO: std::sync::LazyLock<
    DashMap<AttributeGroup, &'static [&'static AttrId<'static>]>,
> = std::sync::LazyLock::new(DashMap::new);
#[cfg(feature = "markup5ever")]
thread_local! {
    static ATTRIBUTE_GROUP_ATTRIBUTES_MEMO: DashMap<AttributeGroup, &'static [&'static AttrId<'static>]> = DashMap::new();
}

impl AttributeGroup {
    /// Returns the list of attributes associated with the attribute group
    ///
    /// # Panics
    ///
    /// If internal memoisation fails to memoise
    pub fn attributes(&self) -> &'static [&'static AttrId<'static>] {
        #[cfg(not(feature = "markup5ever"))]
        if let Some(attributes) = ATTRIBUTE_GROUP_ATTRIBUTES_MEMO.get(self) {
            return attributes.value();
        }
        #[cfg(feature = "markup5ever")]
        if let Some(attributes) =
            ATTRIBUTE_GROUP_ATTRIBUTES_MEMO.with(|memo| memo.get(self).map(|a| *a.value()))
        {
            return attributes;
        }
        let attributes = _attr_id::all
            .iter()
            .filter(|attr| self.intersects(attr.attribute_group()))
            .copied()
            .collect();
        let attributes: &'static [_] = Vec::leak(attributes);
        #[cfg(not(feature = "markup5ever"))]
        ATTRIBUTE_GROUP_ATTRIBUTES_MEMO.insert(*self, attributes);
        #[cfg(feature = "markup5ever")]
        ATTRIBUTE_GROUP_ATTRIBUTES_MEMO.with(|memo| {
            let result = memo.insert(*self, attributes);
            debug_assert!(result.is_none());
        });
        self.attributes()
    }

    /// Returns an `AttrId` that matches the attribute groups.
    /// Returns an unknown `AttrId` otherwise
    pub fn parse_attr_id<'input>(
        &self,
        prefix: &Prefix<'input>,
        local: Atom<'input>,
    ) -> AttrId<'input> {
        self.attributes()
            .iter()
            .find(|attr| attr.prefix() == prefix && *attr.local_name() == local)
            .map_or_else(
                || {
                    AttrId::Unknown(QualName {
                        prefix: prefix.clone(),
                        local,
                    })
                },
                |attr| {
                    if prefix.is_aliased() {
                        AttrId::Aliased {
                            prefix: prefix.clone(),
                            attr_id: Box::new((*attr).clone()),
                        }
                    } else {
                        (*attr).clone()
                    }
                },
            )
    }
}
