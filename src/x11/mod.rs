//! Minimal X11 layer: an override-redirect strip window, keyboard grab and
//! software presentation through `PutImage`.

pub mod keys;
pub mod monitor;

use anyhow::{Context, Result};
use x11rb::connection::Connection;
use x11rb::protocol::xproto::{
    AtomEnum, ConnectionExt as XprotoExt, CreateGCAux, CreateWindowAux, EventMask, Gcontext,
    GrabMode, GrabStatus, ImageFormat, InputFocus, MapState, Pixmap, PropMode, Window, WindowClass,
};
use x11rb::protocol::Event;
use x11rb::rust_connection::RustConnection;
use x11rb::CURRENT_TIME;
use xkeysym::Keysym;

use crate::config::Config;
use keys::{Keymap, Mods};
use monitor::Rect;
use x11rb::wrapper::ConnectionExt as WrapperExt;

/// Events the launcher cares about.
#[derive(Debug, Clone)]
pub enum InputEvent {
    Key { sym: Keysym, mods: Mods },
    Click { x: f32, y: f32, button: u8 },
    Scroll { up: bool },
    Expose,
    FocusIn,
    Quit,
}

pub struct X11 {
    conn: RustConnection,
    win: Window,
    gc: Gcontext,
    back_buffer: Pixmap,
    rect: Rect,
    depth: u8,
    keymap: Keymap,
    monitor_name: String,
    /// Window that had the input focus before the launcher opened.
    previous_focus: Option<Window>,
}

impl X11 {
    /// Connect to `$DISPLAY`, pick the configured monitor and create the
    /// strip window (override-redirect + exclusive keyboard grab).
    pub fn create(cfg: &Config) -> Result<Self> {
        let background = cfg.style.bg;
        let (conn, screen_num) =
            x11rb::connect(None).context("não foi possível abrir o display X11")?;
        let screen = &conn.setup().roots[screen_num];
        let root = screen.root;
        let depth = screen.root_depth;
        let keymap = Keymap::load(&conn)?;

        // Remember the real window that was focused so it can be restored on
        // exit. `focus` 0/1 means None/PointerRoot and is not restorable.
        let previous_focus = conn
            .get_input_focus()
            .ok()
            .and_then(|cookie| cookie.reply().ok())
            .map(|reply| reply.focus)
            .filter(|focus| *focus > 1 && *focus != root);
        debug(|| match previous_focus {
            Some(window) => format!("foco anterior: {window:#x}"),
            None => "foco anterior: nenhum".to_string(),
        });

        let monitors = monitor::list(&conn, root, screen.width_in_pixels, screen.height_in_pixels);
        let monitor_index = monitor::select(&conn, root, &monitors, &cfg.monitor.target);
        let monitor = &monitors[monitor_index];
        let rect = monitor::bar_rect(monitor.rect, cfg);
        let monitor_name = monitor.name.clone();

        let win = conn.generate_id().context("XAllocID falhou")?;
        let background_pixel =
            ((background.r as u32) << 16) | ((background.g as u32) << 8) | background.b as u32;
        conn.create_window(
            depth,
            win,
            root,
            rect.x,
            rect.y,
            rect.width,
            rect.height,
            0,
            WindowClass::INPUT_OUTPUT,
            0,
            &CreateWindowAux::new()
                // Never managed by i3: no borders, no gaps, no tiling.
                .override_redirect(1)
                .background_pixel(background_pixel)
                .border_pixel(background_pixel)
                .event_mask(
                    EventMask::EXPOSURE
                        | EventMask::KEY_PRESS
                        | EventMask::BUTTON_PRESS
                        | EventMask::FOCUS_CHANGE,
                ),
        )
        .context("não foi possível criar a janela do launcher")?;

        // Identification, useful for debug tools and window rules.
        conn.change_property8(
            PropMode::REPLACE,
            win,
            AtomEnum::WM_CLASS,
            AtomEnum::STRING,
            b"i3-launcher\0I3-launcher\0",
        )?;
        let net_wm_name = intern_atom(&conn, b"_NET_WM_NAME")?;
        let utf8_string = intern_atom(&conn, b"UTF8_STRING")?;
        conn.change_property8(
            PropMode::REPLACE,
            win,
            net_wm_name,
            utf8_string,
            b"i3-launcher",
        )?;

        conn.map_window(win)?;
        conn.configure_window(
            win,
            &x11rb::protocol::xproto::ConfigureWindowAux::new()
                .stack_mode(x11rb::protocol::xproto::StackMode::ABOVE),
        )?;

        // Exclusive keyboard grab: all key presses come to us, even if the
        // user moves the focus. Mouse stays free for other clients.
        let grab =
            conn.grab_keyboard(false, win, CURRENT_TIME, GrabMode::ASYNC, GrabMode::ASYNC)?;
        if let Ok(reply) = grab.reply() {
            if reply.status != GrabStatus::SUCCESS {
                eprintln!(
                    "i3-launcher: aviso: grab de teclado recusado ({:?})",
                    reply.status
                );
            }
        }
        conn.set_input_focus(InputFocus::POINTER_ROOT, win, CURRENT_TIME)?;

        let back_buffer = conn.generate_id()?;
        conn.create_pixmap(depth, back_buffer, win, rect.width, rect.height)?;
        let gc = conn.generate_id()?;
        conn.create_gc(gc, back_buffer, &CreateGCAux::new().graphics_exposures(0))?;
        conn.flush()?;

        Ok(Self {
            conn,
            win,
            gc,
            back_buffer,
            rect,
            depth,
            keymap,
            monitor_name,
            previous_focus,
        })
    }

