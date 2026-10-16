# Codebase hardware binding (G2, codebase half)

Scope: how the present device firmware is bound to the hardware it runs on, and what a port to the M5Stack Cardputer-Adv would have to change. Read-only analysis; no source was modified.

## 1. Which tree is the target

**Chosen target: `firmware-rs/` (Rust, esp-hal + embassy). It is the active firmware.** `firmware/` (C, ESP-IDF) is the fallback.

Evidence:
- `justfile:23-30`: `just firmware` / `just flash` build and flash the Rust images; `just flash-c` is the C fallback ("if the Rust firmware misbehaves").
- `tools/build-firmware.sh:1-9, 19-22`: builds `firmware-rs/device` and writes `bootloader.bin`, `partition-table.bin`, `vibebuddy-fw.bin`, `ota-data-initial.bin` to `firmware-rs/device/build`.
- `tools/flash.sh:1-3, 40-46`: writes the Rust images with esptool at 0x0 / 0x8000 / 0x10000 / 0xa10000.
- `.github/workflows/release-firmware.yml:31-64`: CI and releases use `esp-rs/xtensa-toolchain` with `buildtargets: esp32s3`, `espflash@4.6.0`, and `tools/build-firmware.sh`. `ci.yml:2-3` says the device firmware is left to release-firmware.yml.
- `docs/adr/0006-firmware-in-rust-with-esp-hal.md` (the ADR that made this the firmware), `docs/firmware-bringup.md:3-4, 106` (bring-up on hardware 2026-09-28; deletion of `firmware/` is planned once the Rust firmware "has proven itself").
- `firmware-rs/device/Cargo.toml:1-7` describes itself as "Firmware for the box (ATK-DNESP32S3-BOX, ESP32-S3)".

**Important: the repository has no Cardputer or M5Stack support.** A search of the whole repo for `cardputer`, `m5stack`, `m5 ` finds only `.vscode/settings.json:2` (path) and the goal files `.agent_context/goals/G2.md`, `G3.md`. Every board binding in both trees is for the **ALIENTEK ATK-DNESP32S3-BOX V1.1** (`docs/hardware.md`, `docs/hardware-research.md`). The port is therefore a full board-support change, not a pin tweak.

## 2. Hardware bindings in the Rust target (`firmware-rs/device`)

Pins and peripherals are listed in the header comment at `firmware-rs/device/src/main.rs:1-11` and set up in `main()` at `main.rs:322-421`.

### 2.1 MCU, flash, PSRAM, bootloader

| Item | Where | Binding |
|---|---|---|
| Chip / target | `device/Cargo.toml:17` (`esp-hal` features `esp32s3`), `device/rust-toolchain.toml:2` (`channel = "esp"`), `tools/build-firmware.sh:19-22` (`cargo build --release`, target `xtensa-esp32s3-none-elf`) | ESP32-S3, Xtensa, esp-hal ~1.2.2. Target is hard-wired by the toolchain and build script. |
| Flash size | `tools/build-firmware.sh:31` (`espflash save-image --flash-size 16mb`), `firmware-rs/bootloader/sdkconfig.defaults:2` (`CONFIG_ESPTOOLPY_FLASHSIZE_16MB=y`), `firmware/sdkconfig.defaults:4` | 16 MB assumed in three places. |
| Flash mode/freq | `firmware-rs/bootloader/sdkconfig.defaults:4-5` (DIO, 40 MHz) | Bootloader fixed to DIO 40 MHz. |
| PSRAM | `device/src/main.rs:323-330` (`esp_alloc::psram_allocator!`, `PsramConfig { FlashFreq40m }`), comment at `main.rs:323-325` "8 MB octal PSRAM" | Octal PSRAM required. The allocator is set up before anything else; relay heap is placed in PSRAM (`device/src/relay.rs:202`). Also `report_memory` (`main.rs:533`) reports PSRAM free. |
| Bootloader | `firmware-rs/bootloader/CMakeLists.txt:4-6`, `firmware-rs/bootloader/sdkconfig.defaults:6-8` (`CONFIG_PARTITION_TABLE_CUSTOM_FILENAME="../../firmware/partitions.csv"`, `CONFIG_BOOTLOADER_APP_ROLLBACK_ENABLE=y`), prebuilt `firmware-rs/bootloader/bootloader.bin` | Prebuilt ESP-IDF 2nd-stage bootloader that reads the shared partition table. Must be rebuilt for a new chip/flash config. |
| Build ID / app descriptor | `device/build.rs:1-39`, `device/src/main.rs:59` (`esp_app_desc!`) | Hardware-neutral. Note `build.rs:17-22` requires `xtensa-*-elf-gcc` on PATH. |
| Panic / reset | `device/src/main.rs:62-67` (`custom_halt` -> `software_reset`) | Hardware-neutral. |
| CPU clock | `device/src/main.rs:322` (`CpuClock::max()`) | Neutral except that the max clock may differ on another S3 module. |

