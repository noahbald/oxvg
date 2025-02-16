use oxvg_ast::implementations::markup5ever::Element5Ever;
use serde::{Deserialize, Serialize};

#[derive(Deserialize, Serialize, Debug, Default)]
#[serde(rename_all = "camelCase")]
pub struct Config {
    pub optimise: Option<oxvg_optimiser::Jobs<Element5Ever>>,
}
