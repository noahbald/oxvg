//! Container for all the possible content that can be used by an attribute
use super::attribute::animation::{AttributeType, BeginEnd, CalcMode, ControlPoint};
use super::attribute::animation_addition::{Accumulate, Additive};
use super::attribute::animation_timing::{
    ClockValue, Dur, Fill, MinMax, RepeatCount, RepeatDur, Restart,
};
use super::attribute::aria::{
    AriaAutocomplete, AriaCurrent, AriaDropEffect, AriaHasPopup, AriaInvalid, AriaLive,
    AriaOrientation, AriaRelevant, AriaSort, IDReference, Role, Tristate,
};
use super::attribute::core::{
    Angle, Anything, Boolean, Class, Color, Frequency, FuncIRI, Id, Integer, Length, Name,
    NonWhitespace, Number, NumberOptionalNumber, Opacity, Paint, Percentage, SVGTransformList,
    Style, Time, TokenList, Url, IRI,
};
use super::attribute::filter_effect::{
    ChannelSelector, EdgeMode, In, OperatorFeComposite, OperatorFeMorphology,
    StitchTilesFeTurbulence, TypeFeColorMatrix, TypeFeTurbulence,
};
use super::attribute::fonts::{ArabicForm, Orientation};
use super::attribute::inheritable::Inheritable;
use super::attribute::list_of::{ListOf, Seperators};
use super::attribute::path::{Path, Points};
use super::attribute::presentation::{
    AlignmentBaseline, BaselineShift, Clip, ClipPath, ColorInterpolation, ColorProfile,
    ColorRendering, Cursor, Direction, Display, DominantBaseline, EnableBackground, FillRule,
    FilterList, Font, FontFamily, FontSize, FontSizeAdjust, FontStretch, FontStyle, FontVariant,
    FontWeight, GlyphOrientationVertical, ImageRendering, Kerning, LengthOrNumber,
    LengthPercentage, Marker, Mask, MaskType, Overflow, PaintOrder, PointerEvents, Position,
    ShapeRendering, Spacing, StrokeDasharray, StrokeLinecap, StrokeLinejoin, TextAnchor,
    TextDecoration, TextRendering, UnicodeBidi, VectorEffect, Visibility, WritingMode,
};
use super::attribute::transfer_function::TransferFunctionType;
use super::attribute::uncategorised::{
    BlendMode, ColorProfileName, CrossOrigin, LengthAdjust, LinkType, MarkerUnits, MediaQueryList,
    MediaType, NumberPercentage, Orient, Origin, Playbackorder, PreserveAspectRatio, Radius, RefX,
    RefY, ReferrerPolicy, RenderingIntent, Rotate, SpreadMethod, Target, TextPathMethod,
    TextPathSide, TextPathSpacing, Timelinebegin, Transform, TrueFalse, TrueFalseUndefined,
    TypeAnimateTransform, Units, ViewBox, ZoomAndPan,
};
use super::attribute::xlink::{XLinkActuate, XLinkShow, XLinkType};
use super::attribute::xml::XmlSpace;
use lightningcss::values::string::CowArcStr;
use lightningcss::values::{
    alpha::AlphaValue, length::LengthValue, percentage::DimensionPercentage,
};
use lightningcss::visitor::Visit as _;
use lightningcss::{visit_types, visitor};
use std::ops::{Deref, DerefMut};

use crate::atom::Atom;

#[derive(Debug, PartialEq)]
/// A reference to the content type, as received from [`Attr::value`] or [`Attr::value_mut`]
pub enum ContentTypeRef<'a, T: std::fmt::Debug + PartialEq> {
    /// A reference received from [`Attr::value`]
    Ref(&'a T),
    /// A reference received from [`Attr::value_mut`]
    RefMut(&'a mut T),
}
impl<T: std::fmt::Debug + PartialEq> Deref for ContentTypeRef<'_, T> {
    type Target = T;
    fn deref(&self) -> &Self::Target {
        match self {
            Self::Ref(t) => t,
            Self::RefMut(t) => t,
        }
    }
}
impl<T: std::fmt::Debug + PartialEq> DerefMut for ContentTypeRef<'_, T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        match self {
            Self::Ref(_) => panic!("Cannot mutably deref ContentTypeRef::Ref"),
            Self::RefMut(t) => t,
        }
    }
}