### 2.2 Partition table (shared with the C firmware)

`firmware/partitions.csv:1-12`, built into an image by `tools/make-partition-table.py`:

- `nvs` 0x9000 (24 KB), `phy_init` 0xf000, **`ota_0` 0x10000 (4 MB)**, **`voices` 0x410000 (2 MB, subtype 0x40)**, **`ota_1` 0x610000 (4 MB)**, `otadata` 0xa10000, `link` 0xa12000 (8 KB).
- Total used is about 10.1 MB of a 16 MB part. If the Cardputer-Adv flash is smaller than 16 MB, the two 4 MB OTA slots plus the 2 MB voice partition cannot fit, and the layout (ADR-0013, ADR-0003, ADR-0014) must change.

The voice partition is written by flash-independent code: `core/src/voice_pack.rs`, `device/src/storage.rs` (`find_partition`, `read_unaligned`).

### 2.3 Display: ST7789 over 8-bit i8080 parallel (LCD_CAM + DMA)

| Item | Where | Binding |
|---|---|---|
| Bus pins | `device/src/main.rs:358-375`: CS GPIO1, DC GPIO2, WR GPIO42, RD GPIO41 (held high, `main.rs:139, 358`), D0..D7 = GPIO40, 39, 38, 12, 11, 10, 9, 46 | Entirely board-specific. |
| Bus clock | `device/src/main.rs:363` (`Rate::from_mhz(10)`), `device/src/lcd.rs:6-12` (`I8080`, `DmaTxBuf`) | 10 MHz i8080 with DMA through the LCD_CAM peripheral (`main.rs:359`). |
| Controller | `firmware-rs/core/src/lcd.rs:1-4, 18-36` (ST7789 command table: SWRESET, SLPOUT, MADCTL 0x36 = 0x00 / 0x20 / 0x60, COLMOD 0x55 / 0x65, INVON, DISPON) | ST7789 is assumed. The MADCTL and invert values are tuned to this panel ("this panel only shows correct colors when inverted", `core/src/lcd.rs:24`). |
| Resolution / rotation | `firmware-rs/core/src/canvas.rs:7-9` (`WIDTH = 320`, `HEIGHT = 240`, `FRAME_BYTES = 320*240*2`), `core/src/lcd.rs:38-40` (window cols 0-319, rows 0-239 hard-coded), `core/src/lcd.rs:32-35` (landscape via MV/MX) | **Hard-coded 320x240 landscape, RGB565, big-endian framebuffer.** Every drawing coordinate in `core/src/display.rs` (1620 lines) is in these units (e.g. `display.rs:287-288` eyes at 143/169, `display.rs:858` card widths of 188). A different panel size is a layout port, not only a driver port. |
| Reset | `core/src/lcd.rs:19-20` ("No reset pin wired, so software reset") | Software reset only. |
| Frame transfer | `device/src/lcd.rs:37-101`, `device/src/lcd.rs:46-48` (dummy parameter byte workaround for esp-hal i8080, "the one place the Rust firmware drives the panel differently from the C firmware") | Driver quirk specific to i8080 + esp-hal. An SPI panel would not need it. |
| Backlight | `device/src/main.rs:73` (`XL9555_LCD_BACKLIGHT = 0x80`, P0.7), `main.rs:170-172` (`set_backlight` via expander), `main.rs:257-263` (`init_display` sets direction and output low before init) | Backlight is switched by an I2C I/O expander, not by a PWM or GPIO. Dimming is not possible (on/off only). |

### 2.4 Keys: three keys, K0 on GPIO0, K1/K2 on an XL9555 expander

