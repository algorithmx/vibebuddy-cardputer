# M5Stack Cardputer-Adv: research summary (G2, web research half)

Access date for all sources: 2026-10-16.
Local copies and extracts: `docs/references/cardputer-adv/` (`README.md` lists URLs; `extracts.md` holds the tables and the schematic transcription).

Source keys used below:

- **[ADV]** https://docs.m5stack.com/en/products/sku/K132-ADV (Cardputer-Adv product page, vendor). Read by WebFetch, so values are a model summary and should be checked on the page.
- **[K132]** https://docs.m5stack.com/en/products/sku/K132 (original Cardputer product page, vendor). Same caveat.
- **[SCH-ADV]** https://m5stack-doc.oss-cn-shenzhen.aliyuncs.com/1178/Sch_M5CardputerAdv_v1.0_2025_06_20_17_19_58.pdf (Cardputer-Adv schematic v1.0, vendor). Read visually; the net names are ours.
- **[FW-ADV]** https://github.com/m5stack/M5Cardputer-UserDemo/tree/CardputerADV (vendor factory firmware, ESP-IDF, raw files read).
- **[LIB]** https://github.com/m5stack/M5Cardputer (vendor Arduino library, README).
- **[QS]** https://docs.m5stack.com/en/arduino/m5cardputer/program (vendor Arduino quick start).
- **[RET]** third-party retail and press pages (thepihut, evelta, makerlab, cnx-software). Cross-check only.

## 1. Device spec comparison

| Item | Cardputer-Adv | Original Cardputer | Source |
|---|---|---|---|
| Module | Stamp-S3A (DIP-1.27) | Stamp-S3 (K132 page lists "Stamp-S3 schematics v0.2") | [ADV], [SCH-ADV], [K132] |
| SoC | ESP32-S3FN8, Xtensa LX7 dual-core, 240 MHz | ESP32-S3FN8, dual-core, 240 MHz | [ADV], [K132] |
| Flash | 8 MB | 8 MB | [ADV], [K132] |
| PSRAM | None listed; FN8 part has no PSRAM | None listed | [ADV], [K132]; the FN8 part number is our inference |
| Wi-Fi / BLE | 2.4 GHz Wi-Fi, BLE 5 (chip features). Page gives only current draw | 2.4 GHz Wi-Fi (BLE not listed) | [ADV] (currents), [K132], [RET] for BLE 5 |
| Display | ST7789V2, 1.14", 240 x 135, SPI | ST7789V2, 1.14", 240 x 135 | [ADV], [K132] |
| Keyboard | 56 keys, 4 x 14 layout, 160 gf. Scanned by TCA8418RTWR (I2C) | 56 keys, 4 x 14. Scanned by 74HC138 decoder | [ADV], [K132], [SCH-ADV] |
| User buttons | BTN1 = EN (reset), BTN2 = G0 | 1 reset, 1 user | [K132], [SCH-ADV] |
| IMU | BMI270 (6-axis), I2C 0x69 | None | [ADV], [SCH-ADV] |
| Microphone | MEMS microphone, 65 dB SNR. Schematic: analog MSM381A3729H9BPC into ES8311 | SPM1423 PDM digital mic | [ADV], [SCH-ADV], [K132] |
| Audio codec | ES8311 (I2S + I2C) | None (I2S speaker amp only) | [ADV], [SCH-ADV] |
| Speaker amp | NS4150B; 8 Ω 1 W speaker | NS4168 (I2S); 8 Ω 1 W speaker | [ADV], [K132] |
| 3.5 mm headphone jack | Yes (disables speaker amp when inserted) | Not listed | [ADV], [SCH-ADV] |
| microSD | Yes, SPI, CS = G12 | Yes, same GPIO set | [ADV], [K132], [FW-ADV] |
| IR emitter | 1, on G44 | 1, on G44; range 410 cm at 0° | [ADV], [K132] |
| Grove | HY2.0-4P, PORT.CUSTOM on G2/G1, 5 V | HY2.0-4P, 1 port, same GPIOs | [ADV], [K132] |
| Expansion header | EXT 2.54-14P (I2C G8/G9, UART G13/G15, RESET G3, INT G4, BUSY G6, SPI on G40/G14/G39/G5) plus FPC 8-pin (G7/G8/G9) | Not listed | [ADV], [SCH-ADV] |
| Battery | Vendor: 1500 mAh. Retail: 1750 mAh (conflict) | 120 mAh internal + 1400 mAh in base | [ADV], [RET], [K132] |
| Charger | TP4057 (R_PROG 0.8 Ω, about 1.25 A by TP4057 formula, our estimate) | Not checked | [SCH-ADV] |
| Battery ADC | G10, 2:1 divider (100 k / 100 k) | G10 | [ADV], [SCH-ADV], [K132] |
| Power switch | SW1 (SPDT) on battery path; SW2 selects Grove 5 V source | Not checked | [SCH-ADV] |
| Charging note | "When charging the Cardputer-Adv, please switch the power to ON." | Not checked | [ADV] |
| USB | USB-C on the Stamp-S3A module (CC1/CC2 on schematic). Native USB GPIO not confirmed | "USB OTG, USB Serial/JTAG" | [K132], [SCH-ADV] (Stamp schematic) |
| Download mode | Power OFF, hold G0, power ON, release G0 | Not checked | [ADV] |
| Standby current | 0.23 µA at 4.2 V (power switch OFF) | 0.26 µA at 4.2 V | [ADV], [K132] |
| Operating current | 120.2 mA at 4.2 V | Keyboard 165.7 mA; IR 255.6 mA | [ADV], [K132] |
| Operating temperature | 0 to 40 °C | 0 to 40 °C | [ADV], [K132] |
| Dimensions | 84.0 x 54.0 x 19.6 mm | 84.0 x 54.0 x 19.7 mm | [ADV], [K132] |
| Weight | 81.0 g | 92.3 g | [ADV], [K132] |
| SDK support | M5Cardputer Arduino library (covers both), UiFlow2, ESP-IDF factory firmware, PlatformIO example | Same library and platforms | [QS], [LIB], [ADV], [K132] |
| Library dependencies | M5GFX (0.2.15 in FW-ADV repos.json), M5Unified (0.2.10) | Same | [LIB], [FW-ADV] |

