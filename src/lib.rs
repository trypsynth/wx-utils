#![warn(clippy::all, clippy::cargo, clippy::nursery, clippy::pedantic)]

mod about;
mod dialogs;
pub mod dpi;
mod ids;

pub use about::AboutBoxBuilder;
pub use dialogs::{
	DIALOG_PADDING, add_ok_cancel_footer, add_single_button_footer, bind_enter_confirms, build_ok_cancel_buttons,
	dialog_padding, prompt_number, prompt_text, show_error, show_warning,
};
