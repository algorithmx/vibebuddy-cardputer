//! What the device layer does differently per board. The core's board constants (panel size, key map) are in
//! `firmware-core/src/board.rs`; this file is the hardware under them.
//!
//! - `board-box` (default): ALIENTEK ATK-DNESP32S3-BOX V1.1. 16 MB flash, 8 MB octal PSRAM, an 8-bit parallel
//!   ST7789, a XL9555 expander for the backlight and two keys, and an ES8311 codec.
//! - `board-cardputer-adv`: M5Stack Cardputer-Adv. 8 MB flash, no PSRAM, an SPI ST7789V2, a TCA8418 keyboard
//!   controller for the keys, the backlight on G38, and an ES8311 codec. Pins from `docs/references/cardputer-adv`.
//!   Provisional until checked on hardware (see the plan's final phase).

#[cfg(all(feature = "board-box", feature = "board-cardputer-adv"))]
compile_error!("select one board feature: board-box or board-cardputer-adv, not both");
#[cfg(not(any(feature = "board-box", feature = "board-cardputer-adv")))]
compile_error!("select one board feature: board-box (default) or board-cardputer-adv");

/// Whether the board has the 8 MB PSRAM. Without it, the big buffers live in internal RAM.
#[cfg(feature = "board-box")]
pub const HAS_PSRAM: bool = true;
#[cfg(feature = "board-cardputer-adv")]
pub const HAS_PSRAM: bool = false;

/// Whether the backlight is driven by the XL9555 expander (the box) or a GPIO (the Cardputer-Adv).
#[cfg(feature = "board-box")]
pub const BACKLIGHT_ON_EXPANDER: bool = true;
#[cfg(feature = "board-cardputer-adv")]
pub const BACKLIGHT_ON_EXPANDER: bool = false;