enum ReferenceType {
    Url,
    Id,
    Class,
}
/// A reference to some ident found in the document
pub enum Reference<'a, 'input> {
    /// A reference to some ident found in an SVG value
    Atom(&'a mut Atom<'input>),
    /// A reference to some ident found in an SVG or CSS value
    Css(&'a mut CowArcStr<'input>),
}
impl Deref for Reference<'_, '_> {
    type Target = str;

    fn deref(&self) -> &Self::Target {
        match self {
            Self::Atom(atom) => atom,
            Self::Css(css) => css,
        }
    }
}
struct ReferenceVisitor<'input, F: FnMut(Reference<'_, 'input>)> {
    f: F,
    reference_type: ReferenceType,
    marker: std::marker::PhantomData<&'input ()>,
}
impl<'input, F: FnMut(Reference<'_, 'input>)> visitor::Visitor<'input>
    for ReferenceVisitor<'input, F>
{
    type Error = ();

    fn visit_types(&self) -> visitor::VisitTypes {
        if matches!(self.reference_type, ReferenceType::Url) {
            visit_types!(URLS)
        } else {
            visit_types!(SELECTORS)
        }
    }
    fn visit_url(
        &mut self,
        url: &mut lightningcss::values::url::Url<'input>,
    ) -> Result<(), Self::Error> {
        (self.f)(Reference::Css(&mut url.url));
        Ok(())
    }
    fn visit_selector(
        &mut self,
        selector: &mut lightningcss::selector::Selector<'input>,
    ) -> Result<(), Self::Error> {
        use lightningcss::{selector::Component, values::ident::Ident};
        selector.iter_mut_raw_match_order().for_each(|c| match c {
            Component::Class(Ident(ident))
                if matches!(self.reference_type, ReferenceType::Class) =>
            {
                (self.f)(Reference::Css(ident));
            }
            Component::ID(Ident(ident)) if matches!(self.reference_type, ReferenceType::Id) => {
                (self.f)(Reference::Css(ident));
            }
            _ => {}
        });
        Ok(())
    }
}

