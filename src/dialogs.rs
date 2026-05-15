use std::fmt::Display;

use wxdragon::prelude::*;

/// Shows a modal error dialog.
pub fn show_error(parent: &dyn WxWidget, message: impl Display, title: &str) {
	let dialog = MessageDialog::builder(parent, &message.to_string(), title)
		.with_style(MessageDialogStyle::OK | MessageDialogStyle::IconError)
		.build();
	dialog.show_modal();
}

/// Shows a modal warning dialog.
pub fn show_warning(parent: &dyn WxWidget, message: &str, title: &str) {
	let dialog = MessageDialog::builder(parent, message, title)
		.with_style(MessageDialogStyle::OK | MessageDialogStyle::IconWarning)
		.build();
	dialog.show_modal();
}

/// Shows a modal single-line text entry dialog.
///
/// Returns `None` if the user cancelled or entered only whitespace.
pub fn prompt_text(parent: &Frame, message: &str, title: &str) -> Option<String> {
	let dialog = TextEntryDialog::builder(parent, message, title)
		.with_style(TextEntryDialogStyle::Default | TextEntryDialogStyle::ProcessEnter)
		.build();
	if dialog.show_modal() != ID_OK {
		dialog.destroy();
		return None;
	}
	let value = dialog.get_value().unwrap_or_default();
	dialog.destroy();
	let trimmed = value.trim();
	if trimmed.is_empty() { None } else { Some(trimmed.to_string()) }
}
