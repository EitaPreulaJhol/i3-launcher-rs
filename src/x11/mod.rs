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
    ///
    /// With `single_instance`, ownership of the `_I3_LAUNCHER_RUNNING`
    /// selection is claimed for the display. `Ok(None)` means another
    /// launcher already owns it (the caller should exit quietly) and avoids
    /// stacked windows that could not grab the keyboard.
    pub fn create(cfg: &Config, single_instance: bool) -> Result<Option<Self>> {
        let background = cfg.style.bg;
        let (conn, screen_num) = x11rb::connect(None).context("could not open the X11 display")?;
        let screen = &conn.setup().roots[screen_num];
        let root = screen.root;
        let depth = screen.root_depth;
        let keymap = Keymap::load(&conn)?;

        let instance_atom = intern_atom(&conn, b"_I3_LAUNCHER_RUNNING")?;
        if single_instance {
            let owner = instance_owner(&conn, instance_atom)?;
            if owner != x11rb::NONE {
                // Treat another invocation as a request to bring the existing
                // launcher back, rather than silently doing nothing.
                reactivate_instance(&conn, owner);
                return Ok(None);
            }
        }

        // Remember the real window that was focused so it can be restored on
        // exit. `focus` 0/1 means None/PointerRoot and is not restorable.
        let previous_focus = conn
            .get_input_focus()
            .ok()
            .and_then(|cookie| cookie.reply().ok())
            .map(|reply| reply.focus)
            .filter(|focus| *focus > 1 && *focus != root);
        debug(|| match previous_focus {
            Some(window) => format!("previous focus: {window:#x}"),
            None => "previous focus: none".to_string(),
        });

        let monitors = monitor::list(&conn, root, screen.width_in_pixels, screen.height_in_pixels);
        let monitor_index = monitor::select(&conn, root, &monitors, &cfg.monitor.target);
        let monitor = &monitors[monitor_index];
        let rect = monitor::bar_rect(monitor.rect, cfg);
        let monitor_name = monitor.name.clone();

        let win = conn.generate_id().context("XAllocID failed")?;
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
        .context("could not create the launcher window")?;

        if single_instance && !claim_instance_selection(&conn, instance_atom, win)? {
            // Another instance won the atomic claim; this window was never
            // mapped or focused, and dropping `conn` destroys it.
            if let Ok(owner) = instance_owner(&conn, instance_atom) {
                if owner != x11rb::NONE {
                    reactivate_instance(&conn, owner);
                }
            }
            return Ok(None);
        }

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
                    "i3-launcher: warning: keyboard grab refused ({:?})",
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

        Ok(Some(Self {
            conn,
            win,
            gc,
            back_buffer,
            rect,
            depth,
            keymap,
            monitor_name,
            previous_focus,
        }))
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
            .context("PutImage failed")?;
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
                        format!("focus restored: {previous:#x} (current: {focus:x?})")
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

/// Current owner of `atom`, or [`x11rb::NONE`] when the selection is free.
fn instance_owner<C: Connection>(conn: &C, atom: u32) -> Result<Window> {
    Ok(conn.get_selection_owner(atom)?.reply()?.owner)
}

/// Raise and focus the current instance. Its FocusIn handler will reacquire
/// the keyboard grab on the owning connection.
fn reactivate_instance(conn: &RustConnection, window: Window) {
    let _ = conn.configure_window(
        window,
        &x11rb::protocol::xproto::ConfigureWindowAux::new()
            .stack_mode(x11rb::protocol::xproto::StackMode::ABOVE),
    );
    let _ = conn.set_input_focus(InputFocus::POINTER_ROOT, window, CURRENT_TIME);
    let _ = conn.flush();
}

/// Atomically claim the instance selection, so two simultaneous launches
/// cannot both observe an empty owner and proceed to grab/focus the keyboard.
fn claim_instance_selection<C: Connection>(conn: &C, atom: u32, window: Window) -> Result<bool> {
    conn.grab_server()?.check()?;

    let claim = (|| -> Result<bool> {
        if instance_owner(conn, atom)? != x11rb::NONE {
            return Ok(false);
        }
        conn.set_selection_owner(window, atom, CURRENT_TIME)?
            .check()?;
        Ok(instance_owner(conn, atom)? == window)
    })();

    // Always release the server grab, including when an ownership request
    // fails, or the entire X session would appear frozen.
    let ungrab = conn
        .ungrab_server()
        .context("could not send X server ungrab")
        .and_then(|cookie| cookie.check().context("could not release X server grab"));

    match claim {
        Ok(claimed) => {
            ungrab?;
            Ok(claimed)
        }
        Err(error) => {
            let _ = ungrab;
            Err(error)
        }
    }
}
