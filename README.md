# wx-utils

A collection of ergonomic Rust utilities for desktop applicatoin development using wxDragon.

## Current Features

### AboutBoxBuilder

The `AboutBoxBuilder` provides a fluent interface for creating and displaying native wxAboutBox dialogs.

* Zero Manual FFI: No need to manually create, destroy, or cast `AboutDialogInfo` pointers.
* Automatic C-String Handling: Converts Rust Strings and &str to CString safely under the hood.
* Flexible Input: Supports `impl Into<String>`, making it play nicely with standard strings and translation macros.

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

## Todo

* ReadmeOpener: Open local/remote readme's in the user's default browser.
* TranslationManager: Streamlined integration for multi-language desktop apps as seen in Paperback.
* Automatic help menu generation.