macro_rules! define_content_types {
    ($($name:ident($ty:ty)$(<$i:lifetime>)?,)+) => {
        #[derive(Debug, PartialEq)]
        /// A reference to an attribute's value, mutably mapped to it's conent type
        pub enum ContentType<'a, 'input> {
            $(
                #[doc=concat!("a `", stringify!($name), "` value")]
                $name(ContentTypeRef<'a, $ty>)$(<$i>)?,
            )+
            /// A set of a content-type seperated by some deliminator
            ListOf(ListOf<Box<ContentType<'a, 'input>>, Seperators>),
            /// A content type that can be inherited with the `"inherited"` keyword
            Inheritable(Inheritable<Box<ContentType<'a, 'input>>>),
        }

        /// An identifier for an attribute's content type
        pub enum ContentTypeId {
            $(
                #[doc=concat!("a `", stringify!($name), "` value")]
                $name,
            )+
            /// A set of a content-type seperated by some deliminator
            ListOf(Box<ContentTypeId>, Seperators),
            /// A content type that can be inherited with the `"inherited"` keyword
            Inheritable(Box<ContentTypeId>),
        }

        #[cfg(feature = "serialize")]
        impl oxvg_serialize::ToValue for ContentType<'_, '_> {
            fn write_value<W>(&self, dest: &mut oxvg_serialize::Printer<W>) -> Result<(), oxvg_serialize::error::PrinterError>
                where
                    W: std::fmt::Write {
                match self {
                    $(Self::$name(value) => value.write_value(dest),)+
                    Self::Inheritable(value) => value.write_value(dest),
                    Self::ListOf(value) => value.write_value(dest),
                }
            }
        }
    };
}
impl<'input> ContentType<'_, 'input> {
    /// Returns `true` when the attribute is equivalent to the attribute
    /// being omitted.
    ///
    /// e.g. `ContentType::Boolean(Boolean(None))` may resolve to `""`,
    /// but is not empty because it still provides a useful value.
    pub fn is_empty(&self) -> bool {
        match self {
            Self::Anything(value)
            | Self::IRI(value)
            | Self::Name(value)
            | Self::Url(value)
            | Self::MediaType(value) => value.is_empty(),
            Self::Id(value) | Self::IDReference(value) | Self::Class(value) => value.is_empty(),
            Self::Style(style) => {
                style.0.declarations.is_empty() && style.important_declarations.is_empty()
            }
            Self::SVGTransformList(transform_list) => transform_list.0.is_empty(),
            Self::ListOf(ListOf { list, .. }) => list.is_empty(),
            Self::ColorProfile(color_profile) => match &**color_profile {
                ColorProfile::Name(value) | ColorProfile::IRI(value) => value.is_empty(),
                _ => false,
            },
            Self::ColorProfileName(color_profile_name) => match &**color_profile_name {
                ColorProfileName::Name(value) => value.is_empty(),
                ColorProfileName::Srbg => false,
            },
            Self::FilterList(filter_list) => match &**filter_list {
                FilterList::Filters(list) => list.is_empty(),
                FilterList::None => false,
            },
            Self::FontFamily(font_family) => font_family.0.list.is_empty(),
            Self::Mask(mark) => mark.0.list.is_empty(),
            Self::StrokeDasharray(stroke_dasharray) => match &**stroke_dasharray {
                StrokeDasharray::Values(values) => values.is_empty(),
                StrokeDasharray::None => false,
            },
            Self::TokenList(token_list) => token_list.0 .0.is_empty(),
            Self::In(value) => match &**value {
                In::Reference(value) => value.is_empty(),
                _ => false,
            },
            Self::MediaQueryList(media_query_list) => media_query_list.0.media_queries.is_empty(),
            Self::Path(path) => path.0 .0.is_empty(),
            Self::Points(points) => points.0 .0.is_empty(),
            Self::Target(target) => match &**target {
                Target::XMLName(value) => value.is_empty(),
                _ => false,
            },
            Self::Inheritable(Inheritable::Defined(value)) => value.is_empty(),
            _ => false,
        }
    }

    /// For a value, visits any urls it may contain.
    pub fn visit_url<F>(&mut self, mut f: F)
    where
        F: FnMut(Reference<'_, 'input>),
    {
        if let Self::ListOf(ListOf { list, .. }) = self {
            for item in list.iter_mut() {
                item.visit_url_not_list(&mut f);
            }
            return;
        } else if let Self::Inheritable(Inheritable::Defined(value)) = self {
            value.visit_url(f);
            return;
        }
        self.visit_url_not_list(f);
    }
    fn visit_url_not_list<F>(&mut self, mut f: F)
    where
        F: FnMut(Reference<'_, 'input>),
    {
        use lightningcss::values::url::Url;
        use std::marker::PhantomData;
        match self {
            Self::ColorProfile(ContentTypeRef::RefMut(ColorProfile::IRI(url))) => {
                f(Reference::Atom(url));
            }
            Self::FuncIRI(ContentTypeRef::RefMut(url)) | Self::Url(ContentTypeRef::RefMut(url)) => {
                f(Reference::Atom(url));
            }
            Self::ClipPath(ContentTypeRef::RefMut(ClipPath::Url(Url { url, .. })))
            | Self::Paint(ContentTypeRef::RefMut(Paint::Url {
                url: Url { url, .. },
                ..
            }))
            | Self::Marker(ContentTypeRef::RefMut(Marker::Url(Url { url, .. }))) => {
                f(Reference::Css(url));
            }
            Self::Mask(ContentTypeRef::RefMut(Mask(mask))) => {
                use lightningcss::values::image::{Image, ImageSet};
                fn visit_image<'input, F>(image: &mut Image<'input>, f: &mut F)
                where
                    F: FnMut(Reference<'_, 'input>),
                {
                    match image {
                        Image::Url(Url { url, .. }) => f(Reference::Css(url)),
                        Image::ImageSet(ImageSet { options, .. }) => options
                            .iter_mut()
                            .map(|options| &mut options.image)
                            .for_each(|image| visit_image(image, f)),
                        _ => {}
                    }
                }
                mask.list
                    .iter_mut()
                    .map(|mask| &mut mask.image)
                    .for_each(|image| visit_image(image, &mut f));
            }
            Self::FilterList(filters) => {
                filters
                    .visit(&mut ReferenceVisitor {
                        f,
                        reference_type: ReferenceType::Url,
                        marker: PhantomData,
                    })
                    .ok();
            }
            Self::Style(style) => {
                style
                    .0
                    .visit(&mut ReferenceVisitor {
                        f,
                        reference_type: ReferenceType::Url,
                        marker: PhantomData,
                    })
                    .ok();
            }
            _ => {}
        }
    }

    /// For an attribute, visits any IDs it may contain or reference, including [`Attr::Id`]
    pub fn visit_id<F>(&mut self, mut f: F)
    where
        F: FnMut(Reference<'_, 'input>),
    {
        if let Self::ListOf(ListOf { list, .. }) = self {
            for item in list.iter_mut() {
                item.visit_id_not_list(&mut f);
            }
            return;
        } else if let Self::Inheritable(Inheritable::Defined(value)) = self {
            value.visit_id(f);
            return;
        }
        self.visit_id_not_list(f);
    }
    fn visit_id_not_list<F>(&mut self, mut f: F)
    where
        F: FnMut(Reference<'_, 'input>),
    {
        match self {
            Self::IDReference(value) | Self::Id(value) => f(Reference::Atom(&mut (&mut *value).0)),
            Self::BeginEnd(begin_end) => match &mut **begin_end {
                BeginEnd::SyncbaseValue { id, .. }
                | BeginEnd::EventValue { id: Some(id), .. }
                | BeginEnd::RepeatValue { id: Some(id), .. } => f(Reference::Atom(id)),
                _ => {}
            },
            Self::Style(style) => {
                style
                    .0
                    .visit(&mut ReferenceVisitor {
                        f,
                        reference_type: ReferenceType::Id,
                        marker: std::marker::PhantomData,
                    })
                    .ok();
            }
            _ => {}
        }
    }

    /// For an attribute, visits any classes it may contain or reference, including [`Attr::Class`]
    pub fn visit_class<F>(&mut self, mut f: F)
    where
        F: FnMut(Reference<'_, 'input>),
    {
        if let Self::ListOf(ListOf { list, .. }) = self {
            for item in list.iter_mut() {
                item.visit_class_not_list(&mut f);
            }
            return;
        } else if let Self::Inheritable(Inheritable::Defined(value)) = self {
            value.visit_class(f);
            return;
        }
        self.visit_class_not_list(f);
    }
    fn visit_class_not_list<F>(&mut self, mut f: F)
    where
        F: FnMut(Reference<'_, 'input>),
    {
        match self {
            Self::Class(ContentTypeRef::RefMut(NonWhitespace(class))) => f(Reference::Atom(class)),
            Self::Style(style) => {
                style
                    .0
                    .visit(&mut ReferenceVisitor {
                        f,
                        reference_type: ReferenceType::Class,
                        marker: std::marker::PhantomData,
                    })
                    .ok();
            }
            _ => {}
        }
    }

    /// Visits any color values in the content type, excluding from [`ContentType::Style`]
    pub fn visit_color<F>(&mut self, mut f: F)
    where
        F: FnMut(&mut Color),
    {
        if let Self::ListOf(ListOf { list, .. }) = self {
            for item in list.iter_mut() {
                item.visit_color_not_list(&mut f);
            }
            return;
        } else if let Self::Inheritable(Inheritable::Defined(value)) = self {
            value.visit_color(f);
            return;
        }
        self.visit_color_not_list(f);
    }
    fn visit_color_not_list<F>(&mut self, mut f: F)
    where
        F: FnMut(&mut Color),
    {
        match self {
            Self::Color(color) => f(color),
            Self::Paint(paint) => {
                if let Paint::Color(color) = &mut **paint {
                    f(color);
                }
            }
            Self::TextDecoration(text_decoration) => {
                let TextDecoration { color, .. } = &mut **text_decoration;
                f(color);
            }
            _ => {}
        }
    }

    /// Visits any length values in the content type.
    ///
    /// # Arguments
    ///
    /// * `f` - A closure that will run for a visited value
    /// * `follow_list` - Whether to visit list values or not. If this is set to `true`, single values will not be visited
    pub fn visit_length_value<F>(&mut self, mut f: F, follow_list: bool)
    where
        F: FnMut(&mut LengthValue),
    {
        // Assertion should panic when not `ContentTypeRef::RefMut`
        debug_assert!(Some(&mut *self).is_some());

        if follow_list {
            match self {
                Self::StrokeDasharray(ContentTypeRef::RefMut(StrokeDasharray::Values(values))) => {
                    for d in values.iter_mut() {
                        if let DimensionPercentage::Dimension(l) = d {
                            f(l);
                        }
                    }
                    return;
                }
                Self::ListOf(ListOf { list, .. }) => {
                    for item in list.iter_mut() {
                        item.visit_length_value_not_list(&mut f);
                    }
                    return;
                }
                _ => {}
            }
        } else if let Self::Inheritable(Inheritable::Defined(value)) = self {
            value.visit_length_value(f, follow_list);
            return;
        }
        self.visit_length_value_not_list(f);
    }
    fn visit_length_value_not_list<F>(&mut self, mut f: F)
    where
        F: FnMut(&mut LengthValue),
    {
        match self {
            Self::Spacing(ContentTypeRef::RefMut(Spacing::Length(
                lightningcss::values::length::Length::Value(l),
            )))
            | Self::Kerning(ContentTypeRef::RefMut(Kerning::Length(Length::Length(l))))
            | Self::LengthPercentage(ContentTypeRef::RefMut(LengthPercentage(
                DimensionPercentage::Dimension(l),
            )))
            | Self::Length(ContentTypeRef::RefMut(Length::Length(l))) => f(l),
            _ => {}
        }
    }

    /// Visits any floating numbers in the content type, excluding numbers that are part of a [`LengthValue`].
    ///
    /// Use [`Self::visit_length_value`] to visit numbers within a [`LengthValue`].
    ///
    /// # Arguments
    ///
    /// * `f` - A closure that will run for a visited value
    /// * `follow_list` - Whether to visit list values or not. If this is set to `true`, single values will not be visited
    pub fn visit_float<F>(&mut self, mut f: F, follow_list: bool)
    where
        F: FnMut(&mut f32),
    {
        // Assertion should panic when not `ContentTypeRef::RefMut`
        debug_assert!(Some(&mut *self).is_some());

        self.visit_length_value(
            |l| {
                #[allow(clippy::enum_glob_use)]
                use LengthValue::*;
                match l {
                    Px(n) | In(n) | Cm(n) | Mm(n) | Q(n) | Pt(n) | Pc(n) | Em(n) | Rem(n)
                    | Ex(n) | Rex(n) | Ch(n) | Rch(n) | Cap(n) | Rcap(n) | Ic(n) | Ric(n)
                    | Lh(n) | Rlh(n) | Vw(n) | Lvw(n) | Svw(n) | Dvw(n) | Cqw(n) | Vh(n)
                    | Lvh(n) | Svh(n) | Dvh(n) | Cqh(n) | Vi(n) | Svi(n) | Lvi(n) | Dvi(n)
                    | Cqi(n) | Vb(n) | Svb(n) | Lvb(n) | Dvb(n) | Cqb(n) | Vmin(n) | Svmin(n)
                    | Lvmin(n) | Dvmin(n) | Cqmin(n) | Vmax(n) | Svmax(n) | Lvmax(n) | Dvmax(n)
                    | Cqmax(n) => f(n),
                }
            },
            follow_list,
        );
        if follow_list {
            match self {
                Self::EnableBackground(ContentTypeRef::RefMut(EnableBackground::New(Some((
                    x,
                    y,
                    width,
                    height,
                ))))) => {
                    f(x);
                    f(y);
                    f(width);
                    f(height);
                    return;
                }
                Self::StrokeDasharray(ContentTypeRef::RefMut(StrokeDasharray::Values(values))) => {
                    for d in values.iter_mut() {
                        if let DimensionPercentage::Percentage(n) = d {
                            n.0 *= 100.0;
                            f(&mut n.0);
                            n.0 /= 100.0;
                        }
                    }
                    return;
                }
                Self::ControlPoint(ContentTypeRef::RefMut(ControlPoint(values))) => {
                    values.iter_mut().for_each(f);
                    return;
                }
                Self::ListOf(ListOf { list, .. }) => {
                    for item in list.iter_mut() {
                        item.visit_float_not_list(&mut f);
                    }
                    return;
                }
                _ => {}
            }
        } else if let Self::Inheritable(Inheritable::Defined(value)) = self {
            value.visit_float(f, follow_list);
            return;
        }
        self.visit_float_not_list(f);
    }
    fn visit_float_not_list<F>(&mut self, mut f: F)
    where
        F: FnMut(&mut f32),
    {
        match self {
            // Length
            Self::Length(ContentTypeRef::RefMut(Length::Number(n)))
            | Self::Kerning(ContentTypeRef::RefMut(Kerning::Length(Length::Number(n))))
            // Angle
            | Self::Angle(ContentTypeRef::RefMut(
                Angle::Deg(n) | Angle::Rad(n) | Angle::Grad(n) | Angle::Turn(n),
            ))
            // Frequency
            | Self::Frequency(ContentTypeRef::RefMut(Frequency::Hz(n) | Frequency::KHz(n)))
            // Opacity
            | Self::Opacity(ContentTypeRef::RefMut(AlphaValue(n)))
            // RepeateCount
            | Self::RepeatCount(ContentTypeRef::RefMut(RepeatCount::Number(n)))
            // Rotate
            | Self::Rotate(ContentTypeRef::RefMut(Rotate::Number(n)))
            // Number
            | Self::NumberPercentage(ContentTypeRef::RefMut(NumberPercentage::Number(n)))
            // LengthOrNumber
            | Self::LengthOrNumber(ContentTypeRef::RefMut(LengthOrNumber::Number(n)))
            | Self::RefX(ContentTypeRef::RefMut(RefX::LengthOrNumber(LengthOrNumber::Number(n))))
            | Self::RefY(ContentTypeRef::RefMut(RefY::LengthOrNumber(LengthOrNumber::Number(n)))) => f(n),
            // Number
            Self::Number(ContentTypeRef::RefMut(n)) => f(n),
            Self::Percentage(ContentTypeRef::RefMut(Percentage(p)))
            | Self::Radius(ContentTypeRef::RefMut(Radius::LengthPercentage(LengthPercentage(DimensionPercentage::Percentage(Percentage(p))))))
            | Self::LengthPercentage(ContentTypeRef::RefMut(LengthPercentage(DimensionPercentage::Percentage(Percentage(p)))))
            | Self::Length(ContentTypeRef::RefMut(Length::Percentage(Percentage(p))))
            | Self::Kerning(ContentTypeRef::RefMut(Kerning::Length(Length::Percentage(Percentage(p)))))
            | Self::NumberPercentage(ContentTypeRef::RefMut(NumberPercentage::Percentage(Percentage(p)))) => {
                // Normalise percentage to display value
                *p *= 100.0;
                f(p);
                *p /= 100.0;
            }
            Self::NumberOptionalNumber(ContentTypeRef::RefMut(NumberOptionalNumber(n, n2))) => {
                f(n);
                if let Some(n2) = n2 {
                    f(n2);
                }
            }
            Self::ViewBox(ContentTypeRef::RefMut(ViewBox {
                min_x,
                min_y,
                width,
                height,
            })) => {
                f(min_x);
                f(min_y);
                f(width);
                f(height);
            }
            _ => {}
        }
    }

    /// Rounds any safetly roundable numbers in the content type
    pub fn round(&mut self, float_precision: i32, convert_px: bool, round_list: bool) {
        debug_assert!(
            float_precision <= 5,
            "rounding precision should be no greater than 5"
        );
        if let Self::Points(points) = self {
            let factor = 10.0_f64.powi(float_precision);
            let round_float = |n: &mut f64| *n = (*n * factor).round() / factor;
            for point in &mut points.0 .0 {
                point.args_mut().iter_mut().for_each(round_float);
            }
            return;
        }

        let factor = 10.0_f32.powi(float_precision);
        let round_float = |n: &mut f32| *n = (*n * factor).round() / factor;
        if convert_px {
            self.visit_length_value(
                |l| {
                    if let Some(px) = l.to_px() {
                        *l = LengthValue::Px(px);
                    }
                },
                round_list,
            );
        }
        self.visit_float(round_float, round_list);
    }
}

