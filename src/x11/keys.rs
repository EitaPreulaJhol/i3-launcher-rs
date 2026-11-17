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
            .context("GetKeyboardMapping falhou")?
            .reply()
            .context("GetKeyboardMapping sem resposta")?;
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
