# Device probe, 2026-10-16 (read-only)

Connected device, found on this Linux host, read-only probes only. Nothing was flashed or erased.

## USB (sysfs, port not opened)
- Device 303a:1001, Espressif "USB JTAG/serial debug unit", serial E8:F6:0A:98:BF:CC.
- Bus 3, port 1. Interfaces: CDC-ACM (ttyACM0), CDC data, and a vendor JTAG interface.
- `/dev/ttyACM0`, group dialout, user has access.
- The product string is the chip's default for native USB-Serial-JTAG, so it does not identify the board.

## esptool (ROM-only, `--no-stub`, system esptool 4.7.0)
- Chip: ESP32-S3 (QFN56) revision v0.2.
- Embedded flash: 8 MB (XMC). Flash ID manufacturer 0x20, device 0x4017.
- PSRAM: none reported. The chip-features line lists WiFi and BLE only.
- MAC: e8:f6:0a:98:bf:cc.
- Flash mode: quad, set in eFuse.

## Partition table at 0x8000 (decoded)
| Name | Type | Offset | Size |
|---|---|---|---|
| nvs | data/nvs | 0x9000 | 16 KB |
| otadata | data/ota | 0xd000 | 8 KB |
| phy_init | data/phy | 0xf000 | 4 KB |
| factory | app | 0x10000 | 4096 KB |

This is not VibeBuddy's layout (`firmware/partitions.csv`: ota_0, ota_1, voices, link). The device is not running VibeBuddy firmware as flashed. Possibly vendor or example firmware, or a different project.

## Serial output (second probe, after the user approved it)
- Opened `/dev/ttyACM0` at 115200 with `-hupcl` (so closing the port does not reset the board). 20 s of idle capture: 0 bytes.
- Captured across a hard reset (esptool `read_mac`, which resets via RTS): only the ROM banner, `ESP-ROM:esp32s3-20210327` / `Build:Mar 27 2021`. Nothing after it.
- So the second-stage bootloader and the app print nothing on this port (logging off, or output on another interface). The silence is not a diagnosis of a crash.

## Flash contents (read-only)
- Bootloader at 0x0: standard ESP image header (magic 0xE9, 4 segments).
- App at 0x10000 (factory): valid ESP app description (magic 0xABCD5432):
  - project: `cardputer-adv`
  - version: `V0.9-36-ge824a76`
  - built: `09:53:09 Oct 21 2025`
  - ESP-IDF: `v5.4.2`
- Keyword strings in the app header include `cardputer-adv`.

This is M5Stack's Cardputer-Adv factory firmware, built with ESP-IDF v5.4.2. It is not VibeBuddy.

## Conclusions
- The project's firmware assumes 16 MB flash and 8 MB PSRAM. This device has 8 MB flash and no PSRAM, so it is not the box as the code describes it.
- 8 MB flash with no PSRAM matches the Cardputer-Adv's ESP32-S3FN8 (see `cardputer-adv-research.md`). This is consistent with the Adv but not proof: any 8 MB S3 without PSRAM would match.
- Side effect: each esptool run did a hard reset through RTS, so the device rebooted into whatever firmware it has.

## Board identity
- Strong evidence: the factory app is named `cardputer-adv`, and the flash size is 8 MB with no PSRAM (the Adv's ESP32-S3FN8).
- The visual check of the screen (the factory UI should be visible) is not done yet; it would confirm the physical board.

## Not yet checked
- Screen content (visual check by the user).
- Whether the factory firmware responds to any host command (not attempted; it would be an interactive exchange).

## Full flash backup (done)
- File: `~/cardputer-adv-backup/factory-flash-8MB-2026-10-16.bin` (outside the repo, not committed).
- Size: 8388608 bytes (8 MB). SHA-256 in `~/cardputer-adv-backup/factory-flash-8MB-2026-10-16.sha256`.
- Headers intact: 0x0 bootloader (0xE9), 0x8000 partition table (0xAA50), 0x10000 app (0xE9, app description 0xABCD5432).
- Tool: ESP-IDF's esptool 4.11 at `~/.espressif/python_env/idf5.2_py3.13_env/bin/python -m esptool` (stub-capable). The system `/usr/bin/esptool` 4.7 failed with a CRC error at 0x200000 in ROM-only mode, reproducibly, at both 115200 and 921600 baud. The stub version read the same region without error.
- The last 64 KB are not blank (not all 0xFF), so the region holds data. Not inspected yet.
