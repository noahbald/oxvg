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
    True {
        check_entity_references: bool,
        max_entity_depth: u8,
        max_entity_references: u8,
        max_dtd_items: u32,
    },
    /// Will
    #[default]
    False,
}

impl Default for ParseOptions {
    fn default() -> Self {
        Self {
            allow_dtd: AllowDTD::default(),
            max_nodes: u32::MAX,
        }
    }
}

impl ParseOptions {
    #[must_use]
    /// Allows dtd parsing with default options
    pub fn allow_dtd(mut self) -> Self {
        self.allow_dtd = AllowDTD::True {
            check_entity_references: true,
            max_entity_depth: 10,
            max_entity_references: u8::MAX,
            max_dtd_items: u32::MAX,
        };
        self
    }

    pub fn allow_dtd_options(
        mut self,
        check_entity_references: bool,
        max_entity_depth: u8,
        max_entity_references: u8,
        max_dtd_items: u32,
    ) -> Self {
        self.allow_dtd = AllowDTD::True {
            check_entity_references,
            max_entity_depth,
            max_entity_references,
            max_dtd_items,
        };
        self
    }

    pub fn max_nodes(mut self, max_nodes: u32) -> Self {
        self.max_nodes = max_nodes;
        self
    }
}
