//! Whether the keyboard layout in use can press a chord at all.
//!
//! A chord names a key by the character it types without Shift. A punctuation character can live
//! anywhere on another layout, or only behind Shift, as `;` does on a Finnish keyboard; a chord on
//! it then matches no key event, and the list says so rather than showing a shortcut that does
//! nothing.

use super::KeyChord;

/// Whether some key on the current layout reports `chord`'s key, so pressing it with the chord's
/// modifiers can match. Only punctuation is checked; letters, digits and named keys are always
/// there.
#[cfg(windows)]
pub(super) fn is_typable(chord: &KeyChord) -> bool {
	use windows::Win32::UI::Input::KeyboardAndMouse::{
		MAPVK_VK_TO_CHAR, MAPVK_VK_TO_VSC, MapVirtualKeyW, VK_OEM_1, VK_OEM_2, VK_OEM_3, VK_OEM_4, VK_OEM_5, VK_OEM_6,
		VK_OEM_7, VK_OEM_8, VK_OEM_102, VK_OEM_COMMA, VK_OEM_MINUS, VK_OEM_PERIOD, VK_OEM_PLUS,
	};
	// The keys wxMSW reports by the character they type rather than by a fixed key code.
	const OEM_KEYS: [u16; 13] = [
		VK_OEM_1.0,
		VK_OEM_PLUS.0,
		VK_OEM_COMMA.0,
		VK_OEM_MINUS.0,
		VK_OEM_PERIOD.0,
		VK_OEM_2.0,
		VK_OEM_3.0,
		VK_OEM_4.0,
		VK_OEM_5.0,
		VK_OEM_6.0,
		VK_OEM_7.0,
		VK_OEM_8.0,
		VK_OEM_102.0,
	];
	if !is_punctuation(chord) {
		return true;
	}
	// The key code wxMSW gives the key: its unshifted character, nothing for a dead key, and for
	// a character outside Latin-1 the one the same key types on a US layout.
	let key_code = |vk: u16| {
		let mapped = unsafe { MapVirtualKeyW(u32::from(vk), MAPVK_VK_TO_CHAR) };
		if mapped == 0 || mapped & 0x8000_0000 != 0 {
			return None;
		}
		if mapped <= 0xFF {
			return i32::try_from(mapped).ok();
		}
		let us = match unsafe { MapVirtualKeyW(u32::from(vk), MAPVK_VK_TO_VSC) } {
			0x0C => '-',
			0x0D => '=',
			0x1A => '[',
			0x1B => ']',
			0x27 => ';',
			0x28 => '\'',
			0x29 => '`',
			0x2B => '\\',
			0x33 => ',',
			0x34 => '.',
			0x35 => '/',
			_ => return None,
		};
		Some(us as i32)
	};
	OEM_KEYS
		.into_iter()
		.filter_map(key_code)
		.any(|code| chord.matches(code, chord.ctrl || chord.raw_ctrl, chord.alt, chord.shift))
}

/// Other platforms report a key by the character it types, so a chord there is never stranded on
/// a key the layout lacks.
#[cfg(not(windows))]
pub(super) const fn is_typable(_chord: &KeyChord) -> bool {
	true
}

#[cfg(windows)]
fn is_punctuation(chord: &KeyChord) -> bool {
	let mut chars = chord.key.chars();
	matches!((chars.next(), chars.next()), (Some(ch), None) if !ch.is_ascii_alphanumeric() && ch != ' ')
}

#[cfg(all(test, windows))]
mod tests {
	use super::*;

	#[test]
	fn letters_and_named_keys_are_never_checked() {
		assert!(is_typable(&KeyChord::new(true, false, false, "Q")));
		assert!(is_typable(&KeyChord::new(true, false, false, "F5")));
		assert!(is_typable(&KeyChord::new(false, false, false, "Space")));
	}

	#[test]
	fn a_comma_is_on_the_keyboard() {
		assert!(is_typable(&KeyChord::new(true, false, false, ",")));
	}

	/// A US layout, which these tests assume, types `:` only with Shift, as a Finnish one does `;`.
	#[test]
	fn a_character_behind_shift_is_not() {
		assert!(!is_typable(&KeyChord::new(true, false, false, ":")));
	}
}
