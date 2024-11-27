pub trait Node {
    /// # Errors
    /// If the underlying serialization fails
    fn serialize(&self) -> anyhow::Result<String>;

    /// # Errors
    /// If the underlying serialization fails
    fn serialize_into<Wr: std::io::Write>(&self, sink: Wr) -> anyhow::Result<()>;
}