## 2. Pinmap (Cardputer-Adv, Stamp-S3A GPIO)

### 2.1 Display (ST7789V2) and RGB LED

| Signal | GPIO | Source |
|---|---|---|
| Backlight (DISP_BL) | G38 | [ADV] |
| RST | G33 | [ADV] |
| RS (D/C) | G34 | [ADV] |
| DAT (MOSI) | G35 | [ADV] |
| SCK | G36 | [ADV] |
| CS | G37 | [ADV] |
| RGB LED power enable | G38 (set high before driving the RGB LED) | [ADV] |

The original Cardputer uses the same GPIO set for the LCD ([K132]). Backlight and RGB power share G38 on both boards, so a backlight-off call also affects the RGB LED's power. This is an inference from the shared pin, not a tested behaviour.

### 2.2 Keyboard (TCA8418RTWR, I2C)

| Signal | GPIO | Source |
|---|---|---|
| I2C SDA | G8 | [SCH-ADV], [ADV] |
| I2C SCL | G9 | [SCH-ADV], [ADV] |
| INT | G11 | [SCH-ADV], [ADV] |
| Address | 0x34 (7-bit) | [SCH-ADV] |
| Matrix | 7 rows x 8 cols (driver `matrix(7, 8)`), remapped to 4 x 14 | [FW-ADV] |
| Interrupt pin in firmware | GPIO11 (`HAL_PIN_KEYBOARD_INT`), any edge | [FW-ADV] |

Original Cardputer: the page lists a 74HC138 scan with G3-G7, G13, G15 and G8-G11 (exact mapping not shown). The original keyboard is not I2C, so the keyboard driver must change. Matrix scanning code cannot be reused.

### 2.3 Audio (ES8311 codec, NS4150B amp, analog mic)

| Signal | GPIO | Source |
|---|---|---|
| I2C (codec control) | G8 SDA, G9 SCL (shared with keyboard and IMU) | [SCH-ADV] |
| I2S SCLK (BCLK) | G41 | [ADV], [SCH-ADV] |
| I2S LRCK | G43 | [ADV], [SCH-ADV] |
| I2S DSDIN (DAC data to codec) | G42 | [ADV], [SCH-ADV] |
| I2S ASDOUT (ADC data from codec) | G46 | [ADV], [SCH-ADV] |
| Speaker amp enable (AMP_EN) | Not found in the schematic text we read | unknown |
| MCLK | Not on a GPIO we could see | unknown |
| Mic | Analog MEMS into ES8311 MIC1P/MIC1N (not PDM) | [SCH-ADV] |

