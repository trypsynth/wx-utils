# wx-utils

Rust utilities for building desktop apps with wxDragon.

## Features

### AboutBoxBuilder

A fluent interface for creating and showing native wxAboutBox dialogs:

* No manual FFI: you don't create, destroy, or cast `AboutDialogInfo` pointers yourself.
* Every field you set is converted to a C string for you.
* Setters take `impl Into<String>`, so translation macros and plain strings both work.

### Dialog button footers

Build and lay out a standard OK/Cancel row in two steps, which are meant to be used together:

* `build_ok_cancel_buttons` constructs the pair and wires up the behavior that's the same on every platform: `ID_OK` and `ID_CANCEL`, Escape cancels, and Enter confirms. It doesn't decide the on-screen order.
* `add_ok_cancel_footer` (and `add_single_button_footer`, for a single dismiss button) then lays the buttons out with a native `wxStdDialogButtonSizer`. That sizer is what enforces the platform's order (Cancel then OK on macOS, OK then Cancel on Windows), whatever order you pass the buttons in.
* `bind_enter_confirms` submits a dialog when the user presses Enter in a text or spin control.
* `build_yes_no_buttons` and `add_yes_no_footer` are the Yes/No counterparts, and `confirm` is the whole dialog in one call.
* `DIALOG_PADDING` is the padding these functions use, exposed so your own sizers can match. `dialog_padding` returns it scaled for the display that the given window is on, using wxDragon's `WxWidget::from_dip_int`.

Each `build_*_buttons` function has an `_on` variant that parents the buttons to a `Panel` (or any other window) instead of to the dialog itself, for dialogs whose content lives in a panel.

The Cancel label is translated automatically (see [Translation](#translation)). You only supply the OK label, because it varies by dialog, for example "OK" or "Go".

### Why `confirm` instead of a Yes/No `MessageDialog`

On Windows, `MessageDialogStyle::YesNo` is the native task dialog, and the *operating system* labels its buttons, in the system language. An app translated into French and running on English Windows shows a French question above English Yes and No buttons. `confirm` builds real `Button`s labeled through patois, so they read in the app's own language like the rest of the dialog.

### Keyboard shortcuts

`shortcuts::prompt_for_shortcuts` is a complete Customize Keyboard Shortcuts dialog. It has a tab for each group of actions, a list of those actions with their current bindings, Set, Clear, Reset, and Reset All buttons, and conflict detection. Its capture dialog records whatever key combination you press, and announces it to a screen reader as you type.

Your app keeps its own action type and its own storage. Implement `shortcuts::ShortcutModel` over whatever your config already looks like, and the dialog does the rest, handing back an edited copy when the user confirms.

`ShortcutModel::tab_scope` needs the most care. It says when each tab's shortcuts are active, and two chords only conflict if their tabs can be active at the same time:

* `TabScope::Mode(n)`: active while the app is in mode `n`. Tabs in the same mode are views of one keymap, so a chord has to be unique across them, which is what tabs grouped by category need. Tabs in different modes never conflict, so the same key can mean different things in each, which is what tabs for separate input modes need.
* `TabScope::Global`: system-wide hotkeys, active whether or not the window has focus. They fire even over the app's own window, so they conflict with every other tab. The dialog requires Ctrl, Alt, or Win on each of these chords, and adds a Win checkbox to the capture dialog.

The scope also decides what Reset All covers: every tab in the same mode, or only the Global tab itself.

This is behind the `shortcuts` feature, which is off by default because it pulls in [live-region](https://github.com/trypsynth/live-region) for the announcements. The chord type is [key-chord](https://github.com/trypsynth/key-chord), re-exported as `shortcuts::KeyChord`.

### Global hotkeys

`global_hotkeys::GlobalHotkeys::register` registers chords system-wide, each paired with a value you choose, and calls you back with that value when one is pressed. Dropping the returned handle unregisters them all. It also returns the chords it couldn't register, usually because another program holds them, so you can tell the user instead of leaving a hotkey that silently does nothing.

The callback runs on the hotkey thread, so pass the value along (for example, down a channel that your UI thread drains) instead of touching widgets from it. Holding a hotkey down repeats it, like any other key.

This is behind the `global-hotkeys` feature. Only Windows is implemented. On other platforms, every chord comes back unregistered.

### Menus

* `menu_label` joins an item's text and its shortcut into the `"Open	Ctrl+O"` form that wx expects. An empty shortcut gives back a bare label instead of one with a trailing tab.
* `set_menu_item_label` relabels an existing item in a `MenuBar`, and does nothing if the ID isn't there, so you can relabel a menu that varies by platform in one pass.

### Durations

`format_duration_seconds` and `format_duration_ms` write a duration as a localized list of its nonzero parts, such as "1 hour, 5 minutes, 3 seconds". They use words instead of `1:05:03` because screen readers read this text aloud, and a colon-separated form is likely to be read as a time of day.

### Prompts and messages

* `prompt_number` shows a dialog with a labeled spin control, and returns the value entered, or `None` if the user cancels.
* `prompt_text` shows a single-line text entry dialog, and returns the trimmed value, or `None` if the user cancels or leaves it blank.
* `show_error` and `show_warning` show a modal message dialog with the matching icon.

## Translation

wx-utils is marked `[package.metadata.patois] translatable = true`. If your app uses [patois](https://github.com/trypsynth/patois) and calls `patois_build::gen_pot` over your workspace, wx-utils's own translatable strings, such as "Cancel", are added to your app's catalog automatically. You don't need to set anything up.

## Quick start

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
use wx_utils::confirm;
use wxdragon::prelude::*;

pub fn confirm_delete(parent: &Frame) -> bool {
	confirm(parent, "Delete this file permanently?", "Delete File")
}
```

For a dialog of your own, the button helpers lay out the footer:

```rust
use wx_utils::{add_ok_cancel_footer, build_ok_cancel_buttons, dialog_padding};
use wxdragon::prelude::*;

pub fn prompt_for_name(parent: &Frame) -> Option<String> {
	let dialog = Dialog::builder(parent, "Rename").build();
	let padding = dialog_padding(&dialog);
	let entry = TextCtrl::builder(&dialog).build();
	let (ok_button, cancel_button) = build_ok_cancel_buttons(&dialog, "Rename");
	let content = BoxSizer::builder(Orientation::Vertical).build();
	content.add(&entry, 0, SizerFlag::Expand | SizerFlag::All, padding);
	add_ok_cancel_footer(content, ok_button, cancel_button);
	dialog.set_sizer_and_fit(content, true);
	dialog.centre();
	if dialog.show_modal() == ID_OK { Some(entry.get_value()) } else { None }
}
```

## Planned

* ReadmeOpener: open local or remote readmes in the user's default browser.
* Automatic help menu generation.