| Item | Where | Binding |
|---|---|---|
| K0 (BOOT key) | `device/src/main.rs:396` (`Input::new(GPIO0, Pull::Up)`), `main.rs:9, 293` (`k0.is_low()`), `core/src/buttons.rs:5` | GPIO0, active low. GPIO0 is also the strapping pin for download mode. |
| K1 / K2 | `device/src/main.rs:75-76` (`XL9555_K1 = 0x10` = P0.4, `XL9555_K2 = 0x08` = P0.3), `main.rs:286-296` (`init_buttons`, `read_buttons`, `set_direction` to input) | Behind the XL9555 expander on I2C0 (see 2.5). Active low. The read path is an I2C transaction every poll. |
| Debounce / long press | `firmware-rs/core/src/buttons.rs:10-11` (`DEBOUNCE_MS = 40`, `LONG_PRESS_MS = 1000`), `buttons.rs:15-18` (K0 pomodoro/abandon) | Hardware-neutral logic, but the gesture map (K0 short/long, K1 short/long, K2 long) is written for three keys and drives the device menu (`core/src/menu.rs`, `docs/device-menu.md`, ADR-0007). |
| Poll rate | `device/src/main.rs:523-524` ("This wait is the key sampling period: too long would miss quick taps", 10 ms) | 10 ms loop; a keyboard matrix or a different expander changes the scan cost. |

Buttons are the single biggest UX dependency: the Mac-side and device-side flows (K0 pomodoro, K1 duty/pomodoro and menu, K2 "back to the agent", K2 long mute) all assume exactly three keys. See `docs/firmware-bringup.md:86-92` ("Buttons") and `docs/adr/0007-device-menu-behind-a-long-k1.md`.

### 2.5 I2C0 and the XL9555 I/O expander

- `device/src/main.rs:353-356`: I2C0 at 400 kHz, SDA GPIO48, SCL GPIO45.
- `device/src/main.rs:69-76`: XL9555 at 0x20. Registers used: input port 0x00, output port 0x02, config (direction) 0x06.
- `device/src/main.rs:88-112`: `Expander` implements read / write / set_output / set_direction over I2C.
- Expander bits used: P0.7 LCD backlight (`0x80`), P0.5 amplifier enable (`0x20`, `main.rs:74` and `init_audio` at `main.rs:278`), P0.4 K1 (`0x10`), P0.3 K2 (`0x08`).
- Confirmed on the hardware: `docs/hardware.md`, "Confirmed on the hardware" list (K2 = P0.3, K1 = P0.4, backlight P0.7, speaker P0.5). The same I2C0 also carries the ES8311 (0x18) and a possible touch controller CHSC5432 at 0x2E on some boxes (`docs/hardware.md`, "Touch, and what else is on I2C0"). The firmware does not use touch.

### 2.6 Audio: ES8311 codec, I2S0 (TDM/Philips, 16-bit stereo, no MCLK)

| Item | Where | Binding |
|---|---|---|
| I2S pins | `device/src/main.rs:382-392`: BCLK GPIO21, WS GPIO13, DOUT GPIO14, no MCLK pin set; I2S0 + DMA_CH1 | Board-specific. `device/src/main.rs:1-11` and `firmware/main/agent_audio.c:21-23`. |
| Format | `device/src/main.rs:382-390` (`TdmConfig::new_tdm_philips`, `Data16Channel16`, stereo, 24 kHz), `firmware-rs/core/src/audio.rs:151` (`SAMPLE_RATE = 24000`) | 24 kHz. Voice assets (`firmware/main/assets/*.pcm`) were generated for 24 kHz. |
| Codec | `firmware-rs/core/src/audio.rs:164` (`ES8311_ADDRESS = 0x18`), `core/src/audio.rs:199-280` (full ES8311 register sequence, documented as "single speed, osr 0x10, lrck 0x00FF, bclk_div 4. Without MCLK the multiplier is fixed at ×8", `audio.rs:244`), `core/src/audio.rs:156-158` (volume 20..100, default 65), `core/src/audio.rs:179-195` (volume to register mapping) | The register sequence is ES8311-specific and assumes BCLK as the only clock (no MCLK). Any other codec (or ES8311 with MCLK wired) needs its own sequence. |
| Codec detection | `device/src/main.rs:264-284` (`init_audio`: probe chip ID register 0xFD at 0x18, else NS4168 variant with no volume), `core/src/audio.rs:203-205` | ES8311 vs. NS4168 variants both exist in code. The NS4168 path is a no-volume fallback (C: `firmware/main/agent_audio.c:36, 269-270`). |
| Amplifier enable | `device/src/main.rs:74` and `init_audio` `main.rs:265-284`, speaker enable at `main.rs:278-282` (P0.5 on XL9555) | Speaker amp is enabled through the expander. |
| Output path | `device/src/audio.rs:1-26` (DMA stream, 16000 B, 1000 B chunks, I2S runs continuously so the clock never stops), `device/src/audio.rs:152-215` | The stream design depends on BCLK running continuously ("a clock that stops and restarts tends to pop"). |
| Microphone | none | No microphone input is used anywhere in the firmware. `docs/hardware.md` lists a mic on the box but no code reads it. |

