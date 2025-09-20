//! Groups for attributes
bitflags! {
    #[derive(Copy, Clone, Hash, PartialEq, Eq)]
    /// Specifies which attribute groups an attribute may belong to
    pub struct AttributeGroup: u32 {
        /// Attributes used for animation addition
        const AnimationAddition = 1 << 0;
        /// Attributes used for animation targets
        const AnimationAttributeTarget = 1 << 1;
        /// Attributes used for animation events
        const AnimationEvent = 1 << 2;
        /// Attributes used to identify the target element for an animation
        const AnimationTargetElement = 1 << 3;
        /// Attributes used for animation timing
        const AnimationTiming = 1 << 4;
        /// Attributes used for animation values
        const AnimationValue = 1 << 5;
        /// Attributes used for assistive information
        const Aria = 1 << 6;
        /// Attributes used for conditional processing
        const ConditionalProcessing = 1 << 7;
        /// Attributes used for any SVG element
        const Core = 1 << 8;
        /// Attributes used for document events
        const DocumentEvent = 1 << 9;
        /// Attributes used for document element events
        const DocumentElementEvent = 1 << 10;
        /// Attributes used for filter primitive elements
        const FilterPrimitive = 1 << 11;
        /// Attributes used for graphical events
        const GraphicalEvent = 1 << 12;
        /// Attributes used for global events
        const GlobalEvent = 1 << 13;
        /// Attributes used for styling
        const Presentation = 1 << 14;
        /// Attributes used for transfer functions
        const TransferFunction = 1 << 15;
        /// Attributes used for xlink features
        const XLink = 1 << 16;
    }
}
impl AttributeGroup {
    /// Returns union of all event groups
    pub const fn event() -> Self {
        Self::AnimationEvent
            .union(Self::DocumentEvent)
            .union(Self::DocumentElementEvent)
            .union(Self::GraphicalEvent)
            .union(Self::GlobalEvent)
    }
}

bitflags! {
    #[derive(Copy, Clone, Hash, PartialEq, Eq)]
    /// Specifies info about an attribute
    pub struct AttributeInfo: u32 {
        /// A deprecated attribute that can be safely removed
        ///
        /// Usually this means the attribute is no longer supported in
        /// the SVG 2 draft and it's removal has no ill-effect
        const DeprecatedSafe = 1 << 0;
        /// A deprecated attribute that cannot be safely removed
        ///
        /// Usually this means the attribute is no longer supported in
        /// the SVG 2 draft but it's removal has some effect
        const DeprecatedUnsafe = 1 << 1;
        /// The attribute automatically inherits from parent elements
        const Inheritable = 1 << 2;
        /// The attribute is an inheritable presentation attribute but
        /// compounds, so it cannot be arbitrarily added or removed.
        const PresentationNonInheritableGroupAttrs = 1 << 3;
    }
}
