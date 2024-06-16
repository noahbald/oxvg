use std::collections::HashSet;

pub trait Group<'a> {
    fn matches<T>(&self, value: T) -> bool
    where
        T: Into<&'a str>,
    {
        self.set().contains(value.into())
    }

    fn set(&self) -> &'a HashSet<&'static str>;
}

pub enum ElementGroup {
    Animation,
    Descriptive,
    Shape,
    Structural,
    PaintServer,
    NonRendering,
    Container,
    TextContent,
    TextContentChild,
    LightSource,
    FilterPrimitive,
}

impl<'a> Group<'a> for ElementGroup {
    fn set(&self) -> &'a HashSet<&'static str> {
        match self {
            Self::Animation => &ANIMATION,
            Self::Descriptive => &DESCRIPTIVE,
            Self::Shape => &SHAPE,
            Self::Structural => &STRUCTURAL,
            Self::PaintServer => &PAINT_SERVER,
            Self::NonRendering => &NON_RENDERING,
            Self::Container => &CONTAINER,
            Self::TextContent => &TEXT_CONTENT,
            Self::TextContentChild => &TEXT_CONTENT_CHILD,
            Self::LightSource => &LIGHT_SOURCE,
            Self::FilterPrimitive => &FILTER_PRIMITIVE,
        }
    }
}

lazy_static! {
    static ref ANIMATION: HashSet<&'static str> = HashSet::from([
        "animate",
        "animateColor",
        "animateMotion",
        "animateTransform",
        "set"
    ]);
    static ref DESCRIPTIVE: HashSet<&'static str> = HashSet::from(["desc", "metadata", "title"]);
    static ref SHAPE: HashSet<&'static str> =
        HashSet::from(["circle", "ellipse", "line", "path", "polygon", "polyline", "rect"]);
    static ref STRUCTURAL: HashSet<&'static str> =
        HashSet::from(["defs", "g", "svg", "symbol", "use"]);
    static ref PAINT_SERVER: HashSet<&'static str> = HashSet::from([
        "hatch",
        "linearGradient",
        "meshGradient",
        "pattern",
        "radialGradient",
        "solidColor"
    ]);
    static ref NON_RENDERING: HashSet<&'static str> = HashSet::from([
        "clipPath",
        "filter",
        "linearGradient",
        "marker",
        "mask",
        "pattern",
        "radialGradient",
        "solidColor",
        "symbol"
    ]);
    static ref CONTAINER: HashSet<&'static str> = HashSet::from([
        "a",
        "defs",
        "foreignObject",
        "g",
        "marker",
        "mask",
        "missing-glyph",
        "pattern",
        "svg",
        "switch",
        "symbol"
    ]);
    static ref TEXT_CONTENT: HashSet<&'static str> = HashSet::from([
        "a",
        "altGlyph",
        "altGlyphDef",
        "alyGlyphItem",
        "glyph",
        "glyphRef",
        "text",
        "textPath",
        "tref",
        "tspan"
    ]);
    static ref TEXT_CONTENT_CHILD: HashSet<&'static str> =
        HashSet::from(["alyGlyph", "textPath", "tref", "tspan"]);
    static ref LIGHT_SOURCE: HashSet<&'static str> = HashSet::from([
        "feDiffuseLighting",
        "feDistantLight",
        "fePointLight",
        "feSpecularLighting",
        "feSpotLight"
    ]);
    static ref FILTER_PRIMITIVE: HashSet<&'static str> = HashSet::from([
        "feBlend",
        "feColorMatrix",
        "feComponentTransfer",
        "feComposite",
        "feConvolveMatrix",
        "feDiffuseLighting",
        "feDisplacementMap",
        "feDropShadow",
        "feFlood",
        "feFuncA",
        "feFuncB",
        "feFuncG",
        "feFuncR",
        "feGaussianBlur",
        "feImage",
        "feMerge",
        "feMergeNode",
        "feMorphology",
        "feOffset",
        "feSpecularLighting",
        "feTile",
        "feTurbulence"
    ]);
    pub static ref INHERITABLE_ATTRS: HashSet<&'static str> = HashSet::from([
        "clip-rule",
        "color-interpolation-filters",
        "color-interpolation",
        "color-profile",
        "color-rendering",
        "color",
        "cursor",
        "direction",
        "dominant-baseline",
        "fill-opacity",
        "fill-rule",
        "fill",
        "font-family",
        "font-size-adjust",
        "font-size",
        "font-stretch",
        "font-style",
        "font-variant",
        "font-weight",
        "font",
        "glyph-orientation-horizontal",
        "glyph-orientation-vertical",
        "image-rendering",
        "letter-spacing",
        "marker-end",
        "marker-mid",
        "marker-start",
        "marker",
        "paint-order",
        "pointer-events",
        "shape-rendering",
        "stroke-dasharray",
        "stroke-dashoffset",
        "stroke-linecap",
        "stroke-linejoin",
        "stroke-miterlimit",
        "stroke-opacity",
        "stroke-width",
        "stroke",
        "text-anchor",
        "text-rendering",
        "transform",
        "visibility",
        "word-spacing",
        "writing-mode",
    ]);
}
