use html5ever::parse_document;
use html5ever::tendril::TendrilSink;
use html5ever::tree_builder::TreeBuilderOpts;
use html5ever::ParseOpts;
use oxvg_parser::Config;
use rcdom::RcDom;
use std::env;
use std::fs;
use std::process;

fn main() -> anyhow::Result<()> {
    let config: Config = env::args().try_into().unwrap_or_else(|error| {
        eprintln!("Invalid arguments: {error}");
        process::exit(1);
    });
    let opts = ParseOpts {
        tree_builder: TreeBuilderOpts {
            drop_doctype: true,
            ..Default::default()
        },
        ..Default::default()
    };
    let mut file = fs::File::open(config.path).expect("Unable to read file");
    parse_document(RcDom::default(), opts)
        .from_utf8()
        .read_from(&mut file)?;
    Ok(())
}