Original Cardputer: I2S speaker (NS4168) on G41 (BCLK), G42 (SDATA), G43 (LRCLK). PDM mic (SPM1423) on G46 (DAT) and G43 (CLK). The page notes that G43 is in two tables. On the Adv, G43 is LRCK only. Code that uses a PDM mic must be replaced with the ES8311 I2S path.

### 2.4 microSD (SPI)

| Signal | GPIO | Source |
|---|---|---|
| CS | G12 | [ADV], [FW-ADV] |
| MOSI | G14 | [ADV], [FW-ADV] |
| SCLK | G40 | [ADV], [FW-ADV] |
| MISO | G39 | [ADV], [FW-ADV] |

Same on both boards ([K132]). No change expected.

### 2.5 IMU (BMI270, I2C)

| Signal | GPIO | Source |
|---|---|---|
| SDA / SCL | G8 / G9 (shared I2C bus) | [ADV], [SCH-ADV] |
| Address | 0x69 (7-bit, SDO high per schematic) | [SCH-ADV] |
| INT1 / INT2 | Not connected on the schematic | [SCH-ADV] |

The original Cardputer has no IMU.

### 2.6 Battery

| Signal | GPIO | Source |
|---|---|---|
| Battery voltage ADC | G10 (divider 2:1, 100 k / 100 k, 100 nF filter) | [ADV], [SCH-ADV] |
| Charger status (CHRG/STDBY) | Drawn on TP4057, not wired to a GPIO on the page we read | [SCH-ADV] |

Original Cardputer: G10 is also listed as the battery ADC ([K132]).

### 2.7 Other GPIOs

| Function | GPIO | Source |
|---|---|---|
| IR TX | G44 | [ADV], [SCH-ADV], [FW-ADV] |
| User button (BOOT) | G0 | [SCH-ADV] |
| Grove | G1 / G2 (which is SCL/SDA not confirmed) | [ADV], [SCH-ADV] |
| FPC 8-pin | G7, G8, G9 (pin order not confirmed) | [SCH-ADV] |
| EXT 14P | G3, G4, G5, G6, G8, G9, G13, G14, G15, G39, G40 | [ADV], [SCH-ADV] |

## 3. Adaptation implications (for the next step of G2)

1. The display driver, microSD, battery ADC and IR code can keep the same GPIOs. Verify with the Adv hardware before relying on it.
2. The keyboard path changes from a GPIO matrix to I2C TCA8418 with an INT interrupt on G11. This is the largest driver change.
3. Audio changes from a PDM-mic plus I2S-amp model to an ES8311 I2S codec with an analog mic. Use the M5Unified/M5Cardputer library (`M5.Speaker`, `M5.Mic`) rather than custom I2S code if possible.
4. The IMU (BMI270) is new and shares I2C with the keyboard and codec. The IMU is an added capability and cannot be preserved from the original board; it is an optional enhancement.
5. The 3.5 mm jack and the amp-disable behaviour are new and must be handled in audio playback.
6. The factory firmware `hal_config.h` also lists LoRa and GPS pins. They are not on the Adv board and should not be used.

## 4. Unknown or unverified

- AMP_EN GPIO, ES8311 I2C address strap, ES8311 MCLK source.
- Stamp-S3A USB D+/D- GPIOs (USB-C routing). The schematic shows CC1/CC2 at the module, but we did not read the USB pins.
- The exact mapping of the original keyboard's 74HC138 pins (for the original board only).
- Charger status GPIO (if any) and whether the Adv firmware reads charge state.
- Radio spec on the Adv page: only current draw is listed. Wi-Fi and BLE capability come from the chip family, not from a vendor statement for the module.
- Battery capacity: vendor page 1500 mAh, retail 1750 mAh.
- The factory firmware (`M5Cardputer-UserDemo`) is ESP-IDF based; we did not confirm which M5Unified version the Adv factory firmware actually uses (repos.json names 0.2.10).
- The vendor datasheets for TCA8418, ES8311, BMI270, NS4150B, TP4057 and MSM381A were not fetched.
- The vendor schematic PDF was read visually at 110 dpi. Small net labels (e.g., on the FPC and Grove headers) may be misread; confirm with a larger render or the original file before wiring.