### 2.7 Serial: UART0 (bridge) and the native USB Serial/JTAG

- UART0 TX GPIO43, RX GPIO44 at 115200 (`device/src/main.rs:341-344`, `device/src/serial.rs:1-10`), comment says "to the CH343 bridge". `docs/hardware-research.md` and `docs/hardware.md` say the board is CH340C or native; the code says CH343. This is a doc/code disagreement (see section 6).
- Native USB Serial/JTAG (`device/src/main.rs:346-347`, `device/src/serial.rs:96-113, 118-175`): the chip's built-in USB-serial, D+/D- on GPIO19/20 (Espressif; per `docs/hardware-research.md`, not confirmed on the box schematic).
- USB output has no flow control; the firmware drops output after a 3 ms patience window when no host reads (`serial.rs:124-126, 160-175`). `docs/hardware.md` ("Confirmed on the hardware", line 35) states the enumeration is `303A:1001` on the box.
- `docs/hardware.md` ("Confirmed on the hardware"): opening the USB serial port resets the chip, so the host side must tolerate re-enumeration.
- Other links built on these: LAN/Wi-Fi (section 2.8), Relay (2.8).

### 2.8 Radio (Wi-Fi), mDNS, TLS, TRNG

- Wi-Fi: `device/Cargo.toml:21-22` (`esp-radio` with `esp32s3,wifi`), `device/src/net.rs:103-135` (`WifiController::new(wifi, ...)`), DHCPv4 (`net.rs:116`), TCP server on port 7340 (`net.rs:28`), 2 connection slots (`net.rs:30`), mDNS `_vibebuddy._tcp` (`net.rs:5-6`, `device/Cargo.toml:25-26`). Credentials come over USB (`net.rs:57`, signalled at `main.rs:429` and `main.rs:511`). Hardware-neutral for any ESP32-S3 with Wi-Fi, but needs a board antenna and the RF section to be the same class; RF performance is unverified on any new board.
- Bluetooth: none. No BLE code in either tree (grep for `ble`/`bluetooth` finds nothing in firmware trees).
- TRNG: `device/src/main.rs:415-422` uses `RNG` plus **ADC1** (`TrngSource::new(peripherals.RNG, peripherals.ADC1)`), comment at `main.rs:416-418` says "nothing else uses ADC1". This reserves ADC1 on the chip. Any board change that wants ADC1 for battery sensing would conflict with the entropy source.
- TLS (Relay): `device/Cargo.toml:27-28` (`mbedtls-rs` with `esp32s3`), `device/src/relay.rs:1-2` (relay.korekore.ai), `relay.rs:202, 219`, `device/certs/relay-roots.pem`. Hardware-neutral except for RNG and PSRAM use.

### 2.9 Storage

- On-chip flash, read/write/erase via `esp-storage` (`device/src/storage.rs:1-30`, `device/Cargo.toml:37`). Hardware-neutral; the partition table gives the layout (section 2.2).
- Settings area at 0x9000 (`tools/build-firmware.sh:2-3`): NVS and the `link` partition persist across flashes (`firmware/partitions.csv:3-12`, ADR-0014). Port must keep those partitions or accept the one-time settings reset documented in `docs/firmware-bringup.md:45`.
- No SD card, no external flash, no RTC: none are used (grep for `sd`, `microsd`, `tf card`, `sdmmc` returns nothing in firmware trees).

### 2.10 Things the firmware does NOT use

