# G2: Adapt VibeBuddy to M5Stack Cardputer-Adv

Status: plan written 2026-10-16. Execution status is recorded at the end of this file.

Inputs:
- `.agent_context/goals/G1.md`: architecture map and doc-vs-code inconsistencies.
- `.agent_context/research/codebase-hardware-binding.md`: every hardware binding in the device firmware.
- `.agent_context/research/cardputer-adv-research.md`: Cardputer-Adv spec table and pinmap.
- `docs/references/cardputer-adv/`: saved vendor sources and extracts.

## 1. Findings that set the scope

1. **The repo does not run on a Cardputer today.** The active firmware (`firmware-rs/`, esp-hal + embassy) targets the ALIENTEK ATK-DNESP32S3-BOX V1.1. Nothing in the repo refers to M5Stack hardware.
2. **So this is a port from a different board, not a pin-map swap.** The bindings that change:
   - Display: the box uses ST7789 over 8-bit i8080 parallel at 320x240. The Cardputer-Adv uses ST7789V2 over SPI at 240x135. Layout in `firmware-rs/core/src/display.rs` and `canvas.rs` is hard-coded to 320x240.
   - Keys: the box has K0 on GPIO0 plus K1/K2 on an XL9555 expander. The Cardputer-Adv has a 56-key matrix behind a TCA8418 on I2C (INT on G11). The UX assumes three keys, so it needs a mapping decision.
   - Expander coupling: the XL9555 also drives the backlight and speaker amp on the box. On the Adv, the backlight is G38 and the amp is controlled differently (AMP_EN not yet found).
   - Audio: the box uses ES8311 with an I2S0 BCLK-only design at 24 kHz. The Adv uses ES8311 with full I2S (G41 BCLK, G43 LRCK, G42 DAC data, G46 ADC data), plus an NS4150B amp and an analog mic. The Adv mic is not PDM.
   - Flash and PSRAM: the code assumes 16 MB flash and 8 MB octal PSRAM. The Adv is 8 MB flash and no PSRAM listed (FN8 part). Partition layout (two 4 MB OTA slots and a 2 MB voices partition) does not fit in 8 MB as is.
   - New on the Adv, not preserved from the original: BMI270 IMU (I2C 0x69), 3.5 mm jack, EXT and FPC headers.
3. **Preserved as-is or near-as-is (same GPIOs):** LCD control lines (G33 to G38), microSD SPI (G12/G14/G39/G40), battery ADC (G10), IR TX (G44), Grove (G1/G2), G0 button.
4. **Build environment blocker:** no `esp` Xtensa toolchain is installed here (`rustup toolchain list` shows only stable and nightly). `firmware-rs/device` cannot be built until `espup` is installed. Host tests for `firmware-rs/core` do not need it.

## 2. Decisions to make before coding

These are design choices, not facts. Each has a recommended default. Change them before Phase B if you disagree.

| # | Decision | Recommended default | Why |
|---|---|---|---|
| D1 | Target firmware tree | `firmware-rs/` (Rust). Keep `firmware/` (C) untouched and out of scope | Active tree per justfile, CI, ADR-0006. |
| D2 | Keep the box build working in parallel? | Yes: add a board selector (cargo feature `board-box` / `board-cardputer-adv`), keep the box as default until the Adv path is verified | Avoids breaking the working device. Needs a board-config layer that does not exist yet. |
| D3 | Three-key UX on a full keyboard | Map K0 to G0, K1 and K2 to two keys on the TCA8418 matrix (for example `;` and `.` or Enter and Backspace), recorded in `core/src/buttons.rs` | Keeps pomodoro, menu and mute flows working with minimal change. |
| D4 | Display geometry | Parameterise `core` layout by width and height. Adv = 240x135 landscape | 240x135 is much smaller than 320x240, so some screens need smaller fonts or fewer rows. Needs visual review. |
| D5 | Flash layout | Shrink OTA slots to fit 8 MB (for example two 3 MB OTA slots plus a 1 MB voices area), or move voices to the SD card | Confirm sizes against the actual app binary size first. |
| D6 | PSRAM-dependent features | Check which features need PSRAM (framebuffer, voice buffers). Drop or stream them if PSRAM is absent | Adv has no PSRAM listed. |
| D7 | Audio path | ES8311 over I2S, reuse the existing ES8311 I2C init (address 0x18 in code, to verify on the Adv), drop the 3.5 mm handling until later | Preserves speaker playback. Mic capture is a separate feature, out of scope for the first pass. |
| D8 | IMU | Not used in the first pass | Not needed for existing features. |

