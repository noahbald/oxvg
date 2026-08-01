//! Various commands that can be executed by oxvg
mod action;
mod format;
mod jsx;
mod lint;
mod optimise;
#[cfg(feature = "render")]
mod render;
#[cfg(feature = "visual-regression")]
mod visual_regression;

pub use action::Action;
pub use format::Format;
pub use jsx::JSX;
pub use lint::Lint;
pub use optimise::Optimise;
#[cfg(feature = "render")]
pub use render::Render;
#[cfg(feature = "visual-regression")]
pub use visual_regression::VisualRegression;
