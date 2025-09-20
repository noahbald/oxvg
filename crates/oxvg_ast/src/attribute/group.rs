bitflags! {
    pub struct AttributeGroup: u32 {
        /// Attributes used for animation addition
        const AnimationAddition = 0b000_0000_0000_0001;
        /// Attributes used for animation targets
        const AnimationAttributeTarget = 0b0_0000_0010;
        /// Attributes used for animation events
        const AnimationEvent = 0b0_0000_0000_0000_0100;
        /// Attributes used to identify the target element for an animation
        const AnimationTargetElement = 0b000_0000_1000;
        /// Attributes used for animation timing
        const AnimationTiming = 0b_0000_0000_0001_0000;
        /// Attributes used for animation values
        const AnimationValue = 0b0_0000_0000_0010_0000;
        /// Attributes used for assistive information
        const Aria = 0b0_0000_0000_0000_0000_0100_0000;
        /// Attributes used for conditional processing
        const ConditionalProcessing = 0b0000_1000_0000;
        /// Attributes used for any SVG element
        const Core = 0b0_0000_0000_0000_0001_0000_0000;
        /// Attributes used for document events
        const DocumentEvent = 0b00_0000_0010_0000_0000;
        /// Attributes used for document element events
        const DocumentElementEvent = 0b_0100_0000_0000;
        /// Attributes used for filter primitive elements
        const FilterPrimitive = 0b_0000_1000_0000_0000;
        /// Attributes used for graphical events
        const GraphicalEvent = 0b0_0001_0000_0000_0000;
        /// Attributes used for global events
        const GlobalEvent = 0b0000_0010_0000_0000_0000;
        /// Attributes used for styling
        const Presentation = 0b000_0100_0000_0000_0000;
        /// Attributes used for transfer functions
        const TransferFunction = 0b1000_0000_0000_0000;
        /// Attributes used for xlink features
        const XLink = 0b_0000_0001_0000_0000_0000_0000;
    }
}
