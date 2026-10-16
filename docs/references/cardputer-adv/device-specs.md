# Cardputer-Adv: device specs as measured on the unit

What was read from the connected unit (USB `303a:1001`, `/dev/ttyACM0`) on 2026-10-16 and 2026-10-17, and what the
firmware assumes. Vendor-published values are in `extracts.md`; this file is what the board itself reports, and where
the two differ. Read-only probes unless stated.

## Identity

| Item | Value | How read |
|---|---|---|
| Module | Stamp-S3A (ESP32-S3FN8) | label on the board |
| Chip | ESP32-S3 (QFN56), revision v0.2 | esptool `chip_id` |
| Crystal | 40 MHz | esptool |
| MAC | `e8:f6:0a:xx:xx:xx` (redacted) | esptool `read_mac` |
| Features | Wi-Fi, BLE | esptool (no PSRAM listed) |
| USB | USB-Serial/JTAG (native, no UART bridge) | esptool |
| Secure boot | disabled | esptool `get_security_info` |
| Flash encryption | disabled | esptool `get_security_info` |
| eFuse key blocks | all empty (USER/EMPTY) | esptool `get_security_info` |

## Memory

| Item | Value | Note |
|---|---|---|
| Flash | 8 MB, XMC, manufacturer 0x20, device 0x4017 | esptool `flash_id` |
| Flash mode | quad (4 data lines), set in eFuse | esptool |
| Flash voltage | 3.3 V, set by eFuse | esptool |
| PSRAM | none (not in the features list) | esptool |
| Internal SRAM | 512 KB on the chip; usable for the app about 320 KB after ROM and cache reservations (estimate) | estimate, see below |

The internal RAM figure is an estimate. The firmware's `MEMORY INTERNAL` line reports the real free figure at runtime
(`report_memory` in `firmware-rs/device/src/main.rs`); it has not been captured yet.

### Static RAM of the Adv firmware build (from the ELF)

| Region | Bytes |
|---|---|
| `.data` + `.data.wifi` | ~19,000 |
| `.bss` | ~153,000 |
| `.dram2_uninit` (first heap region, `reclaimed`) | 73,744 |
| extra heap region for the Adv frame | 73,728 |
| heap region (JSON, look buffers) | 16,384 |

Total of statics and heap regions: about 336 KB, before stacks and the Wi-Fi driver's runtime buffers. That is more
than the estimated ~320 KB usable, so the Adv build may not fit with Wi-Fi on. Decide with the `MEMORY INTERNAL` figure.

## Flash layout

On the unit now (written by `tools/flash.sh BOARD=cardputer-adv`, verified against the build):

| Offset | Size | Name | Content |
|---|---|---|---|
| 0x000000 | 21,136 B | bootloader | v6.0.1 2nd-stage, 8 MB header (`bootloader-cardputer-adv.bin`) |
| 0x008000 | 3 KB | partition table | `firmware/partitions-cardputer-adv.csv` |
| 0x009000 | 24 KB | nvs | settings |
| 0x00F000 | 4 KB | phy_init | |
| 0x010000 | 3 MB | ota_0 | app, 2,804,112 B on the unit |
| 0x310000 | 3 MB | ota_1 | empty |
| 0x610000 | 1.75 MB | voices | voice pack (1.2 MB) |
| 0x7D0000 | 8 KB | otadata | written by the firmware after the first boot |
| 0x7D2000 | 8 KB | link | the box's own key and paired computers |

Before this flash, the unit ran M5Stack's factory firmware: app `cardputer-adv`, ESP-IDF v5.4.2, version
`V0.9-36-ge824a76`, built Oct 21 2025, in a 4 MB factory slot at 0x10000. The full 8 MB original image is in
`~/cardputer-adv-backup/factory-flash-8MB-2026-10-16.bin` (SHA-256 alongside).

## Peripherals (pins from the vendor sources; the unit verifies what can be verified without the screen)

