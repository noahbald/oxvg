pub trait Node {
    fn serialize(&self) -> anyhow::Result<String>;
}
