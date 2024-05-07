use std::fs;

use html5ever::{parse_document, tendril::TendrilSink, tree_builder::TreeBuilderOpts, ParseOpts};
use rcdom::RcDom;

/// Parses document based on given config
///
/// # Errors
/// Returns an error at the first I/O error or if the path in the config doesn't exist
pub fn parse(config: Config) -> std::io::Result<RcDom> {
    let opts = ParseOpts {
        tree_builder: TreeBuilderOpts {
            drop_doctype: true,
            ..Default::default()
        },
        ..Default::default()
    };
    let mut file = fs::File::open(config.path)?;
    parse_document(RcDom::default(), opts)
        .from_utf8()
        .read_from(&mut file)
}


pub struct Config {
    pub path: String,
}

impl TryInto<Config> for std::env::Args {
    type Error = &'static str;

    fn try_into(mut self) -> Result<Config, Self::Error> {
        self.next();

        if let Some(path) = self.next() {
            Ok(Config { path })
        } else {
            Err("No path given")
        }
    }
}
