# Cardputer-Adv: extracts from the official sources

Access date: 2026-10-16. See `README.md` in this folder for the URL of each source.

Two kinds of text appear below:

- **Product-page extract**: values read from the M5Stack product page with WebFetch. These are a model summary of the page, so check the exact wording on the page before quoting it.
- **Schematic transcription**: net names read from the vendor schematic PDF (rendered and read visually). Pin-to-net mapping is ours. Where the schematic does not show the GPIO, this is stated.

## 1. Cardputer-Adv pin map (product page, Stamp-S3A GPIO)

| Function | GPIO(s) | Signal names on the page |
|---|---|---|
| LCD ST7789V2 | G38, G33, G34, G35, G36, G37 | DISP_BL, RST, RS, DAT, SCK, CS |
| RGB LED power switch | G38 | Page says: set GPIO38 high before controlling the RGB LED. Same pin as the LCD backlight name. |
| Audio codec ES8311 | G8, G9, G41, G46, G43, G42 | SDA, SCL, SCLK, ASDOUT, LRCK, DSDIN |
| IMU BMI270 | G8, G9 | SDA, SCL (shared I2C bus) |
| Keyboard TCA8418RTWR | G8, G9, G11 | SDA, SCL, INT (shared I2C bus) |
| IR emitter | G44 | IR TX |
| Battery ADC | G10 | ADC |
| microSD | G12 (CS), G14 (MOSI), G40 (CLK), G39 (MISO) | |
| EXT 2.54-14P | G3 (RESET), G4 (INT), G6 (BUSY), G40, G8 (SDA), G14, G9 (SCL), G39, G13, G5, G15 | |
| Grove HY2.0-4P (PORT.CUSTOM) | G2, G1 | Power: 5V and GND |
| User button | G0 | Boot/download pin |

Download mode (per the page): power switch OFF, hold G0, power on, release G0.

## 2. Original Cardputer pin map (product page, K132)

The product page gives these tables. Its flattened text does not show every pin assignment. The page itself flags that G43 appears in two tables.

| Function | GPIO(s) | Notes |
|---|---|---|
| SPM1423 mic (PDM) | G46 (DAT), G43 (CLK) | |
| microSD | G12, G14, G40, G39 | Same as Adv |
| ST7789V2 LCD and RGB LED | G38, G33, G34, G35, G36, G37 | Same as Adv |
| Keyboard and battery detect | G10 (ADC battery), G7, G6, G5, G4, G3, G15, G13, G11, G9, G8 | Page lists a 74HC138 decoder with Y7~Y0 and A2/A1/A0. The exact mapping of G3-G15 and G8-G11 is not shown. |
| Speaker (NS4168) and IR | G41 (BCLK), G42 (SDATA), G43 (LRCLK), G44 (IR TX) | |
| HY2.0-4P Grove | G2, G1 | PORT.CUSTOM |

## 3. Factory firmware constants (`M5Cardputer-UserDemo`, branch `CardputerADV`)

From `main/hal/hal_config.h`:

```
#define HAL_PIN_KEYBOARD_INT   11
#define HAL_PIN_IR_TX          44
#define HAL_PIN_SPI_MISO       GPIO_NUM_39
#define HAL_PIN_SPI_MOSI       GPIO_NUM_14
#define HAL_PIN_SPI_SCLK       GPIO_NUM_40
#define HAL_PIN_SD_CARD_CS     GPIO_NUM_12
#define HAL_PIN_GPS_TX         GPIO_NUM_13
#define HAL_PIN_GPS_RX         GPIO_NUM_15
#define HAL_PIN_LORA_NSS_GPIO  5
#define HAL_PIN_LORA_RST_GPIO  3
#define HAL_PIN_LORA_BUSY_GPIO 6
#define HAL_PIN_LORA_DIO1_GPIO 4
```

The LoRa and GPS defines are present in the Adv firmware header. The board has no LoRa and no GPS on its schematic. Those defines look like they were carried over from a different board (the header also has a `cap_lora868` folder). Do not treat them as Adv hardware.

From `main/hal/keyboard/keyboard.cpp` (as summarised by WebFetch):

- Uses the Adafruit TCA8418 driver with `matrix(7, 8)`, so the scan is 7 rows by 8 columns.
- Interrupt on `HAL_PIN_KEYBOARD_INT` (GPIO11), `GPIO_INTR_ANYEDGE`.
- The driver does not pass an I2C address. The default from the Adafruit library applies (TCA8418 datasheet: 0x34). The schematic shows "7-bit Address 34H".
- Key map is 4 rows by 14 columns after remapping.

From `main/hal/hal.cpp`:

- `M5.begin()`, then `M5.Speaker.begin()`, `M5.In_I2C.scanID()` for an I2C scan, `keyboard_init()`, `spi_init()`. This means the firmware uses M5Unified (`M5.*`) APIs.
- No explicit IMU or codec init appears in this file. Those are probably handled by M5Unified/M5GFX.

## 4. Cardputer-Adv schematic transcription (v1.0, 2025-06-20)

Source: the schematic PDF listed in `README.md`. Read visually. Net names are as printed; blank means not visible.

Page 1: power, USB, module, headers, SD, IR, buttons

- Battery charger U1 = TP4057 (charge program resistor R1 = 0.8 Ω; CHRG/STDBY pins drawn, not connected to a GPIO on this page). Our calculation from the TP4057 formula I_BAT ≈ 1000 V / R_PROG gives about 1.25 A. This is an estimate, not a vendor value.
- Battery sense divider R8 = 100 kΩ, R12 = 100 kΩ, C10 = 100 nF to G10 (ADC). This halves the battery voltage at the ADC.
- Power switch SW1 (SPDT) between BAT and the boost/buck path. SW2 (SPDT) selects Grove +5V from +5VIN or +5VOUT.
- Buttons: BTN1 is EN (reset). BTN2 is G0.
- Module M1 = Stamp-S3A (DIP-1.27). Expansion headers P1 (17-pin) and P2 (11-pin) carry the module GPIOs.
- SD card J3 (TF-015): CS G12, MOSI G14, CLK G40, MISO G39, each with 33 Ω series resistors.
- IR LED on G44 through 22 Ω.
- HY2.0 Grove J2 is labelled IIC_SCL / IIC_SDA on the header and G1 / G2 on the net labels. The product page says PORT.CUSTOM G2 (signal) and G1. Which of G1/G2 is SCL or SDA is not confirmed.
- FPC1 (8-pin, 0.5 mm): 3V3, 5V, GND, IO1, IO2, IO3. Nets G7, G8, G9 appear on pins 4, 6 and 7. Exact pin order not confirmed.
- Boost converter U2 = SY7088 (5 V out, D2 SS34). Buck U4 = SY8089 (3.3 V out, L2 4.7 µH).

Page 2: keyboard and audio

- Keyboard controller U9 = TCA8418RTWR, 7-bit address 34H. SDA on G8, SCL on G9, INT on G11, RESET tied to a 10 kΩ pull-up.
- Matrix: ROW0..ROW6 and COL0..COL7 lines on the TCA8418 to 56 switches (S1..S56). The column side also goes through 22 Ω resistors to Y0..Y7 nets. Exact pin-to-net order was not verified.
- Audio codec U6 = ES8311. Its I2C pins are on G8 (SDA) and G9 (SCL). I2S: G41 = SCLK, G46 = ASDOUT (mic data to MCU), G43 = LRCK, G42 = DSDIN (DAC data from MCU). MCLK is not on a GPIO on this page.
- Microphone U8 = MSM381A3729H9BPC. This is an analog microphone with an electret-type front end. It feeds MIC1P/MIC1N of ES8311. The original Cardputer used a PDM mic (SPM1423) on G46/G43. The Adv does not use PDM.
- Speaker amplifier U5 = NS4150B. CTRL pin is net AMP_EN. Its GPIO was not visible on this page.
- Headphone jack J1 (PJ-342) with detect switching. Page: "When a 3.5mm headphone jack is inserted, the speaker amplifier will be disabled."
- IMU U7 = BMI270, 7-bit address 69H. SDA/SCL on G8/G9. INT1/INT2 are not connected.

Page 3: EXT header P3 (HDR-SMD 14P, 2.54 mm)

- Pins: RESET (G3), INT (G4), BUSY (G6), SCK (G40), MOSI (G14), MISO (G39), CS (G5), SDA (G8), SCL (G9), GPS-RX (G13), GPS-TX (G15), 5VIN, 5VOUT, GND.
- The "GPS-RX/GPS-TX" labels are the same LoRa/GPS names seen in the firmware header. Treat them as the EXT header's generic UART signal names, not as a GPS on the board.

Page 4: blank apart from the frame.

## 5. Not established by these sources

- The Stamp-S3A GPIO for USB D+/D- (native USB). The schematic shows CC1/CC2 for the USB-C connector on the module side, but the pin assignment was not read.
- The GPIO for AMP_EN.
- The ES8311 I2C address (CE strap) and the MCLK source.
- Whether the Adv has PSRAM. Vendor pages list 8 MB flash and no PSRAM; ESP32-S3FN8 has no PSRAM.
