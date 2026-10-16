#!/usr/bin/env bash
# Rebuild firmware-rs/bootloader/bootloader.bin: the ESP-IDF second-stage bootloader with app rollback enabled
# (ADR-0013). The result is committed, so only run this when its configuration or ESP-IDF changes.
# Uses $IDF_PATH if set, else ~/esp/esp-idf-v6.
# BOARD=cardputer-adv builds the Cardputer-Adv's bootloader (8 MB, its partition table) into bootloader-cardputer-adv.bin.
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
project="${repo_root}/firmware-rs/bootloader"
board="${BOARD:-box}"
case "${board}" in
    box) defaults="sdkconfig.defaults"; output="bootloader.bin" ;;
    cardputer-adv) defaults="sdkconfig.cardputer-adv"; output="bootloader-cardputer-adv.bin" ;;
    *) echo "BOARD must be box or cardputer-adv, not '${board}'." >&2; exit 2 ;;
esac
# The box's bootloader was built with ESP-IDF v6 (~/esp/esp-idf-v6). Fall back to whatever ESP-IDF is installed at ~/esp/esp-idf.
if [[ -z "${IDF_PATH:-}" ]]; then
    if [[ -d "${HOME}/esp/esp-idf-v6" ]]; then
        IDF_PATH="${HOME}/esp/esp-idf-v6"
    else
        IDF_PATH="${HOME}/esp/esp-idf"
    fi
fi
export IDF_PATH

# shellcheck disable=SC1091
. "${IDF_PATH}/export.sh" >/dev/null
cd "${project}"
rm -rf build sdkconfig
idf.py -D "SDKCONFIG_DEFAULTS=${defaults}" set-target esp32s3 >/dev/null
idf.py -D "SDKCONFIG_DEFAULTS=${defaults}" bootloader >/dev/null
cp build/bootloader/bootloader.bin "${output}"
echo "ESP-IDF $(cat "${IDF_PATH}/version.txt" 2>/dev/null || echo unknown) ($(idf.py --version)), $(wc -c < "${output}" | tr -d ' ') B -> ${project}/${output}"
