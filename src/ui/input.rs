//! Keyboard shortcuts.

use rune::Tx;

use super::{Cmd, Ui, zoom};

/// Keys nothing else claimed: F5 checks now; Ctrl + / Ctrl - / Ctrl 0 zoom.
pub fn on_key(s: &mut Ui, key: &rune::KeyEvent, _tx: &mut Tx) -> bool {
    if key.key == rune::Key::Function(5) {
        s.check_pressed_at = s.now;
        s.send(Cmd::CheckNow);
        return true;
    }
    let level = s.cfg.zoom_percent;
    let next = if key.is_command('+') || key.is_command('=') {
        zoom::step(level, true)
    } else if key.is_command('-') {
        zoom::step(level, false)
    } else if key.is_command('0') {
        100
    } else {
        return false;
    };
    if next != level {
        s.change(|c| c.zoom_percent = next);
    }
    true
}