    pub fn monitor_name(&self) -> &str {
        &self.monitor_name
    }

    pub fn size(&self) -> (u32, u32) {
        (self.rect.width as u32, self.rect.height as u32)
    }

    /// Present a BGRA buffer (4 bytes per pixel, as produced by the painter).
    pub fn blit(&self, pixels: &[u8]) -> Result<()> {
        let width = self.rect.width;
        let height = self.rect.height;
        debug_assert_eq!(pixels.len(), width as usize * height as usize * 4);
        self.conn
            .put_image(
                ImageFormat::Z_PIXMAP,
                self.back_buffer,
                self.gc,
                width,
                height,
                0,
                0,
                0,
                self.depth,
                pixels,
            )
            .context("PutImage falhou")?;
        self.conn.copy_area(
            self.back_buffer,
            self.win,
            self.gc,
            0,
            0,
            0,
            0,
            width,
            height,
        )?;
        self.conn.flush()?;
        Ok(())
    }

    pub fn wait_for_event(&self) -> Result<Event> {
        Ok(self.conn.wait_for_event()?)
    }

    pub fn poll_event(&self) -> Result<Option<Event>> {
        Ok(self.conn.poll_for_event()?)
    }

    /// Re-grab the keyboard after focus changes or external grabs.
    pub fn regrab(&self) {
        if let Ok(grab) = self.conn.grab_keyboard(
            false,
            self.win,
            CURRENT_TIME,
            GrabMode::ASYNC,
            GrabMode::ASYNC,
        ) {
            let _ = grab.reply();
        }
        let _ = self.conn.set_input_focus(
            x11rb::protocol::xproto::InputFocus::PARENT,
            self.win,
            CURRENT_TIME,
        );
        let _ = self.conn.flush();
    }

    pub fn translate(&self, event: Event) -> Option<InputEvent> {
        match event {
            Event::KeyPress(event) => {
                let mods = Mods::from_state(event.state.into());
                let sym = self.keymap.keysym(event.detail, mods.shift)?;
                Some(InputEvent::Key { sym, mods })
            }
            Event::ButtonPress(event) => match event.detail {
                4 => Some(InputEvent::Scroll { up: true }),
                5 => Some(InputEvent::Scroll { up: false }),
                button => Some(InputEvent::Click {
                    x: event.event_x as f32,
                    y: event.event_y as f32,
                    button,
                }),
            },
            Event::Expose(event) if event.count == 0 => Some(InputEvent::Expose),
            Event::FocusIn(_) => Some(InputEvent::FocusIn),
            Event::DestroyNotify(_) => Some(InputEvent::Quit),
            _ => None,
        }
    }
}

impl Drop for X11 {
    fn drop(&mut self) {
        let _ = self.conn.ungrab_keyboard(CURRENT_TIME);

        // Did the launcher still own the focus? If the user clicked anywhere
        // else while it was open, that window must keep the focus.
        let current_focus = self
            .conn
            .get_input_focus()
            .ok()
            .and_then(|cookie| cookie.reply().ok())
            .map(|reply| reply.focus);
        let owned_focus = current_focus == Some(self.win);

        let _ = self.conn.free_gc(self.gc);
        let _ = self.conn.free_pixmap(self.back_buffer);
        let _ = self.conn.destroy_window(self.win);

        // Destroying the focused window makes the server revert the focus
        // (usually to PointerRoot), so restore the previous window *after*
        // the destroy, while the connection is still open.
        if owned_focus {
            if let Some(previous) = self.previous_focus {
                let viewable = self
                    .conn
                    .get_window_attributes(previous)
                    .ok()
                    .and_then(|cookie| cookie.reply().ok())
                    .map(|attributes| attributes.map_state == MapState::VIEWABLE)
                    .unwrap_or(false);
                if viewable {
                    let _ =
                        self.conn
                            .set_input_focus(InputFocus::POINTER_ROOT, previous, CURRENT_TIME);
                    debug(|| {
                        let focus = self
                            .conn
                            .get_input_focus()
                            .ok()
                            .and_then(|cookie| cookie.reply().ok())
                            .map(|reply| reply.focus);
                        format!("foco restaurado: {previous:#x} (atual: {focus:x?})")
                    });
                }
            }
        }

        let _ = self.conn.flush();
    }
}

fn debug(message: impl Fn() -> String) {
    if std::env::var_os("I3L_DEBUG").is_some() {
        eprintln!("i3-launcher: {}", message());
    }
}

pub fn intern_atom<C: Connection>(conn: &C, name: &[u8]) -> Result<u32> {
    Ok(conn.intern_atom(false, name)?.reply()?.atom)
}