## 3. Phases

### Phase A: host-verifiable core changes (no hardware or esp toolchain needed)

A1. Parameterise display geometry in `firmware-rs/core/src/canvas.rs`, `lcd.rs`, `display.rs`, with a compile-time size. Keep the box size as default.
A2. Add a button abstraction in `core/src/buttons.rs` that maps logical buttons to a board's key source. Update `core/src/menu.rs` and `pomodoro.rs` only if their logic depends on the physical key count.
A3. Add board-specific constants in one module (for example `core/src/board.rs`) for width, height, flash size, PSRAM presence. Device code uses it.
A4. Run `cargo test` in `firmware-rs/core` and the simulator examples. All existing host tests must pass for the box configuration.
A5. Add host tests for the 240x135 layout (text fits, menus do not overflow).

Verification: `just test` (note from G1: it omits clippy, localization and Python checks that CI runs; run those too).

### Phase B: device layer (needs `espup` and, for real verification, a Cardputer-Adv)

B1. Install the Xtensa toolchain: `espup install`, then confirm `firmware-rs/device` builds for the box target (no regression).
B2. Add `board-cardputer-adv` pin map in `firmware-rs/device/src/main.rs` (header comment L4-11 is the pin map to replace). Use the pins in section 2 of the research file.
B3. Replace the LCD driver with SPI ST7789V2 at 240x135 for the Adv (new driver module, keep i8080 for the box).
B4. Replace the keyboard path: TCA8418 driver on I2C (G8/G9, addr 0x34, INT G11). Map to logical buttons per D3.
B5. Replace the audio path: ES8311 I2S (G41/G43/G42/G46). Confirm the codec I2C address and MCLK source against the schematic first (unknown in research).
B6. Keep microSD, battery ADC (G10) and IR (G44) as-is if their code is GPIO-generic. Verify.
B7. Partition and flash sizing for 8 MB (D5). Update `firmware-rs/bootloader`, `tools/build-firmware.sh`, `tools/make-partition-table.py`, and `firmware/partitions.csv` (the Rust build reads it, per G1).
B8. Build the Adv image. Flash with `tools/flash.sh` if a device is attached (`tools/detect-device.sh`).

Verification without a device: B1 and B8 builds succeed. Anything that needs a real screen, keyboard or audio is marked unverified in the execution log below.

### Phase C: app and docs parity (AGENTS.md)

C1. If the adapted device changes anything the user sees (for example the key layout or screen content), make the matching change in `app/` (Mac) and `desktop/` (Linux) in the same PR, with zh-Hans strings in `app/Localization/zh-Hans.lproj`. Run `tools/check-localization.py`.
C2. Update `docs/hardware.md` (its top section contradicts its verified sections, per G1) and `docs/architecture.md` (says USB-only, which is out of date per G1).
C3. Docs in English. If `README.md` changes, update `README.zh-CN.md` to match.

### Phase D: review

D1. Run the checks in CI (`.github/workflows`), not only `just test`.
D2. Keep the box build green throughout.

## 4. Risks