Grep of both trees for battery/ADC/charger, IMU (QMI8658 / BMI270 type), SD/TF, LED, buzzer, microphone, touch, IR, RTC returns nothing. These parts are present on the ALIENTEK box (`docs/hardware.md`) and may be on the Cardputer-Adv, but no code depends on them, so they are not a porting cost. Any use would be new work (see section 4).

### 2.11 Board abstraction seam

- Hardware-neutral logic lives in `firmware-rs/core`. The seams to the hardware are two traits: `core/src/display.rs:113` (`Screen`: frame buffer, `present`, `set_backlight`) and `core/src/firmware.rs:61` (`Board: Screen`: time, serial write, LAN/Relay send, flash, RNG, `init_display`, `init_audio`, `init_buttons`, `read_buttons`, `play`, `set_volume`), plus `core/src/storage.rs:21` (`Flash`).
- The device implements them in `device/src/main.rs:161` (`impl Screen for DeviceBoard`) and `main.rs:175` (`impl Board for DeviceBoard`), and in `device/src/storage.rs:62` (`impl Flash for SharedFlash`).
- Good news: the port should be able to replace `DeviceBoard` and the `device/src/lcd.rs` / `device/src/audio.rs` drivers. Bad news: the buttons trait (`Levels { k0, k1, k2 }`, `core/src/buttons.rs`, `Board::read_buttons`) and the 320x240 canvas are in core, so changing the keyboard or the screen size touches core too.

## 3. Hardware-independent vs. hardware-bound

Hardware-independent (pure logic, protocol, rendering against a canvas; can be kept as-is):
- `core/src/firmware.rs` (state machine, 1608 lines), `pomodoro.rs`, `leisure.rs`, `menu.rs`, `link.rs` (noise sessions), `lan.rs`, `relay_box.rs`, `noise.rs`, `ota.rs`, `character_pack.rs`, `voice_pack.rs`, `voices.rs`, `adpcm.rs`, `storage.rs` (trait), `text.rs`, `lines.rs`, `look.rs`.
- Rendering code in `core/src/display.rs` is hardware-neutral in logic but hard-codes 320x240 coordinates (see 2.3).
- `device/src/serial.rs` protocol and ring logic, `device/src/net.rs` (TCP/mDNS/Wi-Fi control), `device/src/relay.rs` (TLS, WebSocket): radio/IP stack is neutral for any ESP32-S3 with Wi-Fi.
- Voice / look / leisure assets (`firmware/main/assets/*.pcm`, Character packs): neutral except sample rate (24 kHz) and the codec path.

Hardware-bound (board-specific code or constants):
- `device/src/lcd.rs` and `core/src/lcd.rs` (panel controller, bus, MADCTL, window size).
- `core/src/canvas.rs:7-9` and every 320x240 coordinate in `core/src/display.rs`.
- `device/src/audio.rs` and `core/src/audio.rs` (codec, I2S clocking, amp enable).
- `device/src/main.rs` pin maps, I2C0 + XL9555, K0/K1/K2 wiring, UART/USB pins, PSRAM and flash config.
- `firmware-rs/bootloader/*` and `firmware/partitions.csv` (flash size assumptions).
- `device/src/main.rs:416-421` (ADC1 entropy source reservation) and `device/src/serial.rs` (UART0 and USB pins).

## 4. Hardware-bound items ranked by porting effort and risk

Effort and risk are my judgment from the code above; they assume the Cardputer-Adv hardware differs from the ALIENTEK box in the ways that are normal for a different board. The Cardputer-Adv specs themselves are not checked here (that is the web-research half of G2).

