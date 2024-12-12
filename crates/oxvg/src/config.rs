use oxvg_ast::implementations::markup5ever::Element5Ever;
use serde::Deserialize;

#[derive(Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct Config {
    pub optimisation: Option<oxvg_optimiser::Jobs<Element5Ever>>,
}
