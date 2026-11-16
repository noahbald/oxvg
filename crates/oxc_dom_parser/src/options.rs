pub struct ParseOptions {
    /// Whether to allow `<!DOCTYPE>` and parse it's contents.
    ///
    /// This option has no effect for parsing HTML, which always parses `<!DOCTYPE html>` style declarations.
    /// Other doctypes are ignored in HTML.
    pub allow_dtd: AllowDTD,
    pub max_nodes: u32,
}

#[derive(Default)]
pub enum AllowDTD {
    True(AllowDTDOptions),
    /// Will
    #[default]
    False,
}

pub struct AllowDTDOptions {
    pub check_entity_references: bool,
    pub max_entity_depth: u8,
    pub max_entity_references: u8,
    pub max_dtd_items: u32,
}

impl Default for ParseOptions {
    fn default() -> Self {
        Self {
            allow_dtd: AllowDTD::default(),
            max_nodes: u32::MAX,
        }
    }
}

impl Default for AllowDTDOptions {
    fn default() -> Self {
        Self {
            check_entity_references: true,
            max_entity_depth: 10,
            max_entity_references: u8::MAX,
            max_dtd_items: u32::MAX,
        }
    }
}

impl ParseOptions {
    #[must_use]
    /// Allows dtd parsing with default options
    pub fn allow_dtd(self) -> Self {
        self.allow_dtd_options(AllowDTDOptions::default())
    }

    #[must_use]
    pub fn allow_dtd_options(mut self, options: AllowDTDOptions) -> Self {
        self.allow_dtd = AllowDTD::True(options);
        self
    }

    #[must_use]
    pub fn max_nodes(mut self, max_nodes: u32) -> Self {
        self.max_nodes = max_nodes;
        self
    }
}
