use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum Configuration {
    Name(String),
    Configuration {
        name: String,
        value: serde_json::Value,
    },
}

#[test]
fn configuration_serialization() -> Result<(), &'static str> {
    let _: crate::configuration::Configuration =
        serde_json::from_str("{\"name\": \"AddAttributesToSVGElement\"}")
            .map_err(|_| "Failed from serde")?;
    Ok(())
}
