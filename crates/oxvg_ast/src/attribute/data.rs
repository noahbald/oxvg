use animation::{BeginEnd, CalcMode, ControlPoint};
use animation_addition::{Accumulate, Additive};
use animation_timing::{Dur, Fill, MinMax, RepeatCount, RepeatDur, Restart};
use aria::{
    AriaAutocomplete, AriaCurrent, AriaDropEffect, AriaHasPopup, AriaInvalid, AriaLive,
    AriaOrientation, AriaRelevant, AriaSort, IDReference, Role, Tristate,
};
use core::{
    Angle, Anything, Boolean, ClockValue, Color, Coordinate, Frequency, FuncIRI, Integer, Length,
    Name, Number, NumberOptionalNumber, Opacity, Paint, Percentage, Style, Time, TransformList,
    Url, IRI,
};
use filter_effect::{
    ChannelSelector, FeColorMatrixType, FeCompositeOperator, FeEdgeMode, FeOperator,
    FeTurbulenceStitchTiles, FeTurbulenceType, In,
};
use inheritable::Inheritable;
use list_of::{Comma, ListOf, Semicolon, Seperator, Seperators, Space, SpaceOrComma};
use path::Path;
use presentation::{
    AlignmentBaseline, BaselineShift, Clip, ClipPath, ColorInterpolation, ColorProfile, Cursor,
    Direction, Display, DominantBaseline, EnableBackground, FillRule, FilterList, Font, FontFamily,
    FontSize, FontStretch, FontStyle, FontVariant, FontWeight, LengthOrNumber, LengthPercentage,
    Marker, Mask, Overflow, PaintOrder, PointerEvents, Position, Rendering, ShapeRendering,
    Spacing, StrokeDasharray, StrokeLinecap, StrokeLinejoin, TextAnchor, TextDecoration,
    UnicodeBidi, VectorEffect, Visibility, WritingMode,
};
use transfer_function::TransferFunctionType;
use uncategorised::{
    BlendMode, CrossOrigin, LengthAdjust, LinkType, MediaQueryList, MediaType, NumberPercentage,
    Orient, Origin, PreserveAspectRatio, Radius, RefX, RefY, ReferrerPolicy, Rotate, SpreadMethod,
    Target, TextPathMethod, TextPathSide, TextPathSpacing, Transform, TrueFalse,
    TrueFalseUndefined, TypeAnimateTransform, Units, ViewBox,
};
use xml::XmlSpace;

