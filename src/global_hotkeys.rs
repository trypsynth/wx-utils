//! System-wide hotkeys: chords that fire whether or not the app's window has focus.
//!
//! [`GlobalHotkeys::register`] takes each chord together with a value of the app's choosing, and
//! hands that value back to a callback whenever the chord is pressed. Dropping the returned
//! handle gives every chord back to the system.
//!
//! The callback runs on the hotkey thread, not the UI thread, so it should only pass the value
//! along, for example down a channel the UI thread drains, rather than touch any widget.
//!
//! Only Windows has an implementation. Elsewhere nothing is registered and every chord comes
//! back as unregistered, so an app can report that rather than branch on the platform itself.

use key_chord::KeyChord;

/// Registered hotkeys. Dropping it stops the hotkey thread and unregisters every chord.
pub struct GlobalHotkeys {
	#[cfg(windows)]
	thread_id: u32,
	#[cfg(windows)]
	handle: Option<std::thread::JoinHandle<()>>,
}

impl GlobalHotkeys {
	/// Registers each chord in `bindings` system-wide, calling `on_hotkey` with its value when
	/// it's pressed.
	///
	/// Returns the handle, or `None` when nothing could be registered, along with the chords that
	/// couldn't be. That is nearly always because another program already holds them, or because
	/// the key doesn't exist on this keyboard layout. Report those: a hotkey that silently does
	/// nothing is worse than one that says why.
	pub fn register<T: Clone + Send + 'static>(
		bindings: Vec<(KeyChord, T)>,
		on_hotkey: impl Fn(T) + Send + 'static,
	) -> (Option<Self>, Vec<KeyChord>) {
		platform::register(bindings, on_hotkey)
	}
}

#[cfg(windows)]
impl Drop for GlobalHotkeys {
	fn drop(&mut self) {
		platform::stop(self.thread_id, self.handle.take());
	}
}

#[cfg(windows)]
mod platform {
	use std::{sync::mpsc, thread};

	use key_chord::KeyChord;
	use windows::Win32::{
		Foundation::{LPARAM, WPARAM},
		System::Threading::GetCurrentThreadId,
		UI::{
			Input::KeyboardAndMouse::{
				HOT_KEY_MODIFIERS, MOD_ALT, MOD_CONTROL, MOD_SHIFT, MOD_WIN, RegisterHotKey, UnregisterHotKey,
				VkKeyScanW,
			},
			WindowsAndMessaging::{GetMessageW, MSG, PostThreadMessageW, WM_HOTKEY, WM_QUIT},
		},
	};

	use super::GlobalHotkeys;