| Function | Pins | Source / status |
|---|---|---|
| LCD ST7789V2, 240x135, SPI | backlight G38, RST G33, DC G34, MOSI G35, SCK G36, CS G37 | vendor extracts; the panel offsets 40/53 are confirmed by photo |
| Keyboard TCA8418 | I2C G8 (SDA), G9 (SCL), INT G11, address 0x34; 7 rows by 8 columns | vendor `keyboard.cpp`, `hal_config.h` |
| Key numbers (TCA8418) | `;` = 57 (K1), `.` = 58 (K2); general: `row*10 + col + 1` | vendor table, see `firmware-rs/core/src/keymap.rs` |
| BOOT key | G0 (K0) | vendor, schematic |
| Speaker codec ES8311 | I2C address 0x18 (G8/G9); I2S BCK G41, WS G43, DATA G42 | vendor `M5Unified.inl` |
| Speaker amplifier | no separate enable pin; the codec's register writes enable it | vendor `M5Unified.inl` |
| Microphone | analog MEMS into ES8311 (not used by the firmware) | schematic |
| IMU BMI270 | I2C 0x69 (G8/G9) | schematic (not used) |
| microSD | CS G12, MOSI G14, SCK G40, MISO G39 | vendor `hal_config.h` |
| IR TX | G44 | vendor `hal_config.h` |
| Battery ADC | G10, 2:1 divider | schematic |
| USB | native, D- G19, D+ G20 | module label |
| Grove | G1, G2 | schematic |
| UART0 | not used on the Adv (G43/G44 belong to the speaker and the expansion header) | firmware decision |

## Measured on the unit, first boot of the Adv build (2026-10-17)

Boot output over USB, captured after a hard reset, from the fixed build (I2C on G8/G9, I2S on G41/G43/G42, UART off):

```
I (825) boot: Loaded app from partition at offset 0x10000
I (828) boot: Set actual ota_seq=1 in otadata[0]
TALLY LOADED 0 0S DAY 0
DISPLAY READY BUILD 574b700-dirty 2026-10-17 06:42
FIRMWARE VERSION 0.5.0
LINK NOISE
BOX KEY <redacted>
RELAY URL wss://relay.korekore.ai
VOICES builtin
AUDIO READY
AUDIO CODEC ES8311
VOLUME 65
BUTTONS READY
READY vibebuddy-fw 0.5.0
MEMORY INTERNAL 38848 PSRAM 0 TOTAL 38848/163856
```

What this settles:
- The v6.0.1 bootloader with the 8 MB header loads the app from ota_0. The firmware confirms the slot (otadata
  `ota_seq=1`).
- The codec answers at 0x18 and is identified as ES8311. The speaker path is up (sound not yet heard).
- The keyboard driver starts (`BUTTONS READY`). Which keys it reads is not yet confirmed by pressing them.
- The heap regions total 163,856 bytes. Free after boot: 38,848 bytes, so about 125 KB is already in use before any
  network starts. That is high. Wi-Fi and the Relay need more internal RAM than that on top, so the network features
  are not viable on the Adv as built. This supports profile A in `goals/G3.md`.
- The Wi-Fi stack is not started at boot; the figure above excludes it.

Before the fixes to I2C, I2S and UART, the same build printed nothing after boot; the pins were wrong (see the plan log).

## Still to confirm with the unit in hand

- Sound, confirmed on the unit: the focus-end voice line ("ding dong, nice work, time for break") plays through the Adv speaker and codec (fast-clock build, 2026-10-17).
- Keys, confirmed on the unit: K1 (`;`) switches between duty and pomodoro. K0 (BOOT, G0) starts, pauses and resumes the 25-minute focus timer. K2 (`.`) opens the source on the Mac, so without a Mac it shows nothing; not tested.
- The screen shows the idle pet with `READY` after boot.
- Free internal RAM once the Wi-Fi driver is started (not started at boot; a network test would need credentials).
- Battery voltage, charge state, microSD and IR paths.
