//! A collection of elements and attributes in SVG documents
use phf::{phf_map, phf_set};

/// A group is a category of some SVG name
pub trait Group<'a> {
    /// Returns whether the string belongs to the group
    fn matches<T>(&self, value: T) -> bool
    where
        T: Into<&'a str>,
    {
        self.set().contains(value.into())
    }

    /// Returns the group as a set
    fn set(&self) -> &'a phf::Set<&'static str>;
}

#[derive(Hash, PartialEq, Eq)]
/// The different types an element can belong to
pub enum ElementGroup {
    /// Elements used for animation
    Animation,
    /// Elements used for descriptions
    Descriptive,
    /// Elements used for shapes
    Shape,
    /// Elements used for document structure
    Structural,
    /// Elements used for painting
    PaintServer,
    /// Elements used for non-rendering tasks
    NonRendering,
    /// Elements used for containing specific elements
    Container,
    /// Elements used for typography
    TextContent,
    /// Elements used for typography content
    TextContentChild,
    /// Elements used for lighting
    LightSource,
    /// Elements used for filtering
    FilterPrimitive,
}

impl<'a> Group<'a> for ElementGroup {
    fn set(&self) -> &'a phf::Set<&'static str> {
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
/// See [`ElementGroup::Animation`]
pub static ANIMATION: phf::Set<&'static str> = phf_set! {
    "animate",
    "animateColor",
    "animateMotion",
    "animateTransform",
    "set"
};
/// See [`ElementGroup::Descriptive`]
pub static DESCRIPTIVE: phf::Set<&'static str> = phf_set! { "desc", "metadata", "title" };
/// See [`ElementGroup::Shape`]
pub static SHAPE: phf::Set<&'static str> =
    phf_set! { "circle", "ellipse", "line", "path", "polygon", "polyline", "rect" };
/// See [`ElementGroup::Structural`]
pub static STRUCTURAL: phf::Set<&'static str> = phf_set! { "defs", "g", "svg", "symbol", "use" };
/// See [`ElementGroup::PaintServer`]
pub static PAINT_SERVER: phf::Set<&'static str> = phf_set! {
    "hatch",
    "linearGradient",
    "meshGradient",
    "pattern",
    "radialGradient",
    "solidColor"
};
/// See [`ElementGroup::NonRendering`]
pub static NON_RENDERING: phf::Set<&'static str> = phf_set! {
    "clipPath",
    "filter",
    "linearGradient",
    "marker",
    "mask",
    "pattern",
    "radialGradient",
    "solidColor",
    "symbol"
};
/// See [`ElementGroup::Container`]
pub static CONTAINER: phf::Set<&'static str> = phf_set! {
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
};
/// See [`ElementGroup::TextContent`]
pub static TEXT_CONTENT: phf::Set<&'static str> = phf_set! {
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
};
/// See [`ElementGroup::TextContentChild`]
pub static TEXT_CONTENT_CHILD: phf::Set<&'static str> =
    phf_set! { "alyGlyph", "textPath", "tref", "tspan" };
/// See [`ElementGroup::LightSource`]
pub static LIGHT_SOURCE: phf::Set<&'static str> = phf_set! {
    "feDiffuseLighting",
    "feDistantLight",
    "fePointLight",
    "feSpecularLighting",
    "feSpotLight"
};
/// See [`ElementGroup::FilterPrimitive`]
pub static FILTER_PRIMITIVE: phf::Set<&'static str> = phf_set! {
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
};

#[derive(Hash, PartialEq, Eq)]
/// The different types an attribute can belong to
pub enum AttrsGroups {
    /// Attributes used for animation addition
    AnimationAddition,
    /// Attributes used for animation targets
    AnimationAttributeTarget,
    /// Attributes used for animation events
    AnimationEvent,
    /// Attributes used for animation timing
    AnimationTiming,
    /// Attributes used for animation values
    AnimationValue,
    /// Attributes used for conditional processing
    ConditionalProcessing,
    /// Attributes used for any SVG element
    Core,
    /// Attributes used for graphical events
    GraphicalEvent,
    /// Attributes used for styling
    Presentation,
    /// Attributes used for xlink features
    XLink,
    /// Attributes used for document events
    DocumentEvent,
    /// Attributes used for document element events
    DocumentElementEvent,
    /// Attributes used for global events
    GlobalEvent,
    /// Attributes used for filter primitive elements
    FilterPrimitive,
    /// Attributes used for transfer functions
    TransferFunction,
}

