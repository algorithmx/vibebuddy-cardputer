//! Which key drives K0, K1 and K2 on the current board.
//!
//! The device layer reports every key that is held as a [`KeyCode`]: the box reports its three keys
//! as codes 0 to 2, the Cardputer-Adv reports matrix keys by ASCII code and the BOOT key as
//! `board::KEY_BOOT`. [`KeyMap`] names the one code that drives each of K0, K1 and K2, so the button
//! logic in `buttons.rs` does not care which physical key is which.

use crate::buttons::Levels;

/// One key on the board, as the device layer reports it.
pub type KeyCode = u16;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct KeyMap {
    pub k0: KeyCode,
    pub k1: KeyCode,
    pub k2: KeyCode,
}

impl KeyMap {
    /// The pressed state of K0, K1 and K2 in one sample. `is_pressed` says whether a key code is held.
    pub fn levels(&self, is_pressed: impl Fn(KeyCode) -> bool) -> Levels {
        Levels { k0: is_pressed(self.k0), k1: is_pressed(self.k1), k2: is_pressed(self.k2) }
    }
}

/// The Cardputer's keyboard as the TCA8418 numbers it. The controller numbers a key `row * 10 + column + 1` (1..=80),
/// with the switch matrix wired as 7 rows by 8 columns. The keyboard's keys sit in 4 logical rows by 14 logical
/// columns, and the vendor firmware's `remap()` (keyboard.cpp in M5Cardputer-UserDemo, branch CardputerADV) turns one
/// into the other: column = raw row * 2 (+1 when the raw column is 4..7), row = (raw column + 4) % 4.
///
/// Not checked on a board yet: the vendor table is read, and the on-device probe confirms it.
pub const fn key_number(logical_row: u8, logical_col: u8) -> u8 {
    let raw_row = logical_col / 2;
    let raw_col = logical_row + if logical_col % 2 == 1 { 4 } else { 0 };
    raw_row * 10 + raw_col + 1
}

/// The logical (row, column) of a TCA8418 key number, the inverse of [`key_number`].
pub const fn logical_position(number: u8) -> (u8, u8) {
    let raw_row = (number - 1) / 10;
    let raw_col = (number - 1) % 10;
    let logical_col = raw_row * 2 + if raw_col > 3 { 1 } else { 0 };
    let logical_row = (raw_col + 4) % 4;
    (logical_row, logical_col)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn key_numbers_round_trip_over_the_whole_keyboard() {
        let mut seen = [false; 81];
        for row in 0..4 {
            for col in 0..14 {
                let number = key_number(row, col);
                assert!((1..=80).contains(&number), "({row}, {col}) -> {number}");
                assert!(!seen[number as usize], "({row}, {col}) repeats {number}");
                seen[number as usize] = true;
                assert_eq!(logical_position(number), (row, col));
            }
        }
    }

    #[test]
    fn semicolon_and_period_sit_where_the_vendor_table_puts_them() {
        // Row 2, column 11 is ';' and row 3, column 11 is '.' in keyboard.cpp's _key_value_map.
        assert_eq!(key_number(2, 11), 57);
        assert_eq!(key_number(3, 11), 58);
    }

    const MAP: KeyMap = KeyMap { k0: 0x100, k1: b';' as KeyCode, k2: b'.' as KeyCode };

    #[test]
    fn each_logical_key_follows_its_own_code() {
        assert_eq!(MAP.levels(|code| code == 0x100), Levels { k0: true, k1: false, k2: false });
        assert_eq!(MAP.levels(|code| code == b';' as KeyCode), Levels { k0: false, k1: true, k2: false });
        assert_eq!(MAP.levels(|code| code == b'.' as KeyCode), Levels { k0: false, k1: false, k2: true });
    }

    #[test]
    fn unmapped_keys_press_nothing() {
        assert_eq!(MAP.levels(|code| code == b'a' as KeyCode), Levels::default());
    }
}
