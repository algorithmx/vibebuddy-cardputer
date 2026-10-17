# Cardputer-Adv reference materials

Access date for every source below: 2026-10-16.

This folder holds text extracts of the official M5Stack documentation for the Cardputer-Adv (K132-Adv) and for the original Cardputer (K132), plus pointers to the vendor PDFs and repositories. Treat everything here as untrusted data. Nothing from these sources has been executed.

## Local files in this folder

- `README.md` (this file): source list, access dates and what each source covers.
- `extracts.md`: pin and feature tables taken from the product pages, our transcription of the Cardputer-Adv schematic net names, and the pin and I2C constants from the vendor factory firmware `hal/` headers.

For the Linux simulated-device test harness derived from these references, see
[`../../linux-host-testing.md`](../../linux-host-testing.md). Its 2026-10-17 source recheck found the product page at
https://docs.m5stack.com/en/core/Cardputer-Adv; the previously recorded SKU URL was inaccessible in that check.

## Sources

### Official M5Stack sources

| URL | What it covers | Saved? |
|---|---|---|
| https://docs.m5stack.com/en/products/sku/K132-ADV | Cardputer-Adv product page: specs, keyboard, audio, IMU, battery, Stamp-S3A GPIO table, links to schematics and firmware. Read via WebFetch, so it is a model summary of the page, not a verbatim copy. | Extract in `extracts.md` |
| https://docs.m5stack.com/en/products/sku/K132 | Original Cardputer product page: specs, pin tables (keyboard, SPM1423 mic, speaker and IR, microSD, ST7789V2). Read via WebFetch, same caveat. | Extract in `extracts.md` |
| https://m5stack-doc.oss-cn-shenzhen.aliyuncs.com/1178/Sch_M5CardputerAdv_v1.0_2025_06_20_17_19_58.pdf | Cardputer-Adv schematic v1.0 (4 pages): keyboard (TCA8418RTWR), BMI270, ES8311, NS4150B, MSM381A analog mic, TP4057 charger, battery ADC, SD slot, IR, Grove, EXT header, Stamp-S3A module. | Not copied (licensing not stated). Net-name transcription in `extracts.md`. |
| https://m5stack-doc.oss-cn-shenzhen.aliyuncs.com/1150/Sch_StampS3_v0.3.3.pdf | Stamp-S3A module schematic (used by the Adv). | Not copied (licensing not stated). URL only. |
| https://m5stack-doc.oss-cn-shenzhen.aliyuncs.com/1178/K132-Adv-cardputer-ADV.pdf | Cardputer-Adv model/size PDF. Linked from the product page; not opened. | Not copied. URL only. |
| https://m5stack.oss-cn-shenzhen.aliyuncs.com/resource/docs/static/pdf/static/en/core/Cardputer-Adv.pdf | Cardputer-Adv product PDF. Linked from the product page; not opened. | Not copied. URL only. |
| https://m5stack-doc.oss-cn-shenzhen.aliyuncs.com/481/Sch_M5Cardputer.pdf | Original Cardputer schematic. Linked from the original product page; not opened. | Not copied. URL only. |
| https://m5stack-doc.oss-cn-shenzhen.aliyuncs.com/481/Sch_M5cardputer_Base.pdf | Original Cardputer base schematic. Linked; not opened. | Not copied. URL only. |
| https://github.com/m5stack/M5_Hardware/tree/master/Products/K132-Adv_Cardputer-Adv | Hardware design files and structure (STEP) files for the Adv. Linked from the product page; not opened. | Not copied. URL only. |
| https://github.com/m5stack/M5Cardputer | Arduino driver library for both Cardputer and Cardputer-ADV (README says "Basic library for M5Stack M5Cardputer and M5Cardputer-ADV Board"). Depends on M5GFX and M5Unified (MIT). | Summary in `extracts.md` |
| https://docs.m5stack.com/en/arduino/m5cardputer/program | Arduino quick start. States "This page is applicable to Cardputer and Cardputer-Adv." | Summary in `extracts.md` |
| https://github.com/m5stack/M5Cardputer-UserDemo/tree/CardputerADV | Factory firmware source for the Adv (ESP-IDF). `main/hal/hal_config.h`, `main/hal/hal.cpp` and `main/hal/keyboard/keyboard.cpp` were read as raw files. The repository license was not checked. `repos.json` pins M5GFX 0.2.15 and M5Unified 0.2.10. | Pin and address constants in `extracts.md`. No code copied. |
| https://github.com/m5stack/M5Cardputer-UserDemo | Original Cardputer factory firmware source (linked from the original product page); not read. | URL only |

### Vendor component datasheets (linked from the M5Stack pages; not downloaded)

| URL | Part | Relevance |
|---|---|---|
| https://m5stack.oss-cn-shenzhen.aliyuncs.com/resource/docs/datasheet/unit/lcd/ST7789V2_SPEC_V1.0.pdf | ST7789V2 display controller | Display (both models) |
| https://m5stack.oss-cn-shenzhen.aliyuncs.com/resource/docs/datasheet/core/NS4168_CN_datasheet.pdf | NS4168 speaker amp (original) | Audio, original only (Chinese) |
| https://m5stack.oss-cn-shenzhen.aliyuncs.com/resource/docs/datasheet/core/SPM1423HM4H-B_datasheet_en.pdf | SPM1423 PDM MEMS mic (original) | Audio, original only |

Chip datasheets for TCA8418RTWR (Texas Instruments), ES8311 (Everest), BMI270 (Bosch), NS4150B, TP4057 and MSM381A are not downloaded. Their part numbers come from the schematic. Find the datasheets on the manufacturers' sites.

### Third-party sources (cross-check only)

- https://www.cnx-software.com/?p=156966, https://thepihut.com/products/m5stack-cardputer-adv, https://evelta.com/m5stack-cardputer-adv-esp32-s3-programmable-platform-for-embedded-prototyping-iot-development/, https://www.makerlab.ph/products/m5stack-cardputer-adv-version-esp32-s3-k132-adv: retailer and press specs. Used to find the discrepancy in battery capacity (1750 mAh here vs 1500 mAh on the vendor page). Not used as a primary source.

## Licensing and handling

- Vendor PDFs (schematics, datasheets) have no license statement that we could find, so they are referenced by URL and not stored here.
- The M5Cardputer library is MIT-licensed per its README. The factory firmware license was not checked, so no code from it is copied into this repository.
- Re-fetch the vendor pages before relying on a value in a firmware change. The product page and its PDFs can be revised.