use crate::{
    atom::Atom,
    attribute::AttributeGroup,
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
pub mod inheritable;
pub mod list_of;
pub mod path;
pub mod presentation;
pub mod transfer_function;
pub mod transform;
pub mod uncategorised;
pub mod xml;

/// An attribute's group.
type C = AttributeGroup;

#[macro_export]
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

macro_rules! define_content_types {
    ($($name:ident($ty:ty)$(<$i:lifetime>)?,)+) => {
        #[derive(Clone, Debug, PartialEq)]
        pub enum ContentType<'a, 'input> {
            $($name(&'a $ty)$(<$i>)?,)+
            Inheritable(Inheritable<Box<ContentType<'a, 'input>>>),
            ListOf(ListOf<Box<ContentType<'a, 'input>>, Seperators>),
        }

        pub enum ContentTypeId {
            $($name,)+
            ListOf(Box<ContentTypeId>, Seperators),
            Inheritable(Box<ContentTypeId>),
        }

        impl std::fmt::Display for ContentType<'_, '_> {
            fn fmt(&self, _f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                todo!()
            }
        }

        impl ToAtom for ContentType<'_, '_> {
            fn write_atom<W>(&self, dest: &mut crate::serialize::Printer<W>) -> Result<(), crate::error::PrinterError>
                where
                    W: std::fmt::Write {
                match self {
                    $(Self::$name(value) => value.write_atom(dest),)+
                    Self::Inheritable(value) => value.write_atom(dest),
                    Self::ListOf(value) => value.write_atom(dest),
                }
            }
        }
    };
}
define_content_types! {
    Angle(Angle),
    Anything(Anything<'input>),
    Boolean(Boolean<'input>),
    ClockValue(ClockValue),
    Color(Color),
    Coordinate(Coordinate),
    Frequency(Frequency),
    FuncIRI(FuncIRI<'input>),
    Integer(Integer),
    IRI(IRI<'input>),
    Length(Length),
    Name(Name<'input>),
    Number(Number),
    NumberOptionalNumber(NumberOptionalNumber),
    Opacity(Opacity),
    Paint(Paint<'input>),
    Percentage(Percentage),
    Style(Style<'input>),
    Time(Time),
    TransformList(TransformList),
    Url(Url<'input>),

    // ARIA specific
    AriaAutocomplete(AriaAutocomplete),
    AriaCurrent(AriaCurrent),
    AriaDropEffect(AriaDropEffect),
    AriaHasPopup(AriaHasPopup),
    AriaInvalid(AriaInvalid),
    AriaLive(AriaLive),
    AriaOrientation(AriaOrientation),
    AriaRelevant(AriaRelevant),
    AriaSort(AriaSort),
    IDReference(IDReference<'input>),
    Role(Role),
    Tristate(Tristate),

    // CSS/Presentation values
    // https://www.w3.org/TR/2011/REC-SVG11-20110816/propidx.html
    AlignmentBaseline(AlignmentBaseline),
    BaselineShift(BaselineShift),
    Clip(Clip),
    ClipPath(ClipPath<'input>),
    ColorInterpolation(ColorInterpolation),
    ColorInterpolationFilters(ColorInterpolation),
    ColorProfile(ColorProfile<'input>),
    Rendering(Rendering),
    Cursor(Cursor<'input>),
    Direction(Direction),
    Display(Display),
    DominantBaseline(DominantBaseline),
    EnableBackground(EnableBackground),
    FillRule(FillRule),
    FilterList(FilterList<'input>),
    Font(Font<'input>),
    FontFamily(FontFamily<'input>),
    FontSize(FontSize),
    FontStretch(FontStretch),
    FontStyle(FontStyle),
    FontVariant(FontVariant),
    FontWeight(FontWeight),
    LengthPercentage(LengthPercentage),
    LengthOrNumber(LengthOrNumber),
    Marker(Marker<'input>),
    Mask(Mask<'input>),
    Overflow(Overflow),
    PaintOrder(PaintOrder),
    PointerEvents(PointerEvents),
    Position(Position),
    ShapeRendering(ShapeRendering),
    Spacing(Spacing),
    StrokeDasharray(StrokeDasharray),
    StrokeLinecap(StrokeLinecap),
    StrokeLinejoin(StrokeLinejoin),
    TextAnchor(TextAnchor),
    TextDecoration(TextDecoration),
    UnicodeBidi(UnicodeBidi),
    VectorEffect(VectorEffect),
    Visibility(Visibility),
    WritingMode(WritingMode),

    // Attr specific
    Accumulate(Accumulate),
    Additive(Additive),
    BeginEnd(BeginEnd<'input>),
    BlendMode(BlendMode),
    CalcMode(CalcMode),
    ChannelSelector(ChannelSelector),
    ControlPoint(ControlPoint),
    CrossOrigin(CrossOrigin),
    Dur(Dur),
    FeColorMatrixType(FeColorMatrixType),
    FeCompositeOperator(FeCompositeOperator),
    FeEdgeMode(FeEdgeMode),
    FeOperator(FeOperator),
    FeTurbulenceStitchTiles(FeTurbulenceStitchTiles),
    FeTurbulenceType(FeTurbulenceType),
    Fill(Fill),
    Units(Units),
    In(In<'input>),
    LengthAdjust(LengthAdjust),
    LinkType(LinkType),
    MediaType(MediaType<'input>),
    MediaQueryList(MediaQueryList<'input>),
    MinMax(MinMax),
    NumberPercentage(NumberPercentage),
    Orient(Orient),
    Origin(Origin),
    Path(Path),
    PreserveAspectRatio(PreserveAspectRatio),
    Radius(Radius),
    RefX(RefX),
    RefY(RefY),
    ReferrerPolicy(ReferrerPolicy),
    RepeatCount(RepeatCount),
    RepeatDur(RepeatDur),
    Restart(Restart),
    Rotate(Rotate),
    SpreadMethod(SpreadMethod),
    Target(Target<'input>),
    TextPathMethod(TextPathMethod),
    TextPathSpacing(TextPathSpacing),
    TextPathSide(TextPathSide),
    TransferFunctionType(TransferFunctionType),
    Transform(Transform),
    TrueFalse(TrueFalse),
    TrueFalseUndefined(TrueFalseUndefined),
    TypeAnimateTransform(TypeAnimateTransform),
    ViewBox(ViewBox),
    XmlSpace(XmlSpace),
}

macro_rules! wrap_content_type {
    ($outer:ident($value:ident)) => {
        ContentType::$outer($value)
    };
    ($outer:ident<$inner:ident$(, $and:ident)?>($value:ident)) => {
        ContentType::$outer(
            $value
                .map(|x| { Box::new(ContentType::$inner(x)) })
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
        name: $name:literal,
        $(categories: $categories:expr,)?
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
        mod _qual_name {
            use crate::name::{Prefix, QualName};
            use crate::atom::Atom;
            $(pub const $attr: &'static QualName<'static> = &QualName {
                prefix: prefix_else!($($prefix)?),
                local: Atom::Static($name),
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
        }

        #[derive(Eq, Clone, Debug, Hash)]
        pub enum AttrId<'input> {
            $($attr,)+
            Aliased {
                prefix: Prefix<'input>,
                attr_id: Box<AttrId<'input>>,
            },
            Unknown(QualName<'input>),
        }

        #[derive(PartialEq, Debug, Clone)]
        /// Represents one of an element's attributes.
        ///
        /// [MDN | Attr](https://developer.mozilla.org/en-US/docs/Web/API/Attr)
        pub enum Attr<'input> {
            $($attr($outer$(<$outer_lt>)?$(<
                $inner$(<$inner_lt>)?
                $(, $and)?
            >)?),)+
            // $($attr($outer$(<$outer_lt>)?$(<$inner$(<$inner_lt>)?$(, $sep)?>)?),)+
            /// A known attribute aliased by a different prefix
            Aliased {
                prefix: Prefix<'input>,
                value: Box<Attr<'input>>,
            },
            /// An attribute with an unknown name or invalid value
            Unparsed {
                attr_id: AttrId<'input>,
                value: Atom<'input>,
            },
        }

        impl<'input> AttrId<'input> {
            /// Creates a new attribute
            pub fn new(prefix: Prefix<'input>, local: Atom<'input>) -> Self {
                match (prefix, local.as_ref()) {
                    $((prefix_else!($($prefix)?), stringify!($name)) => Self::$attr,)+
                    (prefix, _) => Self::Unknown(QualName {
                        prefix,
                        local,
                    }),
                }
            }

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

            pub fn attribute_group(&self) -> C {
                match self {
                    $(Self::$attr => _c::$attr,)+
                    Self::Aliased { attr_id, .. } => attr_id.attribute_group(),
                    Self::Unknown(_) => AttributeGroup::empty(),
                }
            }

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

        impl std::fmt::Display for AttrId<'_> {
            fn fmt(&self, _f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                todo!()
            }
        }

        impl<'input> Attr<'input> {
            /// Creates a new attribute
            pub fn new(name: AttrId<'input>, value: &'input str) -> Self {
                match name {
                    $(AttrId::$attr => match $outer::parse_string(value) {
                        Ok(value) => Self::$attr(value),
                        Err(_) => Self::Unparsed {
                            attr_id: AttrId::$attr,
                            value: value.into(),
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
                    Self::Unparsed { value, .. } => ContentType::Anything(value),
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
                }
            }
        }
    };
}
define_attrs! {
    Accumulate(Accumulate) {
        name: "accumulate",
        categories: AttributeGroup::AnimationAddition,
    },
    Additive(Additive) {
        name: "additive",
        categories: AttributeGroup::AnimationAddition,
    },
    AlignmentBaseline(AlignmentBaseline) {
        name: "alignment-baseline",
        categories: AttributeGroup::Presentation,
    },
    Amplitude(Number) {
        name: "amplitude",
        categories: AttributeGroup::TransferFunction,
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
    AriaGrabbed(TrueFalseUndefined) {
        name: "aria-grabbed",
        categories: AttributeGroup::Aria,
    },
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
    AttributeName(Anything<'input>) {
        name: "attributeName",
        categories: AttributeGroup::AnimationAttributeTarget,
    },
    AutoFocus(Boolean<'input>) {
        name: "autofocus",
        categories: AttributeGroup::Core,
    },
    BaselineShift(Inheritable<BaselineShift>) {
        name: "baseline-shift",
        categories: AttributeGroup::Presentation,
    },
    Begin(ListOf<BeginEnd<'input>, Semicolon>) {
        name: "begin",
        categories: AttributeGroup::AnimationTiming,
    },
    By(Anything<'input>) {
        name: "by",
        categories: AttributeGroup::AnimationValue,
    },
    CalcMode(CalcMode) {
        name: "calcMode",
        categories: AttributeGroup::AnimationValue,
    },
    Class(ListOf<Anything<'input>, Space>) {
        name: "class",
        categories: AttributeGroup::Core,
    },
    Clip(Inheritable<Clip>) {
        name: "clip",
        categories: AttributeGroup::Presentation,
    },
    ClipPath(ClipPath<'input>) {
        name: "clip-path",
        categories: AttributeGroup::Presentation,
    },
    ClipPathUnits(Units) {
        name: "clipPathUnits",
    },
    ClipRule(FillRule) {
        name: "clip-rule",
        categories: AttributeGroup::Presentation,
    },
    Color(Color) {
        name: "color",
        categories: AttributeGroup::Presentation,
    },
    ColorInterpolation(ColorInterpolation) {
        name: "color-interpolation",
        categories: AttributeGroup::Presentation,
    },
    ColorInterpolationFilters(ColorInterpolation) {
        name: "color-interpolation-filters",
        categories: AttributeGroup::Presentation,
    },
    ColorProfile(Inheritable<ColorProfile<'input> >) {
        name: "color-profile",
        categories: AttributeGroup::Presentation,
    },
    ColorRendering(Rendering) {
        name: "color-rendering",
        categories: AttributeGroup::Presentation,
    },
    CrossOrigin(CrossOrigin) {
        name: "crossorigin",
    },
    Cursor(Cursor<'input>) {
        name: "cursor",
        categories: AttributeGroup::Presentation,
    },
    CX(LengthPercentage) {
        name: "cx",
    },
    CY(LengthPercentage) {
        name: "cy",
    },
    D(Path) {
        name: "d",
    },
    Direction(Direction) {
        name: "direction",
    },
    Display(Display) {
        name: "display",
        categories: AttributeGroup::Presentation,
    },
    DominantBaseline(DominantBaseline) {
        name: "dominant-baseline",
        categories: AttributeGroup::Presentation,
    },
    Download(Anything<'input>) {
        name: "download",
    },
    Dur(Dur) {
        name: "dur",
        categories: AttributeGroup::AnimationTiming,
    },
    DX(Length) {
        name: "dx",
    },
    DY(Length) {
        name: "dy",
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
    FeColorMatrixType(FeColorMatrixType) {
        name: "type",
    },
    FeColorMatrixValues(ListOf<Number, SpaceOrComma>) {
        name: "values",
    },
    FeCompositeOperator(FeCompositeOperator) {
        name: "operator",
    },
    FeCompositeK1(Number) {
        name: "k1",
    },
    FeCompositeK2(Number) {
        name: "k2",
    },
    FeCompositeK3(Number) {
        name: "k3",
    },
    FeCompositeK4(Number) {
        name: "k4",
    },
    FeConvolveMatrixKernalMatrix(ListOf<Number, SpaceOrComma>) {
        name: "kernelMatrix",
    },
    FeConvolveMatrixDivisor(Number) {
        name: "divisor",
    },
    FeConvolveMatrixBias(Number) {
        name: "bias",
    },
    FeConvolveMatrixTargetX(Integer) {
        name: "targetX",
    },
    FeConvolveMatrixTargetY(Integer) {
        name: "targetY",
    },
    FeConvolveMatrixPreserveAlpha(TrueFalse) {
        name: "preserveAlpha",
    },
    FeDiffuseLightingDiffuseConstant(Number) {
        name: "diffuseConstant",
    },
    FeDisplacementMapScale(Number) {
        name: "scale",
    },
    FeDisplacementMapXChannelSelector(ChannelSelector) {
        name: "xChannelSelector",
    },
    FeDisplacementMapYChannelSelector(ChannelSelector) {
        name: "yChannelSelector",
    },
    FeDistantLightAzimuth(Number) {
        name: "azimuth",
    },
    FeDistantLightElevation(Number) {
        name: "elevation",
    },
    FeDx(Number) {
        name: "dx",
    },
    FeDy(Number) {
        name: "dy",
    },
    FeEdgeMode(FeEdgeMode) {
        name: "edgeMode",
    },
    PreserveAspectRatio(PreserveAspectRatio) {
        name: "preserveAspectRatio",
    },
    FeKernelUnitLength(NumberOptionalNumber) {
        name: "kernelUnitLength",
    },
    FeOperator(FeOperator) {
        name: "operator",
    },
    FePointsAtX(Number) {
        name: "pointsAtX",
    },
    FePointsAtY(Number) {
        name: "pointsAtY",
    },
    FePointsAtZ(Number) {
        name: "pointsAtZ",
    },
    FeSpecularExponent(Number) {
        name: "specularExponent",
    },
    FeSpecularLightingSpecularConstant(Number) {
        name: "specularConstant",
    },
    FeSpotLightLimitingConeAngle(Number) {
        name: "limitingConeAngle",
    },
    FeStdDeviation(NumberOptionalNumber) {
        name: "stdDeviation",
    },
    FeSurfaceScale(Number) {
        name: "surfaceScale",
    },
    FeRadius(NumberOptionalNumber) {
        name: "radius",
    },
    FeTurbulenceBaseFrequency(NumberOptionalNumber) {
        name: "baseFrequency",
    },
    FeTurbulenceNumOctaves(Integer) {
        name: "numOctaves",
    },
    FeTurbulenceSeed(Number) {
        name: "seed",
    },
    FeTurbulenceStitchTiles(FeTurbulenceStitchTiles) {
        name: "stitchTiles",
    },
    FeTurbulenceType(FeTurbulenceType) {
        name: "type",
    },
    FeX(Number) {
        name: "x",
    },
    FeY(Number) {
        name: "y",
    },
    FeZ(Number) {
        name: "z",
    },
    Fill(Paint<'input>) {
        name: "fill",
        categories: AttributeGroup::Presentation,
    },
    FillTiming(Fill) {
        name: "fill",
        categories: AttributeGroup::AnimationTiming,
    },
    FillOpacity(Opacity) {
        name: "fill-opacity",
        categories: AttributeGroup::Presentation,
    },
    FillRule(FillRule) {
        name: "fill-rule",
        categories: AttributeGroup::Presentation,
    },
    Filter(FilterList<'input>) {
        name: "filter",
        categories: AttributeGroup::Presentation,
    },
    FilterUnits(Units) {
        name: "filterUnits",
    },
    FloodColor(Color) {
        name: "flood-color",
        categories: AttributeGroup::Presentation,
    },
    FloodOpacity(Opacity) {
        name: "flood-opacity",
        categories: AttributeGroup::Presentation,
    },
    Font(Font<'input>) {
        name: "font",
        categories: AttributeGroup::Presentation,
    },
    FontFamily(FontFamily<'input>) {
        name: "font-family",
        categories: AttributeGroup::Presentation,
    },
    FontSize(FontSize) {
        name: "font-size",
        categories: AttributeGroup::Presentation,
    },
    FontSizeAdjust(Number) {
        name: "font-size-adjust",
        categories: AttributeGroup::Presentation,
    },
    FontStretch(FontStretch) {
        name: "font-stretch",
        categories: AttributeGroup::Presentation,
    },
    FontStyle(FontStyle) {
        name: "font-style",
        categories: AttributeGroup::Presentation,
    },
    FontVariant(FontVariant) {
        name: "font-variant",
        categories: AttributeGroup::Presentation,
    },
    FontWeight(FontWeight) {
        name: "font-weight",
        categories: AttributeGroup::Presentation,
    },
    FR(Length) {
        name: "fr",
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
    GlyphOrientationHorizontal(Angle) {
        name: "glyph-orientation-horizontal",
        categories: AttributeGroup::Presentation,
    },
    GlyphOrientationVertical(Angle) {
        name: "glyph-orientation-vertical",
        categories: AttributeGroup::Presentation,
    },
    GradientUnits(Units) {
        name: "gradientUnits",
    },
    GradientTransform(TransformList) {
        name: "gradientTransform",
    },
    ImageRendering(Rendering) {
        name: "image-rendering",
        categories: AttributeGroup::Presentation,
    },
    Height(LengthPercentage) {
        name: "height",
        categories: AttributeGroup::FilterPrimitive,
    },
    Href(Url<'input>) {
        name: "href",
        categories: AttributeGroup::AnimationTargetElement,
    },
    Hreflang(Anything<'input>) {
        name: "hreflang",
    },
    Id(Anything<'input>) {
        name: "id",
        categories: AttributeGroup::Core,
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
    Kerning(Length) {
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
    },
    LetterSpacing(Length) {
        name: "letter-spacing",
        categories: AttributeGroup::Presentation,
    },
    LightingColor(Color) {
        name: "lighting-color",
        categories: AttributeGroup::Presentation,
    },
    Marker(Marker<'input>) {
        name: "marker",
        categories: AttributeGroup::Presentation,
    },
    MarkerEnd(Marker<'input>) {
        name: "marker-end",
        categories: AttributeGroup::Presentation,
    },
    MarkerHeight(LengthOrNumber) {
        name: "markerHeight",
    },
    MarkerMid(Marker<'input>) {
        name: "marker-mid",
        categories: AttributeGroup::Presentation,
    },
    MarkerStart(Marker<'input>) {
        name: "marker-start",
        categories: AttributeGroup::Presentation,
    },
    MarkerUnits(Units) {
        name: "markerUnits",
    },
    MarkerWidth(LengthOrNumber) {
        name: "markerWidth",
    },
    Mask(ListOf<Mask<'input>, Comma>) {
        name: "mask",
        categories: AttributeGroup::Presentation,
    },
    MaskContentUnits(Units) {
        name: "maskContentUnits",
    },
    MaskUnits(Units) {
        name: "maskUnits",
    },
    Max(MinMax) {
        name: "max",
        categories: AttributeGroup::AnimationTiming,
    },
    Media(MediaQueryList<'input>) {
        name: "media",
    },
    Min(MinMax) {
        name: "min",
        categories: AttributeGroup::AnimationTiming,
    },
    Mode(BlendMode) {
        name: "mode",
    },
    Offset(Number) {
        name: "offset",
        categories: AttributeGroup::TransferFunction,
    },
    OnBegin(BeginEnd<'input>) {
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
        categories: AttributeGroup::GlobalEvent,
    },
    OnClose(Anything<'input>) {
        name: "onclose",
        categories: AttributeGroup::GlobalEvent,
    },
    OnCopy(Anything<'input>) {
        name: "oncopy",
        categories: AttributeGroup::DocumentElementEvent,
    },
    OnCuechange(Anything<'input>) {
        name: "oncuechange",
        categories: AttributeGroup::GlobalEvent,
    },
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
        categories: AttributeGroup::GlobalEvent,
    },
    OnScroll(Anything<'input>) {
        name: "onscroll",
        categories: AttributeGroup::GlobalEvent,
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
    Opacity(Opacity) {
        name: "opacity",
        categories: AttributeGroup::Presentation,
    },
    Order(NumberOptionalNumber) {
        name: "order",
    },
    Orient(Orient) {
        name: "orient",
    },
    Origin(Origin) {
        name: "origin",
    },
    Overflow(Overflow) {
        name: "overflow",
        categories: AttributeGroup::Presentation,
    },
    PaintOrder(PaintOrder) {
        name: "paint-order",
        categories: AttributeGroup::Presentation,
    },
    Path(Path) {
        name: "path",
    },
    PathLength(Number) {
        name: "pathLength",
    },
    PatternUnits(Units) {
        name: "patternUnits",
    },
    PatternContentUnits(Units) {
        name: "patternContentUnits",
    },
    PatternTransform(TransformList) {
        name: "patternTransform",
    },
    PointerEvents(PointerEvents) {
        name: "pointer-events",
        categories: AttributeGroup::Presentation,
    },
    Points(ListOf<Number, SpaceOrComma>) {
        name: "points",
    },
    PrimitiveUnits(Units) {
        name: "primitiveUnits",
    },
    R(LengthPercentage) {
        name: "r",
    },
    ReferrerPolicy(ReferrerPolicy) {
        name: "referrerpolicy",
    },
    RefX(RefX) {
        name: "refX",
    },
    RefY(RefY) {
        name: "refY",
    },
    Rel(ListOf<LinkType, Space>) {
        name: "rel",
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
    ScriptType(MediaType<'input>) {
        name: "type",
    },
    ShapeRendering(ShapeRendering) {
        name: "shape-rendering",
        categories: AttributeGroup::Presentation,
    },
    Slope(Number) {
        name: "slope",
        categories: AttributeGroup::TransferFunction,
    },
    SpreadMethod(SpreadMethod) {
        name: "spreadMethod",
    },
    StartOffset(LengthOrNumber) {
        name: "startOffset",
    },
    StopColor(Color) {
        name: "stop-color",
        categories: AttributeGroup::Presentation,
    },
    StopOffset(NumberPercentage) {
        name: "offset",
    },
    StopOpacity(Opacity) {
        name: "stop-opacity",
        categories: AttributeGroup::Presentation,
    },
    Stroke(Paint<'input>) {
        name: "stroke",
        categories: AttributeGroup::Presentation,
    },
    StrokeDasharray(StrokeDasharray) {
        name: "stroke-dasharray",
        categories: AttributeGroup::Presentation,
    },
    StrokeDashoffset(LengthPercentage) {
        name: "stroke-dashoffset",
        categories: AttributeGroup::Presentation,
    },
    StrokeLinecap(StrokeLinecap) {
        name: "stroke-linecap",
        categories: AttributeGroup::Presentation,
    },
    StrokeLinejoin(StrokeLinejoin) {
        name: "stroke-linejoin",
        categories: AttributeGroup::Presentation,
    },
    StrokeMiterlimit(Number) {
        name: "stroke-miterlimit",
        categories: AttributeGroup::Presentation,
    },
    StrokeOpacity(Opacity) {
        name: "stroke-opacity",
        categories: AttributeGroup::Presentation,
    },
    StrokeWidth(LengthPercentage) {
        name: "stroke-width",
        categories: AttributeGroup::Presentation,
    },
    Style(Style<'input>) {
        name: "style",
        categories: AttributeGroup::Core,
    },
    StyleType(MediaType<'input>) {
        name: "type",
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
    },
    TextAnchor(TextAnchor) {
        name: "text-anchor",
        categories: AttributeGroup::Presentation,
    },
    TextDecoration(TextDecoration) {
        name: "text-decoration",
        categories: AttributeGroup::Presentation,
    },
    TextLength(LengthOrNumber) {
        name: "textLength",
    },
    TextPathMethod(TextPathMethod) {
        name: "method",
    },
    TextPathSpacing(TextPathSpacing) {
        name: "spacing",
    },
    TextPathSide(TextPathSide) {
        name: "side",
    },
    TextRendering(Rendering) {
        name: "text-rendering",
        categories: AttributeGroup::Presentation,
    },
    Title(Anything<'input>) {
        name: "title",
    },
    To(Anything<'input>) {
        name: "to",
        categories: AttributeGroup::AnimationValue,
    },
    Transform(Transform) {
        name: "transform",
    },
    TransformOrigin(Position) {
        name: "transform-origin",
        categories: AttributeGroup::Presentation,
    },
    Type(TransferFunctionType) {
        name: "type",
        categories: AttributeGroup::TransferFunction,
    },
    TypeAnimateTransform(TypeAnimateTransform) {
        name: "type",
    },
    UnicodeBidi(UnicodeBidi) {
        name: "type",
        categories: AttributeGroup::Presentation,
    },
    Values(ListOf<Anything<'input>, Semicolon>) {
        name: "values",
        categories: AttributeGroup::AnimationValue,
    },
    VectorEffect(VectorEffect) {
        name: "vector-effect",
        categories: AttributeGroup::Presentation,
    },
    ViewBox(ViewBox) {
        name: "viewBox",
    },
    Visibility(Visibility) {
        name: "visibility",
        categories: AttributeGroup::Presentation,
    },
    Width(LengthPercentage) {
        name: "width",
        categories: AttributeGroup::FilterPrimitive,
    },
    WordSpacing(Spacing) {
        name: "word-spacing",
        categories: AttributeGroup::Presentation,
    },
    WritingMode(WritingMode) {
        name: "writing-mode",
        categories: AttributeGroup::Presentation,
    },
    X(LengthPercentage) {
        name: "x",
        categories: AttributeGroup::FilterPrimitive,
    },
    X1(LengthPercentage) {
        name: "x1",
    },
    X2(LengthPercentage) {
        name: "x2",
    },
    XLinkHref(Url<'input>) {
        prefix: XLink,
        name: "href",
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
    XmlSpace(XmlSpace) {
        prefix: XML,
        name: "space",
        categories: AttributeGroup::Core,
    },
    Y(LengthPercentage) {
        name: "y",
        categories: AttributeGroup::FilterPrimitive,
    },
    Y1(LengthOrNumber) {
        name: "y1",
    },
    Y2(LengthOrNumber) {
        name: "y2",
    },
}

impl PartialEq for AttrId<'_> {
    fn eq(&self, other: &Self) -> bool {
        self.prefix() == other.prefix() && self.local_name() == other.local_name()
    }
}

impl std::fmt::Display for Attr<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let prefix = self.prefix();
        if !prefix.is_empty() {
            prefix.fmt(f)?;
            f.write_str(":")?;
        }
        self.local_name().fmt(f)?;

        Ok(())
    }
}
