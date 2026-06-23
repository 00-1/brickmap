//! A numeric **keypad** — a reusable, data-free UI widget. Pure geometry plus hit-testing over a
//! pixel box; **no game/topic literals**, so any game (or menu) can lay it out and route taps. The
//! renderer chooses each key's glyph; it may fall back from `✓`/`⌫` to ASCII if the face lacks them.

/// A logical key. `Back` deletes a char; `Enter` submits.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Key {
    Digit(u8),
    Dot,
    Back,
    Enter,
}

/// A laid-out key: its logical value + pixel rect (top-left origin).
#[derive(Clone, Copy)]
pub struct Cell {
    pub key: Key,
    pub x: f32,
    pub y: f32,
    pub w: f32,
    pub h: f32,
}

/// A 3-column keypad: rows `1 2 3 / 4 5 6 / 7 8 9 / . 0 ⌫`, then a full-width `Enter` bar.
pub struct Keypad {
    pub cells: Vec<Cell>,
}

impl Keypad {
    /// Lay the keypad inside the box (`x`,`y`,`w`,`h`) with `gap` px between keys.
    pub fn layout(x: f32, y: f32, w: f32, h: f32, gap: f32) -> Keypad {
        // 5 rows tall (4 digit rows + 1 enter row); 3 columns.
        let cw = (w - gap * 2.0) / 3.0;
        let ch = (h - gap * 4.0) / 5.0;
        let col_x = |c: usize| x + c as f32 * (cw + gap);
        let row_y = |r: usize| y + r as f32 * (ch + gap);
        let mut cells = Vec::with_capacity(13);
        // digit rows 1..9
        for (i, d) in (1u8..=9).enumerate() {
            let (r, c) = (i / 3, i % 3);
            cells.push(Cell {
                key: Key::Digit(d),
                x: col_x(c),
                y: row_y(r),
                w: cw,
                h: ch,
            });
        }
        // bottom digit row: . 0 ⌫
        cells.push(Cell {
            key: Key::Dot,
            x: col_x(0),
            y: row_y(3),
            w: cw,
            h: ch,
        });
        cells.push(Cell {
            key: Key::Digit(0),
            x: col_x(1),
            y: row_y(3),
            w: cw,
            h: ch,
        });
        cells.push(Cell {
            key: Key::Back,
            x: col_x(2),
            y: row_y(3),
            w: cw,
            h: ch,
        });
        // full-width Enter bar
        cells.push(Cell {
            key: Key::Enter,
            x,
            y: row_y(4),
            w,
            h: ch,
        });
        Keypad { cells }
    }

    /// The key whose rect contains (`px`,`py`), if any.
    pub fn hit(&self, px: f32, py: f32) -> Option<Key> {
        self.cells
            .iter()
            .find(|c| px >= c.x && px < c.x + c.w && py >= c.y && py < c.y + c.h)
            .map(|c| c.key)
    }

    /// Map a typed character ('0'..'9', '.', backspace, enter/'=') to a [`Key`].
    pub fn key_for_char(c: char) -> Option<Key> {
        match c {
            '0'..='9' => Some(Key::Digit(c as u8 - b'0')),
            '.' => Some(Key::Dot),
            '\u{8}' | '\u{7f}' => Some(Key::Back),
            '\n' | '\r' | '=' => Some(Key::Enter),
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lays_out_thirteen_keys_in_bounds() {
        let kp = Keypad::layout(0.0, 0.0, 300.0, 500.0, 8.0);
        assert_eq!(kp.cells.len(), 13);
        assert!(kp
            .cells
            .iter()
            .all(|c| c.x >= 0.0 && c.y >= 0.0 && c.x + c.w <= 300.001 && c.y + c.h <= 500.001));
        // the four logical specials + ten digits are all present
        assert!(kp.cells.iter().any(|c| c.key == Key::Enter));
        assert!(kp.cells.iter().any(|c| c.key == Key::Back));
        assert!(kp.cells.iter().any(|c| c.key == Key::Dot));
        for d in 0u8..=9 {
            assert!(
                kp.cells.iter().any(|c| c.key == Key::Digit(d)),
                "digit {d} missing"
            );
        }
    }

    #[test]
    fn hit_test_finds_the_key_under_a_point() {
        let kp = Keypad::layout(0.0, 0.0, 300.0, 500.0, 8.0);
        // centre of the '1' cell (row 0, col 0)
        let one = kp.cells[0];
        assert_eq!(
            kp.hit(one.x + one.w / 2.0, one.y + one.h / 2.0),
            Some(Key::Digit(1))
        );
        assert_eq!(kp.hit(-5.0, -5.0), None);
    }

    #[test]
    fn maps_typed_chars() {
        assert_eq!(Keypad::key_for_char('7'), Some(Key::Digit(7)));
        assert_eq!(Keypad::key_for_char('.'), Some(Key::Dot));
        assert_eq!(Keypad::key_for_char('\u{8}'), Some(Key::Back));
        assert_eq!(Keypad::key_for_char('\n'), Some(Key::Enter));
        assert_eq!(Keypad::key_for_char('x'), None);
    }
}
