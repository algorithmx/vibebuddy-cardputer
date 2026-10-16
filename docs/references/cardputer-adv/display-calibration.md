# Cardputer-Adv panel calibration

How the 240x135 ST7789V2 panel on the Cardputer-Adv was aligned with the firmware's frame, and what came out of it.
Status: panel offsets confirmed on the device, 2026-10-16 (`PANEL_OFFSET_X = 40`, `PANEL_OFFSET_Y = 53`). The scene-by-scene
check of the compact layout continues separately (see `.agent_context/plans/G2-cardputer-adv-plan.md`).

## Why calibration was needed

The firmware draws into a 240x135 frame (`firmware-rs/core/src/canvas.rs`). The ST7789V2 controller has more memory
than the panel shows, so the frame has to be placed inside that memory with a column and row offset. The offsets are
set in `firmware-rs/core/src/board.rs` (`PANEL_OFFSET_X`, `PANEL_OFFSET_Y`) and go into the LCD window command
(`firmware-rs/core/src/lcd.rs`, `WINDOW`). Wrong offsets hide the frame's outer pixels behind the bezel, so the first
thing to confirm is that every edge of the frame is visible.

No vendor source in `docs/references/cardputer-adv/` gives these offsets, so they were found on the device.

## Method

1. **Calibration screen.** `firmware-rs/adv-display-check/` is a display-only firmware for the Cardputer-Adv. Its first
   scene (scene 0) draws, on the 240x135 frame:
   - a one-pixel white frame on the outermost pixels of all four edges,
   - two green diagonals corner to corner,
   - a red centre cross,
   - six colour bars across the top row (red, green, blue, yellow, magenta, cyan),
   - the text `CALIBRATION` and `240 X 135`.

   The frame, the diagonals and the bars make any cut-off edge obvious: a missing white line means that edge is hidden.

   Scene 0 also shows a status line near the bottom: the BOOT (G0) level (`DOWN` or `UP`) and a press count. It lets
   you check the button without a serial monitor. Pressing BOOT steps to the next scene, and the count wraps the cycle.
2. **Flash the app only.** Build with `. ~/export-esp.sh && cargo build --release` in `firmware-rs/adv-display-check/`,
   convert the ELF with `esptool elf2image --flash_mode dio --flash_freq 80m --flash_size 8MB`, and write it at
   `0x10000` with `esptool write_flash`. The bootloader (`0x0`) and partition table (`0x8000`) are not written.
   The factory app was backed up first (see Restore below).
3. **Photograph the panel** and compare each edge against the frame.
4. **Change the offsets** in `board.rs`, rebuild, reflash, and repeat until every edge is visible.

## Results

| Attempt | `PANEL_OFFSET_X` | `PANEL_OFFSET_Y` | What the photo showed |
|---|---|---|---|
| 1 | 52 | 40 (portrait values) | Top rows hidden: colour bars missing, no white frame on the top edge. Right column hidden: no white frame on the right edge. Left and bottom edges visible. |
| 2 | 40 | 53 (landscape) | All four edges visible with the white frame, all six colour bars along the top, diagonals reaching the corners, colours correct (RGB order), text crisp. |

The result used in the firmware is attempt 2: `PANEL_OFFSET_X = 40`, `PANEL_OFFSET_Y = 53`. A later photo of the
final build confirmed it: the white frame shows on all four edges, the six colour bars run along the top, and the
diagonals reach the corners.

The attempt-1 failure matches the offsets being off by the portrait-to-landscape swap: 52 pushes the right column past
the visible area, and 40 for y leaves the top 13 rows above it. The landscape pair is the consistent fit. The offsets are
confirmed by the photo, not by a datasheet value.

## Debugging notes from the bring-up

These came up while getting the calibration screen and the scene cycle to run. They are for whoever next brings up the
panel or the buttons.

- **`Display` draws nothing until `start()` is called.** Every `set_*` and `tick` call checks a ready flag set by
  `start()`. The first calibration frame worked only because it is drawn straight into the frame buffer. Without
  `start()`, the scene change after a BOOT press ran but left the screen unchanged. The fix is to call
  `display.start(...)` once before the first scene.
- **Serial output needs a sink feature.** `esp-println` builds with no output when no sink is chosen. Use
  `default-features = false, features = ["esp32s3", "jtag-serial"]`. The default `auto` feature conflicts with
  an explicit sink, and the build fails with "exactly one of ... must be enabled".
- **Serial monitoring.** Open the port with `stty -F /dev/ttyACM0 115200 raw -echo -hupcl` so closing it does not reset
  the chip. Opening the port can reset the chip through DTR, so start the capture before the event you want to see.
- **Stale esptool.** The system `/usr/bin/esptool` (4.7) cannot read past `0x200000` in ROM-only mode. Use the ESP-IDF
  esptool 4.11 (`~/.espressif/python_env/idf5.2_py3.13_env/bin/python -m esptool`) for reads and writes.
- **Diagnostic code to remove.** The G0 status line on scene 0 and the 1-second `loop t=...` serial report in
  `firmware-rs/adv-display-check/src/main.rs` are debug aids. They are not part of the layout.

## Limits

- The result comes from a photo of one unit, at one brightness. Edges can look slightly different under other light.
- It checks the panel's visible area only. Colour depth and the backlight level were not measured.
- Unit-to-unit variation of the panel is not known; another board could need different offsets.

## Restore the factory firmware

The full 8 MB factory flash image was saved before any write:
`~/cardputer-adv-backup/factory-flash-8MB-2026-10-16.bin` (SHA-256 in the matching `.sha256` file, outside the repo).
To restore the factory app, write its 4 MB app slot back at `0x10000`:

```sh
# <scratch> is any writable directory outside the repo.
dd if=~/cardputer-adv-backup/factory-flash-8MB-2026-10-16.bin of=<scratch>/factory-app.bin bs=64K skip=1 count=64
/home/dabajabaza/.espressif/python_env/idf5.2_py3.13_env/bin/python -m esptool --port /dev/ttyACM0 write_flash 0x10000 <scratch>/factory-app.bin
```

Use the ESP-IDF esptool for this: the system `/usr/bin/esptool` (4.7) fails on reads past `0x200000` without the stub.

## Related

- Board constants: `firmware-rs/core/src/board.rs`
- Calibration firmware: `firmware-rs/adv-display-check/`
- Pin map and panel sources: `docs/references/cardputer-adv/extracts.md`
- Plan and execution log: `.agent_context/plans/G2-cardputer-adv-plan.md`
