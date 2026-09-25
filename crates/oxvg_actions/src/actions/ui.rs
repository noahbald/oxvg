use crate::{Actor, Error, actions::UIAction};

impl<'input> Actor<'input, '_> {
    /// Returns version and application info
    ///
    /// # Errors
    ///
    /// Never
    ///
    /// # Spec
    ///
    #[doc = include_str!("../spec/ui/about.md")]
    pub fn about(&mut self) -> Result<(), Error<'input>> {
        self.effect_ui(UIAction::Dialog {
            heading: "About OXVG".to_string(),
            // NOTE: Contributors are welcome to add themselves here!
            body: concat!(
                "OXVG (",
                env!("CARGO_PKG_VERSION"),
                r#") is the fastest SVG toolchain for optimisation, linting, transformation, and manipulation.

Created by Noah Baldwin

- [GitHub](https://github.com/noahbald/oxvg)
- [License](https://github.com/noahbald/oxvg/blob/main/LICENSE)"#
            )
            .to_string(),
        })
    }
}
