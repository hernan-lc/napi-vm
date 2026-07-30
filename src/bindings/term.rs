//! Terminal mouse support: SGR (mode 1006) event parsing and the ANSI
//! escape sequences needed to enable/disable tracking.
//!
//! Gated behind the `mouse` Cargo feature — off by default because mouse
//! mode changes global terminal behavior (breaks scrollback selection in
//! some emulators) and is meaningless in CI / piped environments.
//!
//! The I/O stays in Node.js (`process.stdin` / `process.stdout`); this
//! module only provides the *parser* and the *escape strings*, so the
//! event loop is never blocked on the Rust side.

use napi::bindgen_prelude::Buffer;
use napi_derive::napi;

/// A parsed SGR mouse event, returned to JS as a plain object.
#[napi(object)]
#[derive(Debug, Clone)]
pub struct MouseEvent {
    /// `"press"`, `"release"`, `"move"`, `"scroll-up"`, or `"scroll-down"`.
    pub kind: String,
    /// 0-based column.
    pub x: u32,
    /// 0-based row.
    pub y: u32,
    /// Button index: 0 = left, 1 = middle, 2 = right.
    /// Always 0 for scroll events.
    pub button: u32,
    pub shift: bool,
    pub alt: bool,
    pub ctrl: bool,
}

/// Parse an SGR-encoded mouse event from raw stdin bytes.
///
/// SGR format (mode 1006):
///   press / move:  `\x1b[<Cb;Cx;CyM`
///   release:       `\x1b[<Cb;Cx;Cym`
///
/// Returns `None` when the buffer is not a valid SGR mouse sequence, so
/// callers can fall through to keyboard handling.
#[napi]
pub fn parse_mouse_event(buf: Buffer) -> Option<MouseEvent> {
    let bytes: &[u8] = buf.as_ref();

    // Minimum: \x1b [ < 0 ; 1 ; 1 M  → 9 bytes
    if bytes.len() < 9 {
        return None;
    }
    if bytes[0] != 0x1b || bytes[1] != b'[' || bytes[2] != b'<' {
        return None;
    }

    // The final byte distinguishes press (M) from release (m).
    let final_byte = *bytes.last()?;
    if final_byte != b'M' && final_byte != b'm' {
        return None;
    }

    // Parse the three semicolon-separated numbers between '<' and the
    // final byte:  Cb;Cx;Cy
    let inner = std::str::from_utf8(&bytes[3..bytes.len() - 1]).ok()?;
    let mut parts = inner.split(';');
    let cb: u32 = parts.next()?.parse().ok()?;
    let cx: u32 = parts.next()?.parse().ok()?;
    let cy: u32 = parts.next()?.parse().ok()?;
    if parts.next().is_some() {
        return None; // trailing garbage
    }

    let shift = cb & 4 != 0;
    let alt = cb & 8 != 0;
    let ctrl = cb & 16 != 0;
    let motion = cb & 32 != 0;
    let wheel = cb & 64 != 0;
    let button_bits = cb & 3;

    let (kind, button) = if wheel {
        // 64 → scroll up, 65 → scroll down
        if button_bits == 0 {
            ("scroll-up".to_string(), 0)
        } else {
            ("scroll-down".to_string(), 0)
        }
    } else if final_byte == b'm' {
        ("release".to_string(), button_bits)
    } else if motion {
        ("move".to_string(), button_bits)
    } else {
        ("press".to_string(), button_bits)
    };

    Some(MouseEvent {
        kind,
        // SGR coordinates are 1-based; convert to 0-based.
        x: cx.saturating_sub(1),
        y: cy.saturating_sub(1),
        button,
        shift,
        alt,
        ctrl,
    })
}

/// ANSI sequence to enable button-event mouse tracking in SGR mode.
///
/// `1000` = report button press events; `1006` = SGR extended encoding
/// (gives pixel-accurate coordinates and distinguishes press from release).
#[napi]
pub fn mouse_enable_seq() -> String {
    "\x1b[?1000h\x1b[?1006h".to_string()
}

/// ANSI sequence to disable mouse tracking (reverse of `mouse_enable_seq`).
#[napi]
pub fn mouse_disable_seq() -> String {
    "\x1b[?1006l\x1b[?1000l".to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn buf(s: &str) -> Buffer {
        Buffer::from(s.as_bytes().to_vec())
    }

    #[test]
    fn parses_left_press() {
        let ev = parse_mouse_event(buf("\x1b[<0;10;5M")).unwrap();
        assert_eq!(ev.kind, "press");
        assert_eq!(ev.x, 9);
        assert_eq!(ev.y, 4);
        assert_eq!(ev.button, 0);
        assert!(!ev.shift && !ev.alt && !ev.ctrl);
    }

    #[test]
    fn parses_left_release() {
        let ev = parse_mouse_event(buf("\x1b[<0;10;5m")).unwrap();
        assert_eq!(ev.kind, "release");
    }

    #[test]
    fn parses_right_press_with_ctrl() {
        // button 2 + ctrl (16) = 18
        let ev = parse_mouse_event(buf("\x1b[<18;1;1M")).unwrap();
        assert_eq!(ev.kind, "press");
        assert_eq!(ev.button, 2);
        assert!(ev.ctrl);
        assert!(!ev.shift && !ev.alt);
    }

    #[test]
    fn parses_drag_move() {
        // button 0 + motion (32) = 32
        let ev = parse_mouse_event(buf("\x1b[<32;3;7M")).unwrap();
        assert_eq!(ev.kind, "move");
        assert_eq!(ev.button, 0);
    }

    #[test]
    fn parses_scroll_up() {
        // 64 = wheel + button 0
        let ev = parse_mouse_event(buf("\x1b[<64;1;1M")).unwrap();
        assert_eq!(ev.kind, "scroll-up");
    }

    #[test]
    fn parses_scroll_down() {
        // 65 = wheel + button 1
        let ev = parse_mouse_event(buf("\x1b[<65;1;1M")).unwrap();
        assert_eq!(ev.kind, "scroll-down");
    }

    #[test]
    fn parses_shift_alt_modifiers() {
        // button 0 + shift (4) + alt (8) = 12
        let ev = parse_mouse_event(buf("\x1b[<12;2;3M")).unwrap();
        assert!(ev.shift && ev.alt && !ev.ctrl);
    }

    #[test]
    fn rejects_keyboard_escape() {
        assert!(parse_mouse_event(buf("\x1b[A")).is_none());
        assert!(parse_mouse_event(buf("q")).is_none());
        assert!(parse_mouse_event(buf("")).is_none());
    }

    #[test]
    fn rejects_malformed_sgr() {
        assert!(parse_mouse_event(buf("\x1b[<0;1M")).is_none()); // missing field
        assert!(parse_mouse_event(buf("\x1b[<a;b;cM")).is_none()); // non-numeric
        assert!(parse_mouse_event(buf("\x1b[<0;1;2X")).is_none()); // bad final byte
    }

    #[test]
    fn coordinates_saturate_at_zero() {
        // SGR sends 1-based; a buggy terminal sending 0 should not underflow.
        let ev = parse_mouse_event(buf("\x1b[<0;0;0M")).unwrap();
        assert_eq!(ev.x, 0);
        assert_eq!(ev.y, 0);
    }
}
