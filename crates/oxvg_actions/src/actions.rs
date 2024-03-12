use strum_macros::EnumIter;

#[derive(EnumIter, Debug)]
pub enum Actions {
    About,
    ActionList,
}

impl Actions {
    pub fn describe(&self) -> &str {
        match self {
            Self::About => "Oxvg version, authors, license",
            Self::ActionList => "Print a list of actions and exit",
        }
    }
}
