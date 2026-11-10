//! Keyboard shortcuts.

use rune::Tx;

use super::{Cmd, Ui};

/// Keys nothing else claimed: F5 checks now.
pub fn on_key(s: &mut Ui, key: &rune::KeyEvent, _tx: &mut Tx) -> bool {
    if key.key == rune::Key::Function(5) {
        s.check_pressed_at = s.now;
        s.send(Cmd::CheckNow);
        return true;
    }
    false
}
