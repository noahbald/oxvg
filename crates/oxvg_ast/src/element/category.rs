bitflags! {
    pub struct ElementCategory: u32 {
        /// Elements used for animation
        const Animation = 0b0000_0000_0000_0000_0000_0001;
        /// Elements used for basic shapes
        const BasicShape = 0b000_0000_0000_0000_0000_0010;
        /// Elements used for containing specific elements
        const Container = 0b0000_0000_0000_0000_0000_0100;
        /// Elements used for descriptions
        const Descriptive = 0b00_0000_0000_0000_0000_1000;
        /// Elements used for filtering
        const FilterPrimitive = 0b000_0000_0000_0001_0000;
        /// Elements used for gradients
        const Gradient = 0b_0000_0000_0000_0000_0010_0000;
        /// Elements used for graphics
        const Graphics = 0b_0000_0000_0000_0000_0100_0000;
        /// Element used for referencing graphics
        const GraphicsReferencing = 0b0000_0000_1000_0000;
        /// Elements used for lighting
        const LightSource = 0b00_0000_0000_0001_0000_0000;
        /// Elements used for non-rendering tasks
        const NeverRendered = 0b_0000_0000_0010_0000_0000;
        /// Elements used for painting
        const PaintServer = 0b00_0000_0000_0100_0000_0000;
        /// Elements that are renderable
        const Renderable = 0b000_0000_0000_1000_0000_0000;
        /// Elements used for shapes
        const Shape = 0b000_0000_0000_0001_0000_0000_0000;
        /// Elements used for document structure
        const Structural = 0b000_0000_0010_0000_0000_0000;
        /// Elements referencing an external resource, specifically when using `href`
        const StructurallyExternal = 0b100_0000_0000_0000;
        /// Elements used for typography
        const TextContent = 0b00_0000_1000_0000_0000_0000;
        /// Elements used for typography content
        const TextContentChild = 0b01_0000_0000_0000_0000;
        /// Elements used as transfer functions for the color components of an input graphic
        const TransferFunction = 0b10_0000_0000_0000_0000;
        /// Uncategorised elements
        const Uncategorised = 0b_0100_0000_0000_0000_0000;
    }
}
