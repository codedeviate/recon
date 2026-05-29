//! Terminal key-event → byte-sequence translation.
//!
//! Shared by the interactive `ssh://` and `telnet://` clients. It is a
//! pure crossterm→bytes mapping with no transport dependency, so it lives
//! in its own module rather than inside `ssh.rs` — that keeps it available
//! when the `ssh` feature (and thus `ssh.rs`) is compiled out, which the
//! `telnet` client still needs. See GitHub issue #1.

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

/// Convert a crossterm KeyEvent to the byte sequence a terminal sends.
pub(crate) fn key_event_to_bytes(key: &KeyEvent) -> Vec<u8> {
    match key.code {
        KeyCode::Char(c) => {
            if key.modifiers.contains(KeyModifiers::CONTROL) {
                let byte = (c.to_ascii_uppercase() as u8).wrapping_sub(b'@');
                vec![byte]
            } else {
                let mut buf = [0u8; 4];
                let s = c.encode_utf8(&mut buf);
                s.as_bytes().to_vec()
            }
        }
        KeyCode::Enter     => vec![b'\r'],
        KeyCode::Backspace => vec![b'\x7f'],
        KeyCode::Tab       => vec![b'\t'],
        KeyCode::Esc       => vec![b'\x1b'],
        KeyCode::Up        => vec![b'\x1b', b'[', b'A'],
        KeyCode::Down      => vec![b'\x1b', b'[', b'B'],
        KeyCode::Right     => vec![b'\x1b', b'[', b'C'],
        KeyCode::Left      => vec![b'\x1b', b'[', b'D'],
        KeyCode::Home      => vec![b'\x1b', b'[', b'H'],
        KeyCode::End       => vec![b'\x1b', b'[', b'F'],
        KeyCode::PageUp    => vec![b'\x1b', b'[', b'5', b'~'],
        KeyCode::PageDown  => vec![b'\x1b', b'[', b'6', b'~'],
        KeyCode::Delete    => vec![b'\x1b', b'[', b'3', b'~'],
        KeyCode::Insert    => vec![b'\x1b', b'[', b'2', b'~'],
        KeyCode::F(n) => match n {
            1  => vec![b'\x1b', b'O', b'P'],
            2  => vec![b'\x1b', b'O', b'Q'],
            3  => vec![b'\x1b', b'O', b'R'],
            4  => vec![b'\x1b', b'O', b'S'],
            5  => vec![b'\x1b', b'[', b'1', b'5', b'~'],
            6  => vec![b'\x1b', b'[', b'1', b'7', b'~'],
            7  => vec![b'\x1b', b'[', b'1', b'8', b'~'],
            8  => vec![b'\x1b', b'[', b'1', b'9', b'~'],
            9  => vec![b'\x1b', b'[', b'2', b'0', b'~'],
            10 => vec![b'\x1b', b'[', b'2', b'1', b'~'],
            11 => vec![b'\x1b', b'[', b'2', b'3', b'~'],
            12 => vec![b'\x1b', b'[', b'2', b'4', b'~'],
            _  => vec![],
        },
        _ => vec![],
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn key_ctrl_c_is_etx() {
        let key = KeyEvent::new(KeyCode::Char('c'), KeyModifiers::CONTROL);
        assert_eq!(key_event_to_bytes(&key), vec![0x03]);
    }

    #[test]
    fn key_enter_is_cr() {
        let key = KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE);
        assert_eq!(key_event_to_bytes(&key), vec![b'\r']);
    }

    #[test]
    fn key_up_arrow_is_escape_sequence() {
        let key = KeyEvent::new(KeyCode::Up, KeyModifiers::NONE);
        assert_eq!(key_event_to_bytes(&key), vec![0x1b, b'[', b'A']);
    }

    #[test]
    fn key_regular_char() {
        let key = KeyEvent::new(KeyCode::Char('a'), KeyModifiers::NONE);
        assert_eq!(key_event_to_bytes(&key), vec![b'a']);
    }

    #[test]
    fn key_ctrl_d_is_eot() {
        let key = KeyEvent::new(KeyCode::Char('d'), KeyModifiers::CONTROL);
        assert_eq!(key_event_to_bytes(&key), vec![0x04]);
    }

    #[test]
    fn key_f1_sequence() {
        let key = KeyEvent::new(KeyCode::F(1), KeyModifiers::NONE);
        assert_eq!(key_event_to_bytes(&key), vec![0x1b, b'O', b'P']);
    }
}