impl<'a> Group<'a> for AttrsGroups {
    fn set(&self) -> &'a phf::Set<&'static str> {
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
            Self::DocumentElementEvent => &DOCUMENT_ELEMENT_EVENT,
            Self::GlobalEvent => &GLOBAL_EVENT,
            Self::FilterPrimitive => &FILTER_PRIMITIVE,
            Self::TransferFunction => &TRANSFER_FUNCTION,
        }
    }
}

// NOTE: Can't seem to macronise this
/// Attributes used for events
pub static EVENT_ATTRS: phf::Set<&'static str> = phf_set! {
    // ANIMATION_EVENT
    "onbegin",
    "onend",
    "onrepeat",
    "onload",
    // DOCUMENT_EVENT
    "onabort",
    "onerror",
    "onresize",
    "onscroll",
    "onunload",
    "onzoom",
    // DOCUMENT_ELEMENT_EVENT
    "oncopy",
    "oncut",
    "onpaste",
    // GLOBAL_EVENT (NOTE: Deduplicated)
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
    "onfocus",
    "oninput",
    "oninvalid",
    "onkeydown",
    "onkeypress",
    "onkeyup",
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
    // GRAPHICAL_EVENT (NOTE: Deduplicated)
    "onactivate",
    "onfocusin",
    "onfocusout",
};

// Attrs groups
/// See [`AttrsGroups::AnimationAddition`]
pub static ANIMATION_ADDITION: phf::Set<&'static str> = phf_set! { "additive", "accumulate" };
/// See [`AttrsGroups::AnimationAttributeTarget`]
pub static ANIMATION_ATTRIBUTE_TARGET: phf::Set<&'static str> =
    phf_set! { "attributeType", "attributeName" };
/// See [`AttrsGroups::AnimationEvent`]
pub static ANIMATION_EVENT: phf::Set<&'static str> =
    phf_set! { "onbegin", "onend", "onrepeat", "onload" };
/// See [`AttrsGroups::AnimationTiming`]
pub static ANIMATION_TIMING: phf::Set<&'static str> = phf_set! {
    "begin",
    "dur",
    "end",
    "fill",
    "max",
    "min",
    "repeatCount",
    "repeatDur",
    "restart",
};
/// See [`AttrsGroups::AnimationValue`]
pub static ANIMATION_VALUE: phf::Set<&'static str> = phf_set! {
    "by",
    "calcMode",
    "from",
    "keySplines",
    "keyTimes",
    "to",
    "values",
};
/// See [`AttrsGroups::ConditionalProcessing`]
pub static CONDITIONAL_PROCESSING: phf::Set<&'static str> =
    phf_set! { "requiredExtensions", "requiredFeatures", "systemLanguage", };
/// See [`AttrsGroups::Core`]
pub static CORE: phf::Set<&'static str> =
    phf_set! { "id", "tabindex", "xml:base", "xml:lang", "xml:space" };
/// See [`AttrsGroups::GraphicalEvent`]
pub static GRAPHICAL_EVENT: phf::Set<&'static str> = phf_set! {
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
};
/// See [`AttrsGroups::Presentation`]
pub static PRESENTATION: phf::Set<&'static str> = phf_set! {
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
};
/// See [`AttrsGroups::XLink`]
pub static X_LINK: phf::Set<&'static str> = phf_set! {
    "xlink:actuate",
    "xlink:arcrole",
    "xlink:href",
    "xlink:role",
    "xlink:show",
    "xlink:title",
    "xlink:type",
};
/// See [`AttrsGroups::DocumentEvent`]
pub static DOCUMENT_EVENT: phf::Set<&'static str> =
    phf_set! {"onabort", "onerror", "onresize", "onscroll", "onunload", "onzoom",};
