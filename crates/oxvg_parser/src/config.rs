use serde::Deserialize;

#[derive(Deserialize, Default)]
pub struct Config {
    pub optimisation: Option<oxvg_optimiser::Jobs>,
}