- Porting from a different board is larger than the goal implied. Phase B is most of the work.
- Keyboard mapping (D3) changes the UX. Needs a user decision before Phase B4.
- Vendor schematic was read at 110 dpi. Small net labels (FPC, Grove, AMP_EN, codec address) must be checked on a higher-resolution render or the source file before wiring.
- Battery capacity conflicts between vendor (1500 mAh) and retail (1750 mAh) pages. Not used by code today.
- Vendor chip datasheets (TCA8418, ES8311, BMI270, NS4150B) were not fetched.
- No physical Cardputer-Adv is confirmed attached. Without one, Phase B is verified by build only.

## 5. Execution log

### Phase A (host-only), 2026-10-16: partly done

Baseline: box preview frames (43 PPMs) were captured before any change and kept in the session scratchpad under `baseline/`. The box output is byte-identical after the changes (`diff -rq` clean).

Done:
- A3: `firmware-rs/core/src/board.rs` holds the panel size, panel offsets and key bindings per board. Selected by cargo features in `firmware-rs/core/Cargo.toml`: `board-box` (default) or `board-cardputer-adv`. Exactly one must be on (`compile_error!` otherwise).
- A2 / D3 (configurable key mapping): `firmware-rs/core/src/keymap.rs` maps logical K0, K1, K2 onto key codes. Values per board are in `board.rs`. Box: codes 0, 1, 2. Cardputer-Adv: K0 = BOOT key (`KEY_BOOT`), K1 = `;`, K2 = `.`. The Adv values are provisional and change in one place.
- A1 (partial): `canvas.rs` WIDTH/HEIGHT and `lcd.rs` WINDOW come from the board module. The LCD window is computed from the board's panel offsets.
- A4: `cargo test` in `firmware-rs/core`, both feature sets: 99 unit tests and 47 integration tests pass. Clippy is clean on both.
- A5 (partial): the screenshot tests read the board size instead of a hard-coded 320x240.

Not done (A1 layout, the main remaining Phase A work):
- The screen layouts in `display.rs` use fixed pixel positions for the 320x240 box (pet ring centred at (118, 122) with radius 86, right-hand panel from x=208, look centred at (160, 156)). Rendered at 240x135 (scratchpad `adv_png/sheet.png`), the right panel is cut off, the ring is cropped at the bottom, and the side text runs off the edge. A proper 240x135 layout needs positions derived from the panel size and a visual review. That is a design task, not a constant change.
- A5 layout-fit tests need that layout work first.

### Phase A, layout (option 1 chosen by the user), 2026-10-16

Approach: a `View` transform in `canvas.rs` (offset plus a fraction) maps the 320x240 design onto the panel for shapes and the pet, the ring and the skits. Text keeps its glyph size and only its position is mapped. The compact layout is selected by `board::COMPACT`; the box uses the identity view and its code path is unchanged.

Verified:
- Box frames: byte-identical to HEAD, for the standard set (43 frames) and the leisure set (784 frames). Checked against a scratch copy of HEAD (`git archive HEAD`), not against a fixed baseline.
- Tests: 99 unit + 47 integration pass for both feature sets. Clippy is clean for both.
- Adv previews rendered and inspected: idle pet with status and idle line; pet with three task cards (left) and pet (right); pomodoro (focus, paused, break, idle, alarm, no link); menus (list, network, status); skits (ball, read, stars, hide).

Known gaps on the compact panel (to fix or accept):
- The countdown digits overlap the ring's edge (cosmetic). Possible fix: smaller ring or digits.
- Task cards drop the project name (the second line shows state and elapsed time only).
- Not yet reviewed: the look screens (`look_preview`), the remaining menu and alarm frames, and any screen that has not been rendered above.
- Nothing has been run on the device. All checks are host renders.

Device layer (`firmware-rs/device`): not changed. It still hard-codes the box pins and panel. Phase B.

### On-device probes, 2026-10-16 (read-only)

