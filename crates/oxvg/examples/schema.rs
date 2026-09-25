//! Builds common schema files

fn main() {
    let schema = schemars::schema_for!(oxvg::config::Config);
    let output = serde_json::to_string_pretty(&schema).unwrap();
    std::fs::create_dir("schema");
    std::fs::write("schema/oxvg.json", output).unwrap();

    let schema = schemars::schema_for!(oxvg_optimiser::Jobs);
    let output = serde_json::to_string_pretty(&schema).unwrap();
    std::fs::write("schema/oxvg_optimiser.json", output).unwrap();
}