1. **Display (highest effort, high risk).** Panel controller and bus change (i8080 parallel vs. whatever the Cardputer uses), and the resolution is a different size, so `canvas.rs` and every `display.rs` coordinate change. The layout (eyes, cards, text) must be reworked and re-verified visually. Pixel-compare tooling (`tools/compare-display.sh`) only compares Rust against C on the same 320x240 canvas, so it will not catch a layout error on a new size without a new reference.
2. **Keys (high effort, high risk for UX).** Three keys (K0 GPIO0, K1/K2 via XL9555) drive the whole interaction model: pomodoro, duty switch, the menu (ADR-0007), mute and "back to agent". A keyboard on the Cardputer-Adv changes the input driver, the `Levels` type and the gesture map. Mapping gestures to a different key set is a product decision, not only code.
3. **I2C expander and I2C0 (medium-high effort).** Backlight, speaker amp enable and K1/K2 all sit on XL9555 at 0x20. If the Cardputer-Adv has no such expander, backlight, amp enable and key reads each need a new mechanism (GPIO, PWM for dimming, or another IC). This is the single place where several features fail together.
4. **Audio codec and I2S (medium effort, medium risk).** ES8311 register sequence (`core/src/audio.rs:199-280`) and I2S pins (`device/src/main.rs:382-392`) are reusable only if the new board uses an ES8311 (or another codec whose sequence is written). The no-MCLK assumption (`core/src/audio.rs:244`) and the BCLK-continuous design (`device/src/audio.rs:1-9`) must hold on the new board. Speaker amp enable also depends on item 3.
5. **Flash and partition layout (medium effort, high risk if flash shrinks).** Two 4 MB OTA slots, a 2 MB voice partition and a `link` partition require at least about 10 MB; `tools/build-firmware.sh:31`, the bootloader config and `partitions.csv` all assume 16 MB. A smaller flash forces a new layout, with a different OTA story (ADR-0013) and possibly a smaller voice pack.
6. **PSRAM (medium effort, high risk if absent).** The code allocates the framebuffer (DMA, 150 KB), relay heap and JSON in PSRAM first (`device/src/main.rs:323-330`, `device/src/relay.rs:202`). Without PSRAM the internal heap may not hold the DMA buffers and the relay/TLS heap. Needs a memory audit on the new board.
7. **Serial and USB pins (low-medium effort).** UART0 (GPIO43/44, CH343 bridge) is a second link; the USB Serial/JTAG link is the primary (`docs/firmware-bringup.md:52`, step 2 uses the native port). On the Cardputer-Adv, the native USB pins are GPIO19/20 on the S3 and may be wired differently, and a UART bridge may not exist. The UART path can be dropped, but the USB path must be verified (the "one cable" claim in `docs/hardware.md` depends on it).
8. **Boot button and strapping (low effort).** K0 is GPIO0 (BOOT). Any Cardputer-Adv key on GPIO0 keeps the same behaviour; otherwise a different boot-mode strategy is needed.
9. **Radio / TLS / TRNG (low effort).** Wi-Fi, mDNS and mbedTLS are chip features. The ADC1 entropy source reservation (`device/src/main.rs:416-421`) only matters if the new board wants ADC1 for something else.
10. **Build and CI (low effort, but must be done).** `firmware-rs/device/build.rs`, `tools/build-firmware.sh`, `tools/flash.sh`, bootloader prebuilt, release workflow and the `DISPLAY READY BUILD` identifier. Board name string `atk-dnesp32s3-box` appears in docs; the device reports only the build stamp, so the Mac side does not need the board name.

Not needed for parity: battery, IMU, SD, IR, RTC, LED, buzzer, touch, mic (none used by code).

## 5. The C firmware (`firmware/`): status

- Same board, same pins. Display pins `firmware/main/agent_display.c:27-46` (LCD CS 1, DC 2, RD 41, WR 42, D0-D7 40/39/38/12/11/10/9/46, I2C SDA 48, SCL 45, `psram_trans_align` at `agent_display.c:1449`). Buttons `agent_buttons.c:11-18` (XL9555 0x20, K0 GPIO0, K1 0x10, K2 0x08). Audio `agent_audio.c:21-29` (I2S BCLK 21, WS 13, DOUT 14, ES8311 0x18, speaker P0.5 mask 0x20, NS4168 fallback). Build: `firmware/sdkconfig.defaults:1-4` (esp32s3, 16 MB flash, custom partition table), `firmware/main/CMakeLists.txt:3` (drivers for i2c, i2s, lcd, usb serial, esp_codec_dev).
- It is kept as the fallback until on-device verification passes (ADR-0006, `docs/firmware-bringup.md:94-96`), with `just flash-c` (`justfile:31-32`, `tools/flash-c.sh`). `tools/flash-c.sh:1-3, 9-10` needs ESP-IDF v5.5.3 (`~/.espressif/tools/activate_idf_v5.5.3.sh`).
- `firmware/host_tests` and `tools/test-*.sh` cover pomodoro, leisure and voice pack on the host; they are not part of the Rust path.
- Status: **maintained as a fallback, not as the primary target**. Scheduled for deletion. A port should change the Rust tree first; the C tree needs the same pin changes only if the fallback must keep working on the new board.

