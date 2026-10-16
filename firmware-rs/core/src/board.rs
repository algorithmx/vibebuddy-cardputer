//! Per-board constants: the panel size, its offsets inside the controller's memory, and which key
//! drives K0, K1 and K2.
//!
//! Exactly one board is selected with a cargo feature of this crate:
//! - `board-box` (default): ALIENTEK ATK-DNESP32S3-BOX V1.1, the board the firmware was written for.
//! - `board-cardputer-adv`: M5Stack Cardputer-Adv. Its values are provisional until checked on hardware:
//!   the panel offsets and the key bindings are chosen from the vendor sources in
//!   `docs/references/cardputer-adv/`, not yet tested on a device.
//!
//! The device layer selects the same board with the matching feature of `firmware-rs/device`.

use crate::canvas::View;
use crate::keymap::KeyMap;
#[cfg(feature = "board-cardputer-adv")]
use crate::keymap::KeyCode;

#[cfg(all(feature = "board-box", feature = "board-cardputer-adv"))]
compile_error!("select one board feature: board-box or board-cardputer-adv, not both");
#[cfg(not(any(feature = "board-box", feature = "board-cardputer-adv")))]
compile_error!("select one board feature: board-box (default) or board-cardputer-adv");

/// Visible width and height of the panel, in pixels, landscape.
#[cfg(feature = "board-box")]
pub const PANEL_WIDTH: i32 = 320;
#[cfg(feature = "board-box")]
pub const PANEL_HEIGHT: i32 = 240;
/// Where the visible area starts inside the controller's memory (the controller holds more rows and
/// columns than the panel shows). Zero when the panel is the whole memory.
#[cfg(feature = "board-box")]
pub const PANEL_OFFSET_X: u16 = 0;
#[cfg(feature = "board-box")]
pub const PANEL_OFFSET_Y: u16 = 0;

#[cfg(feature = "board-cardputer-adv")]
pub const PANEL_WIDTH: i32 = 240;
#[cfg(feature = "board-cardputer-adv")]
pub const PANEL_HEIGHT: i32 = 135;
/// Landscape offsets for the 240x135 panel, found on the Cardputer-Adv: with 52/40 (the portrait values) the top
/// rows and the right column were hidden by the bezel; 40/53 shows the whole frame (calibration screen, 2026-10-16).
#[cfg(feature = "board-cardputer-adv")]
pub const PANEL_OFFSET_X: u16 = 40;
#[cfg(feature = "board-cardputer-adv")]
pub const PANEL_OFFSET_Y: u16 = 53;

/// Key codes the box's device layer reports: K0 (BOOT, GPIO0) is code 0, K1 is 1, K2 is 2.
#[cfg(feature = "board-box")]
pub const KEY_MAP: KeyMap = KeyMap { k0: 0, k1: 1, k2: 2 };

/// Cardputer-Adv key codes. A matrix key uses its ASCII code; the BOOT key on G0 uses `KEY_BOOT`.
/// Provisional: K1 and K2 are the `;` and `.` keys until the on-device acceptance test settles the layout.
#[cfg(feature = "board-cardputer-adv")]
pub const KEY_BOOT: KeyCode = 0x100;
#[cfg(feature = "board-cardputer-adv")]
pub const KEY_MAP: KeyMap = KeyMap { k0: KEY_BOOT, k1: b';' as KeyCode, k2: b'.' as KeyCode };

/// Whether the screens use the compact layout for a small panel (about 240x135), not the 320x240 one.
#[cfg(feature = "board-box")]
pub const COMPACT: bool = false;
#[cfg(feature = "board-cardputer-adv")]
pub const COMPACT: bool = true;

/// Where the pet is drawn. The design is 320x240; the pet is about 113 px tall there, so the compact panel
/// draws it at 3/5 size, centred at x = 120, with its top at about y = 25.
#[cfg(feature = "board-box")]
pub const PET_VIEW: View = View::IDENTITY;
#[cfg(feature = "board-cardputer-adv")]
pub const PET_VIEW: View = View { x: 24, y: 4, num: 3, den: 5 };
/// With task cards on the left half, the pet moves to the right half: centred at x = 180.
#[cfg(feature = "board-box")]
pub const PET_VIEW_WITH_TASKS: View = View::IDENTITY;
#[cfg(feature = "board-cardputer-adv")]
pub const PET_VIEW_WITH_TASKS: View = View { x: 84, y: 4, num: 3, den: 5 };

/// Where the pomodoro ring is drawn: centre at about (61, 78), outer radius about 51, so its top stays below the title bar.
#[cfg(feature = "board-box")]
pub const RING_VIEW: View = View::IDENTITY;
#[cfg(feature = "board-cardputer-adv")]
pub const RING_VIEW: View = View { x: -16, y: -1, num: 13, den: 20 };
