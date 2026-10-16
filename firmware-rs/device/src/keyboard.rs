//! The Cardputer-Adv keyboard: a TCA8418 key-matrix controller on I2C (address 0x34; INT on G11). The matrix is 7 rows
//! by 8 columns (the factory firmware's `matrix(7, 8)`). The controller keeps a queue of key events; each event says
//! which key changed and whether it went down.
//!
//! Register numbers are from the TCA8418 datasheet; the key numbering and the logical layout are in core's
//! `keymap.rs`, from the vendor firmware's keyboard.cpp. Not yet checked on a board.

use esp_hal::Blocking;
use esp_hal::i2c::master::{Error as I2cError, I2c};
use vibebuddy_firmware_core::keymap::{KeyCode, key_number};

pub const ADDRESS: u8 = 0x34;

const REG_CFG: u8 = 0x01;
const REG_INT_STAT: u8 = 0x02;
const REG_KEY_LCK_EC: u8 = 0x03;
const REG_KEY_EVENT_A: u8 = 0x04;
const REG_KP_GPIO1: u8 = 0x1D;
const REG_KP_GPIO2: u8 = 0x1E;

/// CFG: key events raise INT. The keypad is enabled through KP_GPIO; nothing else changes.
const CFG_KE_IEN: u8 = 0x01;
/// INT_STAT: a key event is pending. Writing a 1 clears it.
const INT_K: u8 = 0x01;
/// KEY_LCK_EC: how many events are waiting (low nibble).
const EVENT_COUNT_MASK: u8 = 0x0F;
/// KEY_EVENT: bit 7 set on press, clear on release; bits 6..0 are the key number.
const EVENT_PRESS: u8 = 0x80;
const EVENT_KEY_MASK: u8 = 0x7F;

/// Rows 0..7 and columns 0..7 are scanned: 7 rows (the matrix is 7 x 8).
const ROW_MASK: u8 = 0x7F;
const COLUMN_MASK: u8 = 0xFF;

/// The keys K1 and K2 are on, as (logical key code, TCA8418 key number). `;` is row 2, column 11 of the keyboard and
/// `.` is row 3, column 11 (keyboard.cpp's _key_value_map). These match core's board::KEY_MAP.
const KNOWN_KEYS: &[(KeyCode, u8)] = &[(b';' as KeyCode, key_number(2, 11)), (b'.' as KeyCode, key_number(3, 11))];

/// Pressed keys by TCA8418 key number (1..=80). Bit n is key n.
pub struct Keyboard {
    pressed: u128,
}

impl Keyboard {
    pub fn new() -> Self {
        Self { pressed: 0 }
    }

    /// Configures the controller: the 7 x 8 matrix, key-event interrupts on, and no events left over.
    pub fn init(&mut self, i2c: &mut I2c<'static, Blocking>) -> Result<(), I2cError> {
        i2c.write(ADDRESS, &[REG_KP_GPIO1, ROW_MASK])?;
        i2c.write(ADDRESS, &[REG_KP_GPIO2, COLUMN_MASK])?;
        i2c.write(ADDRESS, &[REG_CFG, CFG_KE_IEN])?;
        i2c.write(ADDRESS, &[REG_INT_STAT, INT_K])?;
        self.pressed = 0;
        Ok(())
    }

    /// Reads every waiting event and updates the pressed set. Cheap when nothing is waiting: one register read.
    pub fn poll(&mut self, i2c: &mut I2c<'static, Blocking>) -> Result<(), I2cError> {
        let mut count = [0u8];
        i2c.write_read(ADDRESS, &[REG_KEY_LCK_EC], &mut count)?;
        for _ in 0..(count[0] & EVENT_COUNT_MASK) {
            let mut event = [0u8];
            i2c.write_read(ADDRESS, &[REG_KEY_EVENT_A], &mut event)?;
            let number = event[0] & EVENT_KEY_MASK;
            if number == 0 {
                continue;
            }
            let bit = 1u128 << number;
            if event[0] & EVENT_PRESS != 0 {
                self.pressed |= bit;
            } else {
                self.pressed &= !bit;
            }
        }
        i2c.write(ADDRESS, &[REG_INT_STAT, INT_K])?;
        Ok(())
    }

    /// Whether the key with this logical code is held. Only the keys in `KNOWN_KEYS` are read.
    pub fn is_pressed(&self, code: KeyCode) -> bool {
        KNOWN_KEYS.iter().any(|&(known, number)| known == code && self.pressed & (1u128 << number) != 0)
    }
}