	pub(super) fn register<T: Clone + Send + 'static>(
		bindings: Vec<(KeyChord, T)>,
		on_hotkey: impl Fn(T) + Send + 'static,
	) -> (Option<GlobalHotkeys>, Vec<KeyChord>) {
		let mut failed = Vec::new();
		let mut wanted = Vec::new();
		for (chord, value) in bindings {
			match to_hotkey(&chord) {
				Some((modifiers, key)) => wanted.push((chord, modifiers, key, value)),
				None => failed.push(chord),
			}
		}
		if wanted.is_empty() {
			return (None, failed);
		}
		let (tx, rx) = mpsc::channel();
		let handle = thread::spawn(move || {
			// A hotkey belongs to the thread that registered it, and only that thread's message
			// queue receives WM_HOTKEY, so registering, waiting and unregistering all happen here.
			let thread_id = unsafe { GetCurrentThreadId() };
			let mut registered: Vec<(i32, T)> = Vec::new();
			let mut failed_here = Vec::new();
			for (index, (chord, modifiers, key, value)) in wanted.into_iter().enumerate() {
				// Ids only have to be unique within this thread; 0xBFFF is the top of the range an
				// application may use.
				let Some(id) = i32::try_from(index + 1).ok().filter(|&id| id <= 0xBFFF) else {
					failed_here.push(chord);
					continue;
				};
				if unsafe { RegisterHotKey(None, id, modifiers, key) }.is_ok() {
					registered.push((id, value));
				} else {
					failed_here.push(chord);
				}
			}
			let _ = tx.send((thread_id, failed_here));
			let mut msg = MSG::default();
			loop {
				let result = unsafe { GetMessageW(&raw mut msg, None, 0, 0) };
				if result.0 <= 0 {
					break;
				}
				if msg.message == WM_HOTKEY
					&& let Ok(id) = i32::try_from(msg.wParam.0)
					&& let Some((_, value)) = registered.iter().find(|(registered_id, _)| *registered_id == id)
				{
					on_hotkey(value.clone());
				}
			}
			for (id, _) in registered {
				unsafe {
					let _ = UnregisterHotKey(None, id);
				}
			}
		});
		let Ok((thread_id, failed_here)) = rx.recv() else {
			let _ = handle.join();
			return (None, failed);
		};
		failed.extend(failed_here);
		(Some(GlobalHotkeys { thread_id, handle: Some(handle) }), failed)
	}

	pub(super) fn stop(thread_id: u32, handle: Option<thread::JoinHandle<()>>) {
		// Ends GetMessageW, which unregisters everything on its way out.
		unsafe {
			let _ = PostThreadMessageW(thread_id, WM_QUIT, WPARAM(0), LPARAM(0));
		}
		if let Some(handle) = handle {
			let _ = handle.join();
		}
	}

	/// `None` for a key this keyboard layout can't produce.
	pub(super) fn to_hotkey(chord: &KeyChord) -> Option<(HOT_KEY_MODIFIERS, u32)> {
		// Holding a hotkey down repeats it, like any other key, so navigation keys can be held to skim.
		let mut modifiers = HOT_KEY_MODIFIERS(0);
		if chord.ctrl || chord.raw_ctrl {
			modifiers |= MOD_CONTROL;
		}
		if chord.alt {
			modifiers |= MOD_ALT;
		}
		if chord.shift {
			modifiers |= MOD_SHIFT;
		}
		if chord.win {
			modifiers |= MOD_WIN;
		}
		virtual_key(&chord.key).map(|key| (modifiers, key))
	}

	pub(super) fn virtual_key(name: &str) -> Option<u32> {
		// Named keys first, since none of them is a single character.
		let named = match name {
			"Enter" => 0x0D,
			"Tab" => 0x09,
			"Space" => 0x20,
			"Backspace" => 0x08,
			"Delete" => 0x2E,
			"Escape" => 0x1B,
			"Home" => 0x24,
			"End" => 0x23,
			"PageUp" => 0x21,
			"PageDown" => 0x22,
			"Left" => 0x25,
			"Up" => 0x26,
			"Right" => 0x27,
			"Down" => 0x28,
			_ => 0,
		};
		if named != 0 {
			return Some(named);
		}
		if let Some(digits) = name.strip_prefix('F')
			&& let Ok(number) = digits.parse::<u32>()
			&& (1..=24).contains(&number)
		{
			return Some(0x70 + number - 1);
		}
		let ch = name.chars().next().filter(|_| name.chars().count() == 1)?;
		let code = u16::try_from(u32::from(ch)).ok()?;
		let scanned = unsafe { VkKeyScanW(code) };
		// The low byte is the key; the high byte says which modifiers typing it needs, which is
		// not this chord's business.
		if scanned == -1 {
			return None;
		}
		Some(u32::from(u16::try_from(scanned & 0xFF).ok()?))
	}
}

#[cfg(not(windows))]
mod platform {
	use key_chord::KeyChord;

	use super::GlobalHotkeys;

	pub(super) fn register<T>(
		bindings: Vec<(KeyChord, T)>,
		_on_hotkey: impl Fn(T) + Send + 'static,
	) -> (Option<GlobalHotkeys>, Vec<KeyChord>) {
		(None, bindings.into_iter().map(|(chord, _)| chord).collect())
	}
}

#[cfg(all(test, windows))]
mod tests {
	use key_chord::KeyChord;
	use windows::Win32::UI::Input::KeyboardAndMouse::{MOD_ALT, MOD_CONTROL, MOD_SHIFT, MOD_WIN};

	use super::platform::{to_hotkey, virtual_key};

	#[test]
	fn a_chord_becomes_modifiers_and_a_key() {
		let (modifiers, key) = to_hotkey(&KeyChord::new(false, true, true, "Enter")).expect("should convert");
		assert_eq!(modifiers, MOD_ALT | MOD_SHIFT);
		assert_eq!(key, 0x0D);
	}

	#[test]
	fn win_becomes_the_win_modifier() {
		let chord = KeyChord::new(true, false, false, "Up").with_win(true);
		let (modifiers, key) = to_hotkey(&chord).expect("should convert");
		assert_eq!(modifiers, MOD_CONTROL | MOD_WIN);
		assert_eq!(key, 0x26);
	}

	#[test]
	fn a_function_key_lands_in_its_own_range() {
		assert_eq!(virtual_key("F1"), Some(0x70));
		assert_eq!(virtual_key("F12"), Some(0x7B));
		assert_eq!(virtual_key("F25"), None);
	}
}