Details: `.agent_context/research/device-probe-2026-10-16.md`.
- USB: 303a:1001 Espressif USB JTAG/serial, `/dev/ttyACM0`.
- Chip: ESP32-S3 QFN56 rev v0.2, 8 MB flash (XMC), no PSRAM reported.
- Flash contents: the factory app at 0x10000 is ESP-IDF v5.4.2, project `cardputer-adv`, version `V0.9-36-ge824a76`, built Oct 21 2025. The device runs M5Stack's Cardputer-Adv factory firmware, not VibeBuddy.
- Serial: idle capture silent. Across a hard reset only the ROM banner appears, so the bootloader and app print nothing on this port.

Impact on the plan:
- The connected board is the Cardputer-Adv, which confirms the target and the 8 MB / no-PSRAM assumption in D5 and D6.
- Phase B can use this board to verify pin behaviour once the firmware is ready. Flashing VibeBuddy overwrites the factory app; the factory image should be backed up first, from flash at 0x10000 up to the size the app uses, before any flash write.
- Backup taken: the full 8 MB factory flash, `~/cardputer-adv-backup/factory-flash-8MB-2026-10-16.bin` (SHA-256 in the matching `.sha256` file). Use it to restore the factory firmware. Tool note: the system esptool 4.7 fails at 0x200000 without the stub; use the ESP-IDF esptool 4.11 instead.

### Display check on the device, 2026-10-16

- Crate: `firmware-rs/adv-display-check/` (own workspace, not the box firmware). Core built with `board-cardputer-adv`. It draws the compact screens and steps through 16 scenes with the BOOT key (G0): 0 calibration, 1 pet idle, 2 three tasks, 3 input required, 4 pomodoro idle, 5 focus, 6 paused, 7 alarm, 8 menu list, 9 menu network, 10 menu status, 11 no link, 12 muted, 13 ball skit, 14 read skit, 15 sleep skit.
- Panel pins as in `docs/references/cardputer-adv/extracts.md`: backlight G38, reset G33, D/C G34, MOSI G35, SCK G36, CS G37, BOOT G0.
- Built with `. ~/export-esp.sh && cargo build --release` in that crate. The image is `/tmp/claude-1000/.../scratchpad/flash/adv-display-check.bin` (121,488 bytes).
- Flashed at 0x10000 only (`esptool write_flash`, ESP-IDF esptool 4.11), hash verified. The bootloader (0x0) and the partition table (0x8000) were not written.
- Restore the factory app: take the 0x10000 to 0x410000 slice of `~/cardputer-adv-backup/factory-flash-8MB-2026-10-16.bin` and write it back at 0x10000.
- Serial after flashing: silent (no panic). The result on the screen is not yet confirmed; it needs a visual check.

- Calibration on the device, photo check (2026-10-16): first attempt with the portrait offsets 52/40 hid the top rows and the right column. Landscape offsets 40/53 show the whole frame: all four edges, the six colour bars, the diagonals in the corners. The offsets are confirmed on the hardware and `board.rs` is updated.
- Next: the scenes 1 to 15 (pet, tasks, pomodoro, menus, skits), checked on the device one by one.

### Scene review on the device, complete (2026-10-16)

- Scenes 0 to 15 confirmed by photo, with the labels checked against the serial scene log. Confirmed: the alarm (scene 7, ring lit in the break colour and shaking), the menu list (8), and the sleep skit (15, the Z floats).
- Debug code removed from `firmware-rs/adv-display-check/src/main.rs`: the scene-0 G0 status line and the serial report. The firmware was reflashed without them.
- Fixes applied from the review, still to confirm on the device: countdown digit spacing (accepted with a known touch on the ring), task titles up to 18 characters, and the menu frame filling the full screen.
- Next: Phase B (device layer), starting with the display driver for the Cardputer-Adv.

### Phase B (device layer), 2026-10-16: host-verifiable part done

Built and compile-checked for both boards; nothing flashed, nothing run on the device (deferred to the final phase).