/// See [`AttrsGroups::DocumentElementEvent`]
pub static DOCUMENT_ELEMENT_EVENT: phf::Set<&'static str> = phf_set! {"oncopy", "oncut", "onpaste"};
/// See [`AttrsGroups::GlobalEvent`]
pub static GLOBAL_EVENT: phf::Set<&'static str> = phf_set! {
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
};
/// See [`AttrsGroups::FilterPrimitive`]
pub static FILTER_PRIMITIVE_ATTRS: phf::Set<&'static str> =
    phf_set! {"x", "y", "width", "height", "result"};
/// See [`AttrsGroups::TransferFunction`]
pub static TRANSFER_FUNCTION: phf::Set<&'static str> = phf_set! {
    "amplitude",
    "exponent",
    "intercept",
    "offset",
    "slope",
    "tableValues",
    "type",
};

impl AttrsGroups {
    /// Returns the default values for a given attribute group
    pub fn defaults(&self) -> Option<&phf::Map<&'static str, &'static str>> {
        match self {
            Self::Core => Some(&CORE_DEFAULTS),
            Self::Presentation => Some(&PRESENTATION_DEFAULTS),
            Self::TransferFunction => Some(&TRANSFER_FUNCTION_DEFAULTS),
            _ => None,
        }
    }

    /// Returns the default values for an attribute group's static set
    pub fn defaults_from_static<'a>(
        static_set: &'a phf::Set<&str>,
    ) -> Option<&'a phf::Map<&'static str, &'static str>> {
        let key = static_set.map.key;
        if key == CORE.map.key {
            Some(&CORE_DEFAULTS)
        } else if key == PRESENTATION.map.key {
            Some(&PRESENTATION_DEFAULTS)
        } else if key == TRANSFER_FUNCTION.map.key {
            Some(&TRANSFER_FUNCTION_DEFAULTS)
        } else {
            None
        }
    }
}

// Attrs groups defaults
static CORE_DEFAULTS: phf::Map<&'static str, &'static str> = phf_map!("xml:space" => "default");
static PRESENTATION_DEFAULTS: phf::Map<&'static str, &'static str> = phf_map!(
    "clip" => "auto",
    "clip-path" => "none",
    "clip-rule" => "nonzero",
    "mask" => "none",
    "opacity" => "1",
    "stop-color" => "#000",
    "stop-opacity" => "1",
    "fill-opacity" => "1",
    "fill-rule" => "nonzero",
    "fill" => "#000",
    "stroke" => "none",
    "stroke-width" => "1",
    "stroke-linecap" => "butt",
    "stroke-linejoin" => "miter",
    "stroke-miterlimit" => "4",
    "stroke-dasharray" => "none",
    "stroke-dashoffset" => "0",
    "stroke-opacity" => "1",
    "paint-order" => "normal",
    "vector-effect" => "none",
    "display" => "inline",
    "visibility" => "visible",
    "marker-start" => "none",
    "marker-mid" => "none",
    "marker-end" => "none",
    "color-interpolation" => "sRGB",
    "color-interpolation-filters" => "linearRGB",
    "color-rendering" => "auto",
    "shape-rendering" => "auto",
    "text-rendering" => "auto",
    "image-rendering" => "auto",
    "font-style" => "normal",
    "font-variant" => "normal",
    "font-weight" => "normal",
    "font-stretch" => "normal",
    "font-size" => "medium",
    "font-size-adjust" => "none",
    "kerning" => "auto",
    "letter-spacing" => "normal",
    "word-spacing" => "normal",
    "text-decoration" => "none",
    "text-anchor" => "start",
    "text-overflow" => "clip",
    "writing-mode" => "lr-tb",
    "glyph-orientation-vertical" => "auto",
    "glyph-orientation-horizontal" => "0deg",
    "direction" => "ltr",
    "unicode-bidi" => "normal",
    "dominant-baseline" => "auto",
    "alignment-baseline" => "baseline",
    "baseline-shift" => "baseline",
);
static TRANSFER_FUNCTION_DEFAULTS: phf::Map<&'static str, &'static str> = phf_map!(
    "slope" => "1",
    "intercept" => "0",
    "amplitude" => "1",
    "exponent" => "1",
    "offset" => "0",
);

