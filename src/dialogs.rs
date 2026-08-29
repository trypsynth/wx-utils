use std::fmt::Display;

use patois::t;
use wxdragon::prelude::*;

/// Default padding used between widgets and dialog edges, in device-independent pixels.
///
/// This is the value at 100% display scaling. Pass it through [`crate::dpi::scale`] before
/// handing it to a sizer, or use [`dialog_padding`], which does that for you - on a 150%
/// display a raw 10 here is two thirds of the intended gap.
pub const DIALOG_PADDING: i32 = 10;

/// [`DIALOG_PADDING`] scaled for the display `reference` is on.
///
/// Read it once per dialog and reuse it, rather than calling this at every sizer `add`.
#[must_use]
pub fn dialog_padding(reference: &dyn WxWidget) -> i32 {
	crate::dpi::scale(reference, DIALOG_PADDING)
}

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

/// Builds a standard OK/Cancel button pair parented to `dialog`.
///
/// `ok_label` is caller-supplied since it varies by dialog (e.g. "OK" vs. a verb like
/// "Go"); the Cancel button is always the localized "Cancel", translated via
/// [`patois::t`]. Uses the stock `ID_OK`/`ID_CANCEL` IDs (so a plain click ends the
/// modal without extra wiring), wires `dialog`'s Escape key and affirmative
/// (Enter-default) behavior to Cancel/OK, and makes OK the visual default button.
#[must_use]
pub fn build_ok_cancel_buttons(dialog: Dialog, ok_label: &str) -> (Button, Button) {
	let ok_button = Button::builder(&dialog).with_id(ID_OK).with_label(ok_label).build();
	// TRANSLATORS: Label for the cancellation button
	let cancel_button = Button::builder(&dialog).with_id(ID_CANCEL).with_label(&t("Cancel")).build();
	dialog.set_escape_id(ID_CANCEL);
	dialog.set_affirmative_id(ID_OK);
	ok_button.set_default();
	(ok_button, cancel_button)
}

/// Appends an OK/Cancel button row to `content_sizer`.
///
/// Uses a native `wxStdDialogButtonSizer`, which reorders the buttons and applies
/// spacing to match the platform HIG (Cancel/OK on macOS, OK/Cancel on Windows).
pub fn add_ok_cancel_footer(content_sizer: BoxSizer, ok_button: Button, cancel_button: Button) {
	let button_sizer = StdDialogButtonSizerBuilder::new().build();
	button_sizer.add_button(&ok_button);
	button_sizer.add_button(&cancel_button);
	button_sizer.realize();
	content_sizer.add_sizer(&button_sizer, 0, SizerFlag::Expand | SizerFlag::All, dialog_padding(&ok_button));
}

/// Appends a single-button row (e.g. "Close") to `content_sizer`.
///
/// Uses a native `wxStdDialogButtonSizer`. The single-button counterpart to
/// [`add_ok_cancel_footer`], for dialogs that only need a dismiss action.
pub fn add_single_button_footer(content_sizer: BoxSizer, button: Button) {
	let button_sizer = StdDialogButtonSizerBuilder::new().build();
	button_sizer.add_button(&button);
	button_sizer.realize();
	content_sizer.add_sizer(&button_sizer, 0, SizerFlag::Expand | SizerFlag::All, dialog_padding(&button));
}

/// Binds `ctrl`'s Enter key (`TEXT_ENTER`) to confirm `dialog` as if its OK button were
/// clicked. For single-field entry dialogs where pressing Enter in the control should
/// submit the dialog.
// `ctrl` is taken by value (not consumed) to match how every widget handle in this
// file is passed, since callers always have a cheap Copy handle in hand already.
#[allow(clippy::needless_pass_by_value)]
pub fn bind_enter_confirms<W: WxEvtHandler>(dialog: Dialog, ctrl: W) {
	ctrl.bind_internal(EventType::TEXT_ENTER, move |event| {
		event.skip(false);
		dialog.end_modal(ID_OK);
	});
}

/// Shows a modal integer-entry dialog with a label and a bounded spin control.
///
/// Returns `None` if the user cancelled. Enter key submits.
#[must_use]
pub fn prompt_number(
	parent: &Frame,
	title: &str,
	label: &str,
	ok_label: &str,
	min: i32,
	max: i32,
	initial: i32,
) -> Option<i32> {
	let max = max.max(min);
	let dialog = Dialog::builder(parent, title).build();
	let lbl = StaticText::builder(&dialog).with_label(label).build();
	let ctrl = SpinCtrl::builder(&dialog)
		.with_range(min, max)
		.with_style(SpinCtrlStyle::Default | SpinCtrlStyle::ProcessEnter)
		.build();
	ctrl.set_value(initial.clamp(min, max));
	bind_enter_confirms(dialog, ctrl);
	let padding = dialog_padding(&dialog);
	let row = BoxSizer::builder(Orientation::Horizontal).build();
	row.add(&lbl, 0, SizerFlag::AlignCenterVertical | SizerFlag::Right, crate::dpi::scale(&dialog, 5));
	row.add(&ctrl, 1, SizerFlag::Expand, 0);
	let content = BoxSizer::builder(Orientation::Vertical).build();
	content.add_sizer(&row, 0, SizerFlag::Expand | SizerFlag::All, padding);
	let (ok_button, cancel_button) = build_ok_cancel_buttons(dialog, ok_label);
	add_ok_cancel_footer(content, ok_button, cancel_button);
	dialog.set_sizer_and_fit(content, true);
	dialog.centre();
	ctrl.set_focus();
	if dialog.show_modal() == ID_OK { Some(ctrl.value()) } else { None }
}

/// Shows a modal single-line text entry dialog.
///
/// Returns `None` if the user cancelled or entered only whitespace.
#[must_use]
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
