//! Keyboard mapping and modifier decoding (core X11 `GetKeyboardMapping`).

use anyhow::{Context, Result};
use x11rb::connection::Connection;
use x11rb::protocol::xproto::{ConnectionExt, ModMask};
use xkeysym::{KeyCode, Keysym};

/// Modifier state relevant to the launcher.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Mods {
    pub shift: bool,
    pub ctrl: bool,
    pub alt: bool,
    pub logo: bool,
}

impl Mods {
    pub fn from_state(state: u16) -> Self {
        Self {
            shift: state & u16::from(ModMask::SHIFT) != 0,
            ctrl: state & u16::from(ModMask::CONTROL) != 0,
            alt: state & u16::from(ModMask::M1) != 0,
            logo: state & u16::from(ModMask::M4) != 0,
        }
    }
}

/// Snapshot of the keyboard mapping: keycode → keysym per shift level.
pub struct Keymap {
    min_keycode: u8,
    keysyms_per_keycode: u8,
    keysyms: Vec<u32>,
}

impl Keymap {
    pub fn load<C: Connection>(conn: &C) -> Result<Self> {
        let setup = conn.setup();
        let min_keycode = setup.min_keycode;
        let count = setup.max_keycode.saturating_sub(min_keycode) + 1;
        let reply = conn
            .get_keyboard_mapping(min_keycode, count)
            .context("GetKeyboardMapping failed")?
            .reply()
            .context("GetKeyboardMapping returned no reply")?;
        Ok(Self {
            min_keycode,
            keysyms_per_keycode: reply.keysyms_per_keycode,
            keysyms: reply.keysyms,
        })
    }

    /// Keysym for `keycode` at shift level 0/1, with the usual fallbacks.
    pub fn keysym(&self, keycode: u8, shift: bool) -> Option<Keysym> {
        if keycode < self.min_keycode {
            return None;
        }
        let index = (keycode - self.min_keycode) as usize * self.keysyms_per_keycode as usize;
        if index >= self.keysyms.len() {
            return None;
        }
        let column = if shift { 1 } else { 0 };
        let keysym = xkeysym::keysym(
            KeyCode::new(keycode.into()),
            column,
            KeyCode::new(self.min_keycode.into()),
            self.keysyms_per_keycode,
            &self.keysyms,
        )?;
        (keysym != Keysym::NoSymbol).then_some(keysym)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const A: u32 = 0x61; // XK_a
    const SHIFT_A: u32 = 0x41; // XK_A
    const ONE: u32 = 0x31; // XK_1
    const EXCLAM: u32 = 0x21; // XK_exclam
    const RETURN: u32 = 0xff0d; // XK_Return

    /// Keycodes 8.. with two columns: `a/A`, `1/!`, `Return/-`, `-/-`.
    fn keymap() -> Keymap {
        Keymap {
            min_keycode: 8,
            keysyms_per_keycode: 2,
            keysyms: vec![A, SHIFT_A, ONE, EXCLAM, RETURN, 0, 0, 0],
        }
    }

    #[test]
    fn mods_decode_the_relevant_bits() {
        assert_eq!(Mods::from_state(0), Mods::default());
        let all = u16::from(ModMask::SHIFT)
            | u16::from(ModMask::CONTROL)
            | u16::from(ModMask::M1)
            | u16::from(ModMask::M4);
        assert_eq!(
            Mods::from_state(all),
            Mods {
                shift: true,
                ctrl: true,
                alt: true,
                logo: true,
            }
        );
        // Lock (Caps) and Mod2 (NumLock) are ignored.
        let ignored = u16::from(ModMask::LOCK) | u16::from(ModMask::M2);
        assert_eq!(Mods::from_state(ignored), Mods::default());
        assert!(Mods::from_state(u16::from(ModMask::CONTROL)).ctrl);
    }

    #[test]
    fn keysym_respects_shift_level() {
        let map = keymap();
        assert_eq!(map.keysym(8, false), Some(Keysym::new(A)));
        assert_eq!(map.keysym(8, true), Some(Keysym::new(SHIFT_A)));
        assert_eq!(map.keysym(9, false), Some(Keysym::new(ONE)));
        assert_eq!(map.keysym(9, true), Some(Keysym::new(EXCLAM)));
    }

    #[test]
    fn shift_falls_back_to_the_base_symbol() {
        // Return has no shifted symbol; X11 rules reuse column 0.
        assert_eq!(keymap().keysym(10, true), Some(Keysym::new(RETURN)));
    }

    #[test]
    fn unmapped_and_out_of_range_keycodes_are_none() {
        let map = keymap();
        assert_eq!(map.keysym(11, false), None);
        assert_eq!(map.keysym(7, false), None);
        assert_eq!(map.keysym(12, false), None);
        assert_eq!(map.keysym(255, true), None);
    }
}
