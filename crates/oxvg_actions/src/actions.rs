use strum_macros::EnumIter;

#[derive(EnumIter, Debug)]
/// Each action that can be applied to the document or the program's state.
pub enum Actions {
    /// See [`About`]
    About,
    /// See [`ActionList`]
    ActionList,
}

impl Actions {
    /// Describes the effect of each action.
    pub fn describe(&self) -> &str {
        match self {
            Self::About => "Oxvg version, authors, license",
            Self::ActionList => "Print a list of actions and exit",
        }
    }
}
