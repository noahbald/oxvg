//! Categories for element
bitflags! {
    /// Specifies which categories an element may belong to
    pub struct ElementCategory: u32 {
        /// Elements used for animation
        const Animation = 1 << 0;
        /// Elements used for basic shapes
        const BasicShape = 1 << 1;
        /// Elements used for containing specific elements
        const Container = 1 << 2;
        /// Elements used for descriptions
        const Descriptive = 1 << 3;
        /// Elements used for filtering
        const FilterPrimitive = 1 << 4;
        /// Elements used for gradients
        const Gradient = 1 << 5;
        /// Elements used for graphics
        const Graphics = 1 << 6;
        /// Element used for referencing graphics
        const GraphicsReferencing = 1 << 7;
        /// Elements used for lighting
        const LightSource = 1 << 8;
        /// Elements used for non-rendering tasks
        const NeverRendered = 1 << 9;
        /// Elements used for painting
        const PaintServer = 1 << 10;
        /// Elements that are renderable
        const Renderable = 1 << 11;
        /// Elements used for shapes
        const Shape = 1 << 12;
        /// Elements used for document structure
        const Structural = 1 << 13;
        /// Elements referencing an external resource, specifically when using `href`
        const StructurallyExternal = 1 << 14;
        /// Elements used for typography
        const TextContent = 1 << 15;
        /// Elements used for typography content
        const TextContentChild = 1 << 16;
        /// Elements used as transfer functions for the color components of an input graphic
        const TransferFunction = 1 << 17;
        /// Uncategorised elements
        const Uncategorised = 1 << 18;
    }
}

bitflags! {
    /// Specifies which categories an element may belong to
    pub struct ElementInfo: u32 {
        /// Element is used for non-rendering tasks
        const NonRendering = 1 << 0;
        /// Element is a legacy element
        const Legacy = 1 << 1;
    }
}
