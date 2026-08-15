# wx-utils

A collection of ergonomic Rust utilities for desktop application development using wxDragon.

## Features

### AboutBoxBuilder

A fluent interface for creating and displaying native wxAboutBox dialogs.

* Zero manual FFI: no need to create, destroy, or cast `AboutDialogInfo` pointers yourself.
* Automatic C-string handling for every field you set.
* Takes `impl Into<String>`, so translation macros and plain strings both just work.

### Dialog button footers

Build and lay out a standard OK/Cancel row in two steps, meant to be used together:

* `build_ok_cancel_buttons` constructs the pair and wires the logical behavior that never varies by platform: `ID_OK`/`ID_CANCEL`, Escape cancels, Enter/default confirms. It does not decide on-screen order.
* `add_ok_cancel_footer` (and `add_single_button_footer` for a single dismiss button) then lays the buttons out with a native `wxStdDialogButtonSizer`, which is what actually enforces the platform HIG order (Cancel/OK on macOS, OK/Cancel on Windows), regardless of the order you passed the buttons in.
* `bind_enter_confirms` submits a dialog when Enter is pressed in a text or spin control.
* `DIALOG_PADDING` is the padding constant these functions use, exposed so your own sizers can match.

This is the exact pattern every OK/Cancel dialog in Paperback is built on.

The Cancel label is translated automatically (see Translation below); you only supply the OK label, since that varies by dialog ("OK", "Go", etc).

### Prompts and messages

* `prompt_number` shows a labeled spin control dialog and returns the entered value, or `None` if cancelled.
* `prompt_text` shows a single-line text entry dialog and returns the trimmed value, or `None` if cancelled or blank.
* `show_error` and `show_warning` show a modal message dialog with the matching icon.

## Translation

wx-utils is marked `[package.metadata.patois] translatable = true`. If your app uses [patois](https://github.com/trypsynth/patois) and calls `patois_build::gen_pot` over your workspace, wx-utils's own translatable strings (like "Cancel") are picked up into your app's catalog automatically. No extra setup needed.

## Quick Start

```rust
use wx_utils::AboutBoxBuilder;
use wxdragon::prelude::*;

// Inside a Frame event handler or menu callback
pub fn on_about(parent: &Frame) {
	AboutBoxBuilder::new(parent)
		.name("Paperback")
		.version(env!("CARGO_PKG_VERSION"))
		.description("An accessible, lightweight, fast ebook and document reader.")
		.copyright("Copyright (C) 2025-2026 Quin Gillespie")
		.website("https://paperback.dev")
		.add_developer("Quin Gillespie")
		.show();
}
```

```rust
use wx_utils::{DIALOG_PADDING, add_ok_cancel_footer, build_ok_cancel_buttons};
use wxdragon::prelude::*;

pub fn confirm_delete(parent: &Frame) -> bool {
	let dialog = Dialog::builder(parent, "Delete File").build();
	let label = StaticText::builder(&dialog).with_label("Delete this file permanently?").build();
	let (ok_button, cancel_button) = build_ok_cancel_buttons(dialog, "Delete");
	let content = BoxSizer::builder(Orientation::Vertical).build();
	content.add(&label, 0, SizerFlag::All, DIALOG_PADDING);
	add_ok_cancel_footer(content, ok_button, cancel_button);
	dialog.set_sizer_and_fit(content, true);
	dialog.centre();
	dialog.show_modal() == ID_OK
}
```

## Todo

* ReadmeOpener: open local/remote readmes in the user's default browser.
* Automatic help menu generation.