/// Elements that use paths
pub static PATH_ELEMS: phf::Set<&'static str> = phf_set!("glyph", "missing-glyph", "path");
/// Attributes that can be inherited
pub static INHERITABLE_ATTRS: phf::Set<&'static str> = phf_set!(
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
);
/// Presentation attributes that aren't inherited
pub static PRESENTATION_NON_INHERITABLE_GROUP_ATTRS: phf::Set<&'static str> = phf_set!(
    "clip-path",
    "display",
    "filter",
    "mask",
    "opacity",
    "text-decoration",
    "transform",
    "unicode-bidi",
);
/// Attributes that may reference other elements by id
pub static REFERENCES_PROPS: phf::Set<&'static str> = phf_set!(
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
);
/// Pseudoclasses used to select display state
pub static PSEUDO_DISPLAY_STATE: phf::Set<&'static str> =
    phf_set!("fullscreen", "modal", "picture-in-picture");
/// Pseudoclasses used to select input state
pub static PSEUDO_INPUT: phf::Set<&'static str> = phf_set!(
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
);
/// Pseudoclasses used to select language state
pub static PSEUDO_LINGUISTIC: phf::Set<&'static str> = phf_set!("dir", "lang");
/// Pseudoclasses used to select link state
pub static PSEUDO_LOCATION: phf::Set<&'static str> = phf_set!(
    "any-link",
    "link",
    "local-link",
    "scope",
    "target-within",
    "target",
    "visited",
);
/// Pseudoclasses used to select media state
pub static PSEUDO_RESOURCE_STATE: phf::Set<&'static str> = phf_set!("playing", "paused");
/// Pseudoclasses used to select temporal state
pub static PSEUDO_TIME_DIMENSIONAL: phf::Set<&'static str> = phf_set!("current", "past", "future");
/// Pseudoclasses used to select DOM state
pub static PSEUDO_TREE_STRUCTURAL: phf::Set<&'static str> = phf_set!(
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
);
/// Pseudoclasses used to select user state
pub static PSEUDO_USER_ACTION: phf::Set<&'static str> =
    phf_set!("active", "focus-visible", "focus-within", "focus", "hover");
/// Pseudoclasses used to select logical state
pub static PSEUDO_FUNCTIONAL: phf::Set<&'static str> = phf_set!("is", "not", "where", "has");
/// Namespaces used by vector editors
pub static EDITOR_NAMESPACES: phf::Set<&'static str> = phf_set!(
    "http://creativecommons.org/ns#",
    "http://inkscape.sourceforge.net/DTD/sodipodi-0.dtd",
    "http://ns.adobe.com/AdobeIllustrator/10.0/",
    "http://ns.adobe.com/AdobeSVGViewerExtensions/3.0/",
    "http://ns.adobe.com/Extensibility/1.0/",
    "http://ns.adobe.com/Flows/1.0/",
    "http://ns.adobe.com/GenericCustomNamespace/1.0/",
    "http://ns.adobe.com/Graphs/1.0/",
    "http://ns.adobe.com/ImageReplacement/1.0/",
    "http://ns.adobe.com/SaveForWeb/1.0/",
    "http://ns.adobe.com/Variables/1.0/",
    "http://ns.adobe.com/XPath/1.0/",
    "http://purl.org/dc/elements/1.1/",
    "http://schemas.microsoft.com/visio/2003/SVGExtensions/",
    "http://sodipodi.sourceforge.net/DTD/sodipodi-0.dtd",
    "http://taptrix.com/vectorillustrator/svg_extensions",
    "http://www.bohemiancoding.com/sketch/ns",
    "http://www.figma.com/figma/ns",
    "http://www.inkscape.org/namespaces/inkscape",
    "http://www.serif.com/",
    "http://www.vector.evaxdesign.sk",
    "http://www.w3.org/1999/02/22-rdf-syntax-ns#",
);
