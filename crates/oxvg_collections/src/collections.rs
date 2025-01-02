use std::collections::{BTreeMap, BTreeSet};

pub trait Group<'a> {
    fn matches<T>(&self, value: T) -> bool
    where
        T: Into<&'a str>,
    {
        self.set().contains(value.into())
    }

    fn set(&self) -> &'a BTreeSet<&'static str>;
}

#[derive(Hash, PartialEq, Eq)]
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
    fn set(&self) -> &'a BTreeSet<&'static str> {
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

// Element groups
lazy_static! {
    static ref ANIMATION: BTreeSet<&'static str> = BTreeSet::from([
        "animate",
        "animateColor",
        "animateMotion",
        "animateTransform",
        "set"
    ]);
    static ref DESCRIPTIVE: BTreeSet<&'static str> = BTreeSet::from(["desc", "metadata", "title"]);
    static ref SHAPE: BTreeSet<&'static str> =
        BTreeSet::from(["circle", "ellipse", "line", "path", "polygon", "polyline", "rect"]);
    static ref STRUCTURAL: BTreeSet<&'static str> =
        BTreeSet::from(["defs", "g", "svg", "symbol", "use"]);
    static ref PAINT_SERVER: BTreeSet<&'static str> = BTreeSet::from([
        "hatch",
        "linearGradient",
        "meshGradient",
        "pattern",
        "radialGradient",
        "solidColor"
    ]);
    static ref NON_RENDERING: BTreeSet<&'static str> = BTreeSet::from([
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
    static ref CONTAINER: BTreeSet<&'static str> = BTreeSet::from([
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
    static ref TEXT_CONTENT: BTreeSet<&'static str> = BTreeSet::from([
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
    static ref TEXT_CONTENT_CHILD: BTreeSet<&'static str> =
        BTreeSet::from(["alyGlyph", "textPath", "tref", "tspan"]);
    static ref LIGHT_SOURCE: BTreeSet<&'static str> = BTreeSet::from([
        "feDiffuseLighting",
        "feDistantLight",
        "fePointLight",
        "feSpecularLighting",
        "feSpotLight"
    ]);
    static ref FILTER_PRIMITIVE: BTreeSet<&'static str> = BTreeSet::from([
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
}

#[derive(Hash, PartialEq, Eq)]
pub enum AttrsGroups {
    AnimationAddition,
    AnimationAttributeTarget,
    AnimationEvent,
    AnimationTiming,
    AnimationValue,
    ConditionalProcessing,
    Core,
    GraphicalEvent,
    Presentation,
    XLink,
    DocumentEvent,
    DocumentElementEvent,
    GlobalEvent,
    FilterPrimitive,
    TransferFunction,
}

impl<'a> Group<'a> for AttrsGroups {
    fn set(&self) -> &'a BTreeSet<&'static str> {
        match self {
            Self::AnimationAddition => &ANIMATION_ADDITION,
            Self::AnimationAttributeTarget => &ANIMATION_ATTRIBUTE_TARGET,
            Self::AnimationEvent => &ANIMATION_EVENT,
            Self::AnimationTiming => &ANIMATION_TIMING,
            Self::AnimationValue => &ANIMATION_VALUE,
            Self::ConditionalProcessing => &CONDITIONAL_PROCESSING,
            Self::Core => &CORE,
            Self::GraphicalEvent => &GRAPHICAL_EVENT,
            Self::Presentation => &PRESENTATION,
            Self::XLink => &X_LINK,
            Self::DocumentEvent => &DOCUMENT_EVENT,
            Self::DocumentElementEvent => &DOCUMENT_EVEMENT_EVENT,
            Self::GlobalEvent => &GLOBAL_EVENT,
            Self::FilterPrimitive => &FILTER_PRIMITIVE,
            Self::TransferFunction => &TRANSFER_FUNCTION,
        }
    }
}

