//! ST7789 init and frame-flush commands.
//!
//! The C firmware drives it through esp_lcd: `esp_lcd_new_panel_st7789` → reset → init →
//! invert colors → set MADCTL/COLMOD again → swap_xy → mirror → display on. This lists the commands
//! that sequence actually sends, in order, and the device layer sends them from the table.

use crate::board::{PANEL_HEIGHT, PANEL_OFFSET_X, PANEL_OFFSET_Y, PANEL_WIDTH};

/// One command: command byte, parameters, and milliseconds to wait after sending it.
pub struct Command {
    pub command: u8,
    pub parameters: &'static [u8],
    pub delay_ms: u32,
}

const fn command(command: u8, parameters: &'static [u8], delay_ms: u32) -> Command {
    Command { command, parameters, delay_ms }
}

pub const INIT_SEQUENCE: &[Command] = &[
    // No reset pin wired, so software reset; the datasheet asks for at least 5 ms, esp_lcd waits 20 ms.
    command(0x01, &[], 20),
    // The panel powers up asleep; wake it first.
    command(0x11, &[], 100),
    // esp_lcd's init: MADCTL (RGB order), COLMOD 16 bit, RAMCTRL big-endian.
    command(0x36, &[0x00], 0),
    command(0x3A, &[0x55], 0),
    command(0xB0, &[0x00, 0xF0], 0),
    // Invert: this panel only shows correct colors when inverted.
    command(0x21, &[], 0),
    // The two extra commands the C firmware sends on its own.
    command(0x36, &[0x00], 0),
    command(0x3A, &[0x65], 0),
    // Landscape: swap_xy sets MV, then mirror x sets MX.
    command(0x36, &[0x20], 0),
    command(0x36, &[0x60], 0),
    // Display on.
    command(0x29, &[], 0),
];

/// Column and row range for a full-frame flush: the panel's visible area, shifted by the board's offsets.
const COLUMNS: [u8; 4] = span(PANEL_OFFSET_X, PANEL_WIDTH);
const ROWS: [u8; 4] = span(PANEL_OFFSET_Y, PANEL_HEIGHT);

/// Big-endian start and end of `length` pixels that begin at `offset`.
const fn span(offset: u16, length: i32) -> [u8; 4] {
    let start = offset;
    let end = offset + length as u16 - 1;
    [(start >> 8) as u8, start as u8, (end >> 8) as u8, end as u8]
}

/// The window set before a full-frame flush. Then RAMWR (0x2C) plus the pixels.
pub const WINDOW: [Command; 2] = [command(0x2A, &COLUMNS, 0), command(0x2B, &ROWS, 0)];

pub const RAMWR: u8 = 0x2C;