Done:
- `firmware-rs/device/Cargo.toml`: board features `board-box` (default) and `board-cardputer-adv`, forwarded to core. Core's default board is no longer implied.
- `firmware-rs/device/src/board.rs`: per-board hardware switches (PSRAM, backlight on the expander or a GPIO).
- `firmware-rs/device/src/lcd_spi.rs`: ST7789V2 over SPI for the Adv: reset pin, init from core's sequence, window and pixels in chunks. The box's i8080 driver is unchanged (`lcd.rs`).
- `firmware-rs/device/src/keyboard.rs`: TCA8418 driver (I2C 0x34): 7x8 matrix, event queue, pressed set.
- `firmware-rs/device/src/main.rs`: box-only items gated (XL9555 expander, the i8080 bus, PSRAM, `lcd_read`); Adv path: SPI2 panel, backlight on G38, keyboard. K0 stays on G0.
- `firmware/partitions-cardputer-adv.csv`: 8 MB layout, two 3 MB OTA slots, 1.75 MB voices.
- `tools/build-firmware.sh`: `BOARD=cardputer-adv` selects the features, the 8 MB header, the Adv table, and the output `firmware-rs/device/build-cardputer-adv`. The default (box) output is unchanged. The ELF path now comes from cargo, so a machine-wide target directory works.
- Verified: box and Adv `cargo build --release` clean; `cargo clippy` clean; `BOARD=cardputer-adv tools/build-firmware.sh` produces a 2.8 MB app (3 MB slot), and the box build still produces its image.
- `espflash` installed with `cargo install espflash --locked` into `~/.cargo/bin/espflash` (v4.6.0), used by the build script.

Open, to settle in the final phase:
- K1 and K2 on the Adv: the TCA8418 key numbers for `;` and `.` are not in the vendor sources. `KNOWN_KEYS` in `keyboard.rs` is empty, so K1 and K2 are never pressed until the numbers are read on the device.
- Speaker amplifier enable (AMP_EN): not found in the schematic, so the Adv build skips it. The codec probe (ES8311 at 0x18) is shared with the box and unverified on the Adv.
- Memory: without PSRAM, the LCD frame and the heap sit in internal RAM. The heap sizes are provisional.
- Bootloader: the build uses the box's prebuilt `bootloader.bin`. It should work on 8 MB flash, but that is unverified.
- Not yet done: the Mac and Linux apps and the daemon know only the box's flash layout (`firmware/partitions.csv`), so the Adv build is not yet reachable from them (Phase C).

### Flash script and keyboard mapping, 2026-10-16

- `tools/flash.sh` is board-aware: `BOARD=box` (default) or `BOARD=cardputer-adv`. The Adv writes its build from `firmware-rs/device/build-cardputer-adv` and its otadata at `0x7d0000` (the box keeps `0xa10000`). The port is found on macOS (`/dev/cu.usbmodem*`) and on Linux (the Espressif by-id link, `/dev/ttyACM*`). The esptool is chosen with `ESPTOOL` (default `esptool` on PATH). The script was not run end to end: that would flash the Cardputer, which waits for the final phase. Port detection was tested here and finds `/dev/ttyACM0`.
- Keyboard: the vendor's `keyboard.cpp` (M5Cardputer-UserDemo, branch CardputerADV) gives the 4 logical rows by 14 columns, the `_key_value_map` table, and `remap()`, which converts the TCA8418 key number (`row * 10 + column + 1`) into that layout. From it: `;` is logical (row 2, column 11) = TCA8418 key 57, and `.` is logical (row 3, column 11) = key 58. These are now in `firmware-rs/core/src/keymap.rs` (`key_number`, `logical_position`, with host tests that round-trip all 56 keys) and in the device `keyboard.rs`, which reads K1 = `;` and K2 = `.`. Verified against the vendor table, not yet on a board.
- Still open: the speaker amplifier enable is not in the vendor's `hal_config.h` either (the file has the keyboard interrupt, IR, SD card and LoRa/GPS pins only). The next place to look is M5Unified's board definition for the Cardputer-Adv.

### Open items resolved as far as the sources and the build allow, 2026-10-16

