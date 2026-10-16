#!/usr/bin/env bash
# Flash the Rust firmware over the native USB port. The images come from tools/build-firmware.sh; the voices
# partition and the settings area are left alone, so a firmware change keeps the voice.
#
#   BOARD=box (default)        ALIENTEK ATK-DNESP32S3-BOX, 16 MB flash
#   BOARD=cardputer-adv        M5Stack Cardputer-Adv, 8 MB flash
#
# Usage: [BOARD=...] tools/flash.sh [serial-port]
# Without a port, the script looks for the board's native USB port: /dev/cu.usbmodem* on macOS, and
# /dev/ttyACM* or the Espressif by-id link on Linux. Pass the port explicitly if more than one matches.
# With only the UART bridge connected, use tools/flash-bridge.sh. To go back to the C firmware: tools/flash-c.sh.
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
board="${BOARD:-box}"
case "${board}" in
    box)
        build_dir="${BUILD_DIR:-${repo_root}/firmware-rs/device/build}"
        # The offset of otadata in firmware/partitions.csv.
        otadata_offset=0xa10000
        ;;
    cardputer-adv)
        build_dir="${BUILD_DIR:-${repo_root}/firmware-rs/device/build-cardputer-adv}"
        # The offset of otadata in firmware/partitions-cardputer-adv.csv.
        otadata_offset=0x7d0000
        ;;
    *)
        echo "BOARD must be box or cardputer-adv, not '${board}'." >&2
        exit 2
        ;;
esac
serial_port="${1:-}"

find_port() {
    local ports=()
    shopt -s nullglob
    case "$(uname -s)" in
        Darwin)
            ports=(/dev/cu.usbmodem*)
            ;;
        Linux)
            ports=(/dev/serial/by-id/*Espressif_USB_JTAG* /dev/ttyACM*)
            ;;
    esac
    shopt -u nullglob
    # by-id links and the ttyACM device they point to can both match; keep the real device names.
    local unique=()
    local port real
    for port in "${ports[@]}"; do
        real="$(readlink -f "${port}")"
        if [[ ! " ${unique[*]:-} " == *" ${real} "* ]]; then
            unique+=("${real}")
        fi
    done
    if [[ ${#unique[@]} -ne 1 ]]; then
        echo "Expected exactly one native USB serial port, found ${#unique[@]}: ${unique[*]:-none}. Pass the port explicitly." >&2
        exit 1
    fi
    echo "${unique[0]}"
}

if [[ -z "${serial_port}" ]]; then
    serial_port="$(find_port)"
fi

# The ESP32-S3 needs esptool's stub, which the packaged esptool on some systems lacks. ESPTOOL names the binary to
# use, e.g. ESPTOOL="python3 -m esptool" with an ESP-IDF environment; the default is esptool on PATH.
esptool_cmd="${ESPTOOL:-esptool}"
if ! ${esptool_cmd} version >/dev/null 2>&1; then
    echo "esptool not found (ESPTOOL='${esptool_cmd}'); install it (pip install esptool) or set ESPTOOL." >&2
    exit 1
fi

"${repo_root}/tools/build-firmware.sh"

# One connection writes all four images (the app to ota_0, and a blank otadata so the board boots it) and resets once at
# the end, so a half-updated firmware never runs. The bootloader header already says the board's flash size; keep the
# flash settings rather than letting esptool patch them.
# shellcheck disable=SC2086
# Underscore spellings work in esptool 4.x and 5.x alike.
${esptool_cmd} --chip esp32s3 --port "${serial_port}" --before default_reset --after hard_reset \
    write_flash --flash_mode keep --flash_freq keep --flash_size keep \
    0x0 "${build_dir}/bootloader.bin" \
    0x8000 "${build_dir}/partition-table.bin" \
    0x10000 "${build_dir}/vibebuddy-fw.bin" \
    "${otadata_offset}" "${build_dir}/ota-data-initial.bin"