## 6. Status of the other tree and doc inconsistencies

- `firmware/` (C) is **a supported fallback** with the same board binding and its own build path; it is not abandoned, but it is not the primary target.
- Inconsistencies to resolve during the port:
  - UART bridge chip: `device/src/main.rs:11` and `device/src/serial.rs:1` say CH343; `docs/hardware-research.md` and the board description say CH340C or native USB. Unknown which chip is on the box.
  - Key mapping: `firmware-rs/device/src/main.rs:8` says K1 = XL9555 bit 4 and K2 = bit 3, matching the probes in `docs/hardware.md` (K1 P0.4, K2 P0.3). `docs/hardware-research.md` warns that BOX3 or DNESP32S3 pinouts must not be used for this board; the code uses the ALIENTEK BOX V1.1 pins only.
  - Serial path: `docs/hardware.md:36` says the box enumerates through its native USB Serial/JTAG "rather than an external CH340/CP210/FTDI bridge", while `docs/firmware-bringup.md:19` lists tests on "both the native USB port and the UART bridge", and `device/src/main.rs:341` calls UART0 the CH343 bridge. Whether a bridge chip exists on the box is not settled in the docs.
  - `docs/hardware.md` says the expander is at 0x20; the touch controller is at 0x2E on some boxes only (not used).

## 7. Unknowns that block a precise plan

1. Cardputer-Adv specs that this codebase cannot answer: flash size and PSRAM (the code assumes 16 MB and 8 MB octal), display controller and resolution, the keyboard matrix or expander, the codec (is it an ES8311, and is MCLK wired?), the amp enable, the USB and UART chips, battery and charging, and whether there is an I2C expander at all. These come from M5Stack's official documentation in G2's web-research half.
2. Whether the Cardputer-Adv has a UART bridge, which decides if the UART serial path (`device/src/serial.rs`) can stay.
3. Whether the Cardputer-Adv has a 3-key-equivalent input for K0/K1/K2, or whether a keyboard-to-gesture map must be designed. This is the main product decision.
4. Whether 320x240 layouts can be scaled or must be re-drawn for the new panel (`core/src/display.rs`, 1620 lines of hard-coded coordinates).
5. Whether the ES8311 + I2S + BCLK-only design works on the new codec and whether MCLK on the new board changes the codec register sequence.
6. Whether the voice partition (2 MB) and two OTA slots (4 MB each) fit in the new flash, and whether an OTA-capable layout (ADR-0013) is required by the product.
7. Memory: whether the new board's PSRAM (if any) is enough for the 150 KB DMA frame buffer and relay TLS heap.
8. Whether the C fallback must work on the new board at all (cost of maintaining a second board port vs. deleting `firmware/` once the Rust port is proven).

## 8. Key file index

- Rust device: `firmware-rs/device/src/main.rs`, `lcd.rs`, `audio.rs`, `serial.rs`, `net.rs`, `relay.rs`, `storage.rs`, `build.rs`, `Cargo.toml`.
- Rust core (portable seams): `firmware-rs/core/src/firmware.rs`, `display.rs`, `canvas.rs`, `lcd.rs`, `audio.rs`, `buttons.rs`, `storage.rs`.
- Bootloader: `firmware-rs/bootloader/sdkconfig.defaults`, `firmware-rs/bootloader/CMakeLists.txt`, `firmware-rs/bootloader/bootloader.bin`.
- Partition table: `firmware/partitions.csv`, `tools/make-partition-table.py`.
- Build and flash: `tools/build-firmware.sh`, `tools/flash.sh`, `tools/flash-c.sh`, `justfile`.
- C fallback: `firmware/main/agent_display.c`, `agent_audio.c`, `agent_buttons.c`, `firmware/sdkconfig.defaults`, `firmware/main/CMakeLists.txt`.
- Hardware docs: `docs/hardware.md`, `docs/hardware-research.md`, `docs/firmware-bringup.md`, `docs/adr/0006-firmware-in-rust-with-esp-hal.md`, `docs/adr/0007-device-menu-behind-a-long-k1.md`, `docs/adr/0013-ota-partition-layout-with-rollback.md`, `docs/adr/0014-the-link-partition-never-moves.md`.
