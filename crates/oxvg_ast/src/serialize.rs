pub trait Node {
    /// # Errors
    /// If the underlying serialization fails
    fn serialize(&self) -> anyhow::Result<String>;
}
