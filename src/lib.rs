#![warn(clippy::all, clippy::cargo, clippy::nursery, clippy::pedantic)]

mod about;
mod dialogs;
mod ids;

pub use about::AboutBoxBuilder;
pub use dialogs::{prompt_text, show_error, show_warning};