// Attrs groups
lazy_static! {
    pub static ref ANIMATION_ADDITION: BTreeSet<&'static str> =
        BTreeSet::from(["additive", "accumulate"]);
    pub static ref ANIMATION_ATTRIBUTE_TARGET: BTreeSet<&'static str> =
        BTreeSet::from(["attributeType", "attributeName"]);
    pub static ref ANIMATION_EVENT: BTreeSet<&'static str> =
        BTreeSet::from(["onbegin", "onend", "onrepeat", "onload"]);
    pub static ref ANIMATION_TIMING: BTreeSet<&'static str> = BTreeSet::from([
        "begin",
        "dur",
        "end",
        "fill",
        "max",
        "min",
        "repeatCount",
        "repeatDur",
        "restart",
    ]);
    pub static ref ANIMATION_VALUE: BTreeSet<&'static str> = BTreeSet::from([
        "by",
        "calcMode",
        "from",
        "keySplines",
        "keyTimes",
        "to",
        "values",
    ]);
    pub static ref CONDITIONAL_PROCESSING: BTreeSet<&'static str> =
        BTreeSet::from(["requiredExtensions", "requiredFeatures", "systemLanguage",]);
    pub static ref CORE: BTreeSet<&'static str> =
        BTreeSet::from(["id", "tabindex", "xml:base", "xml:lang", "xml:space"]);
    pub static ref GRAPHICAL_EVENT: BTreeSet<&'static str> = BTreeSet::from([
        "onactivate",
        "onclick",
        "onfocusin",
        "onfocusout",
        "onload",
        "onmousedown",
        "onmousemove",
        "onmouseout",
        "onmouseover",
        "onmouseup",
    ]);
    pub static ref PRESENTATION: BTreeSet<&'static str> = BTreeSet::from([
        "alignment-baseline",
        "baseline-shift",
        "clip-path",
        "clip-rule",
        "clip",
        "color-interpolation-filters",
        "color-interpolation",
        "color-profile",
        "color-rendering",
        "color",
        "cursor",
        "direction",
        "display",
        "dominant-baseline",
        "enable-background",
        "fill-opacity",
        "fill-rule",
        "fill",
        "filter",
        "flood-color",
        "flood-opacity",
        "font-family",
        "font-size-adjust",
        "font-size",
        "font-stretch",
        "font-style",
        "font-variant",
        "font-weight",
        "glyph-orientation-horizontal",
        "glyph-orientation-vertical",
        "image-rendering",
        "letter-spacing",
        "lighting-color",
        "marker-end",
        "marker-mid",
        "marker-start",
        "mask",
        "opacity",
        "overflow",
        "paint-order",
        "pointer-events",
        "shape-rendering",
        "stop-color",
        "stop-opacity",
        "stroke-dasharray",
        "stroke-dashoffset",
        "stroke-linecap",
        "stroke-linejoin",
        "stroke-miterlimit",
        "stroke-opacity",
        "stroke-width",
        "stroke",
        "text-anchor",
        "text-decoration",
        "text-overflow",
        "text-rendering",
        "transform-origin",
        "transform",
        "unicode-bidi",
        "vector-effect",
        "visibility",
        "word-spacing",
        "writing-mode",
    ]);
    pub static ref X_LINK: BTreeSet<&'static str> = BTreeSet::from([
        "xlink:actuate",
        "xlink:arcrole",
        "xlink:href",
        "xlink:role",
        "xlink:show",
        "xlink:title",
        "xlink:type",
    ]);
    pub static ref DOCUMENT_EVENT: BTreeSet<&'static str> =
        BTreeSet::from(["onabort", "onerror", "onresize", "onscroll", "onunload", "onzoom",]);
    pub static ref DOCUMENT_EVEMENT_EVENT: BTreeSet<&'static str> =
        BTreeSet::from(["oncopy", "oncut", "onpaste"]);
    pub static ref GLOBAL_EVENT: BTreeSet<&'static str> = BTreeSet::from([
        "oncancel",
        "oncanplay",
        "oncanplaythrough",
        "onchange",
        "onclick",
        "onclose",
        "oncuechange",
        "ondblclick",
        "ondrag",
        "ondragend",
        "ondragenter",
        "ondragleave",
        "ondragover",
        "ondragstart",
        "ondrop",
        "ondurationchange",
        "onemptied",
        "onended",
        "onerror",
        "onfocus",
        "oninput",
        "oninvalid",
        "onkeydown",
        "onkeypress",
        "onkeyup",
        "onload",
        "onloadeddata",
        "onloadedmetadata",
        "onloadstart",
        "onmousedown",
        "onmouseenter",
        "onmouseleave",
        "onmousemove",
        "onmouseout",
        "onmouseover",
        "onmouseup",
        "onmousewheel",
        "onpause",
        "onplay",
        "onplaying",
        "onprogress",
        "onratechange",
        "onreset",
        "onresize",
        "onscroll",
        "onseeked",
        "onseeking",
        "onselect",
        "onshow",
        "onstalled",
        "onsubmit",
        "onsuspend",
        "ontimeupdate",
        "ontoggle",
        "onvolumechange",
        "onwaiting",
    ]);
    pub static ref FILTER_PRIMITIVE_ATTRS: BTreeSet<&'static str> =
        BTreeSet::from(["x", "y", "width", "height", "result"]);
    pub static ref TRANSFER_FUNCTION: BTreeSet<&'static str> = BTreeSet::from([
        "amplitude",
        "exponent",
        "intercept",
        "offset",
        "slope",
        "tableValues",
        "type",
    ]);
}

impl AttrsGroups {
    pub fn defaults(&self) -> Option<&BTreeMap<&'static str, &'static str>> {
        match self {
            Self::Core => Some(&CORE_DEFAULTS),
            Self::Presentation => Some(&PRESENTATION_DEFAULTS),
            Self::TransferFunction => Some(&TRANSFER_FUNCTION_DEFAULTS),
            _ => None,
        }
    }
}