- Speaker amplifier enable: not a GPIO on the Cardputer-Adv. M5Unified (`src/M5Unified.inl`, `_speaker_enabled_cb_cardputer_adv`) enables the speaker through ES8311 register writes at address 0x18 (power-up, DAC, headphone output). Our codec init writes the same power-up registers. The I2S pins (BCK G41, WS G43, DATA G42) match. Not yet heard on the board.
- Bootloader: `firmware-rs/bootloader/sdkconfig.cardputer-adv` (8 MB, the Adv partition table, rollback on) and `tools/build-bootloader.sh` with `BOARD=cardputer-adv`, which writes `bootloader-cardputer-adv.bin`. Built with the ESP-IDF v5.2.2 installed at `~/esp/esp-idf` (the box's bootloader was built with v6, which isn't installed here). The toolchain install put the Xtensa tools in `~/.espressif/tools/xtensa-esp-elf`. The image's flash-size field reads 8 MB. `build-firmware.sh` uses it for the Adv build.
- Memory without PSRAM: estimated, not measured. Static RAM (`.data`, `.bss`, the heap regions) is about 172 KB; the heap regions alone total about 164 KB. With the Wi-Fi driver's buffers, this is likely above the S3's usable internal RAM (about 320 KB). The firmware's `MEMORY INTERNAL` line will show the real figure on the first device run. Fallbacks if short: a smaller JSON heap, or no Wi-Fi/relay on the Adv.

### Final build state, 2026-10-16

- ESP-IDF v6.0.1 installed at `~/esp/esp-idf-v6` (shallow clone of the tag, then `install.sh esp32s3`; the Xtensa tools are in `~/.espressif/tools/xtensa-esp-elf`). This is the version the box's boot log (`daemon/src/fixtures/box-boot-ota-layout.txt`) names for the box's bootloader.
- `firmware-rs/bootloader/bootloader-cardputer-adv.bin` is rebuilt with v6.0.1 for 8 MB flash (header size field 8 MB).
- `BOARD=cardputer-adv tools/build-firmware.sh` builds: app 2,804,112 B (3 MB slot), partition table for 8 MB, the v6 bootloader, otadata at 0x7d0000. Output in `firmware-rs/device/build-cardputer-adv/`.
- Not flashed to the Cardputer. Flashing and on-device checks are for the final phase.
- Minimization proposal: `.agent_context/goals/G3.md`.

### Device checks on the unit, 2026-10-17

- Boot with the Adv build: OK (v6.0.1 bootloader, 8 MB header; `AUDIO CODEC ES8311`; `BUTTONS READY`; `MEMORY INTERNAL 38848/163856`).
- K1 (`;`): switches duty and pomodoro. OK.
- K0 (BOOT, G0): starts, pauses and resumes the focus timer. OK.
- K2 (`.`): needs a Mac, not tested.
- Speaker: not yet confirmed by ear.
- Wi-Fi/Relay: not viable on the Adv as built (38.8 KB of heap free before any network start). Profile A in G3.
- Speaker, confirmed on the unit (fast-clock build): the focus-end voice line plays through the ES8311 and the speaker. The speaker-enable question is closed.

### Linux desktop screenshot acceptance, 2026-10-17

- The user confirmed the screenshot function working in the Linux desktop app with the physical Cardputer-ADV over
  USB after a fresh daemon/app restart. The shared host decoder now reads `SHOT BEGIN` dimensions rather than assuming
  320×240; the standalone serial screenshot tool follows the same rule. No firmware change was needed for this fix.
- Five focused Rust tests, four simulated-device screenshot scenarios (no expected failures), and two standalone-tool
  tests passed. Automated PNG checks cover 240×135 Adv and 320×240 BOX output. The manual run's saved image was not
  independently inspected. See `docs/linux-host-testing.md` for the full record and remaining acceptance boundaries.
- This closes the Linux screenshot acceptance item only; board identity, safe update targeting and other device/app
  gates remain separate work.