define_content_types! {
    Angle(Angle),
    Anything(Anything<'input>),
    ArabicForm(ArabicForm),
    Boolean(Boolean<'input>),
    ClockValue(ClockValue),
    Color(Color),
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
    SVGTransformList(SVGTransformList),
    Time(Time),
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
    ColorRendering(ColorRendering),
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
    FontSizeAdjust(FontSizeAdjust),
    FontStretch(FontStretch),
    FontStyle(FontStyle),
    FontVariant(FontVariant),
    FontWeight(FontWeight),
    GlyphOrientationVertical(GlyphOrientationVertical),
    ImageRendering(ImageRendering),
    Kerning(Kerning),
    LengthPercentage(LengthPercentage),
    LengthOrNumber(LengthOrNumber),
    Marker(Marker<'input>),
    MarkerUnits(MarkerUnits),
    Mask(Mask<'input>),
    MaskType(MaskType),
    Orientation(Orientation),
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
    TextRendering(TextRendering),
    TokenList(TokenList<'input>),
    UnicodeBidi(UnicodeBidi),
    VectorEffect(VectorEffect),
    Visibility(Visibility),
    WritingMode(WritingMode),

    // Attr specific
    Accumulate(Accumulate),
    Additive(Additive),
    AttributeType(AttributeType),
    BeginEnd(BeginEnd<'input>),
    BlendMode(BlendMode),
    CalcMode(CalcMode),
    ChannelSelector(ChannelSelector),
    Class(Class<'input>),
    ColorProfileName(ColorProfileName<'input>),
    ControlPoint(ControlPoint),
    CrossOrigin(CrossOrigin),
    Dur(Dur),
    TypeFeColorMatrix(TypeFeColorMatrix),
    OperatorFeComposite(OperatorFeComposite),
    EdgeMode(EdgeMode),
    OperatorFeMorphology(OperatorFeMorphology),
    StitchTilesFeTurbulence(StitchTilesFeTurbulence),
    TypeFeTurbulence(TypeFeTurbulence),
    Fill(Fill),
    Units(Units),
    Id(Id<'input>),
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
    Playbackorder(Playbackorder),
    Points(Points),
    PreserveAspectRatio(PreserveAspectRatio),
    Radius(Radius),
    RefX(RefX),
    RefY(RefY),
    ReferrerPolicy(ReferrerPolicy),
    RenderingIntent(RenderingIntent),
    RepeatCount(RepeatCount),
    RepeatDur(RepeatDur),
    Restart(Restart),
    Rotate(Rotate),
    SpreadMethod(SpreadMethod),
    Target(Target<'input>),
    TextPathMethod(TextPathMethod),
    TextPathSpacing(TextPathSpacing),
    TextPathSide(TextPathSide),
    Timelinebegin(Timelinebegin),
    TransferFunctionType(TransferFunctionType),
    Transform(Transform),
    TrueFalse(TrueFalse),
    TrueFalseUndefined(TrueFalseUndefined),
    TypeAnimateTransform(TypeAnimateTransform),
    ViewBox(ViewBox),
    XLinkActuate(XLinkActuate),
    XLinkShow(XLinkShow),
    XLinkType(XLinkType),
    XmlSpace(XmlSpace),
    ZoomAndPan(ZoomAndPan),
}