// Attrs groups defaults
lazy_static! {
    static ref CORE_DEFAULTS: BTreeMap<&'static str, &'static str> =
        BTreeMap::from([("xml:space", "default")]);
    static ref PRESENTATION_DEFAULTS: BTreeMap<&'static str, &'static str> = BTreeMap::from([
        ("clip", "auto"),
        ("clip-path'", "none"),
        ("clip-rule'", "nonzero"),
        ("mask", "none"),
        ("opacity", "1"),
        ("stop-color'", "#000"),
        ("stop-opacity'", "1"),
        ("fill-opacity'", "1"),
        ("fill-rule'", "nonzero"),
        ("fill", "#000"),
        ("stroke", "none"),
        ("stroke-width'", "1"),
        ("stroke-linecap'", "butt"),
        ("stroke-linejoin'", "miter"),
        ("stroke-miterlimit'", "4"),
        ("stroke-dasharray'", "none"),
        ("stroke-dashoffset'", "0"),
        ("stroke-opacity'", "1"),
        ("paint-order'", "normal"),
        ("vector-effect'", "none"),
        ("display", "inline"),
        ("visibility", "visible"),
        ("marker-start'", "none"),
        ("marker-mid'", "none"),
        ("marker-end'", "none"),
        ("color-interpolation'", "sRGB"),
        ("color-interpolation-filters'", "linearRGB"),
        ("color-rendering'", "auto"),
        ("shape-rendering'", "auto"),
        ("text-rendering'", "auto"),
        ("image-rendering'", "auto"),
        ("font-style'", "normal"),
        ("font-variant'", "normal"),
        ("font-weight'", "normal"),
        ("font-stretch'", "normal"),
        ("font-size'", "medium"),
        ("font-size-adjust'", "none"),
        ("kerning", "auto"),
        ("letter-spacing'", "normal"),
        ("word-spacing'", "normal"),
        ("text-decoration'", "none"),
        ("text-anchor'", "start"),
        ("text-overflow'", "clip"),
        ("writing-mode'", "lr-tb"),
        ("glyph-orientation-vertical'", "auto"),
        ("glyph-orientation-horizontal'", "0deg"),
        ("direction", "ltr"),
        ("unicode-bidi'", "normal"),
        ("dominant-baseline'", "auto"),
        ("alignment-baseline'", "baseline"),
        ("baseline-shift'", "baseline"),
    ]);
    static ref TRANSFER_FUNCTION_DEFAULTS: BTreeMap<&'static str, &'static str> = BTreeMap::from([
        ("slope", "1"),
        ("intercept", "0"),
        ("amplitude", "1"),
        ("exponent", "1"),
        ("offset", "0"),
    ]);
}

lazy_static! {
    pub static ref PATH_ELEMS: BTreeSet<&'static str> =
        BTreeSet::from(["glyph", "missing-glyph", "path"]);
    pub static ref INHERITABLE_ATTRS: BTreeSet<&'static str> = BTreeSet::from([
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
    pub static ref PRESENTATION_NON_INHERITABLE_GROUP_ATTRS: BTreeSet<&'static str> =
        BTreeSet::from([
            "clip-path",
            "display",
            "filter",
            "mask",
            "opacity",
            "text-decoration",
            "transform",
            "unicode-bidi",
        ]);
    pub static ref REFERENCES_PROPS: BTreeSet<&'static str> = BTreeSet::from([
        "clip-path",
        "color-profile",
        "fill",
        "filter",
        "marker-end",
        "marker-mid",
        "marker-start",
        "mask",
        "stroke",
        "style",
    ]);
    pub static ref PSEUDO_DISPLAY_STATE: BTreeSet<&'static str> =
        BTreeSet::from(["fullscreen", "modal", "picture-in-picture"]);
    pub static ref PSEUDO_INPUT: BTreeSet<&'static str> = BTreeSet::from([
        "autofill",
        "blank",
        "checked",
        "default",
        "disabled",
        "enabled",
        "in-range",
        "indetermined",
        "invalid",
        "optional",
        "out-of-range",
        "placeholder-shown",
        "read-only",
        "read-write",
        "required",
        "user-invalid",
        "valid",
    ]);
    pub static ref PSEUDO_LINGUISTIC: BTreeSet<&'static str> = BTreeSet::from(["dir", "lang"]);
    pub static ref PSEUDO_LOCATION: BTreeSet<&'static str> = BTreeSet::from([
        "any-link",
        "link",
        "local-link",
        "scope",
        "target-within",
        "target",
        "visited",
    ]);
    pub static ref PSEUDO_RESOURCE_STATE: BTreeSet<&'static str> =
        BTreeSet::from(["playing", "paused"]);
    pub static ref PSEUDO_TIME_DIMENSIONAL: BTreeSet<&'static str> =
        BTreeSet::from(["current", "past", "future"]);
    pub static ref PSEUDO_TREE_STRUCTURAL: BTreeSet<&'static str> = BTreeSet::from([
        "empty",
        "first-child",
        "first-of-type",
        "last-child",
        "last-of-type",
        "nth-child",
        "nth-last-child",
        "nth-last-of-type",
        "nth-of-type",
        "only-child",
        "only-of-type",
        "root",
    ]);
    pub static ref PSEUDO_USER_ACTION: BTreeSet<&'static str> =
        BTreeSet::from(["active", "focus-visible", "focus-within", "focus", "hover",]);
    pub static ref PSEUDO_FUNCTIONAL: BTreeSet<&'static str> =
        BTreeSet::from(["is", "not", "where", "has"]);
}
