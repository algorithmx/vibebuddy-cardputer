#!/usr/bin/env bash
# Build the Rust firmware and write the three flash images plus the build ID to firmware-rs/device/build:
#   bootloader.bin       → 0x0
#   partition-table.bin  → 0x8000
#   vibebuddy-fw.bin     → 0x10000 (ota_0)
#   ota-data-initial.bin → 0xa10000 (otadata, blank, so the box boots ota_0 rather than an older ota_1)
#   build.txt            matches the box's DISPLAY READY BUILD line character for character
#   version.txt          the firmware version (firmware-rs/device/Cargo.toml), the box's FIRMWARE VERSION line
# None of them touches the settings area at 0x9000 or the voices partition at 0x410000.
#
# FAST_CLOCK=1 builds the time-compressed acceptance firmware (five minutes of leisure become five seconds).
# BOARD=cardputer-adv builds for the M5Stack Cardputer-Adv (8 MB flash, no PSRAM) into firmware-rs/device/build-cardputer-adv,
# with firmware/partitions-cardputer-adv.csv. The default is the box.
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
device="${repo_root}/firmware-rs/device"
board="${BOARD:-box}"
case "${board}" in
    box) out="${device}/build"; partitions="firmware/partitions.csv"; flash_size="16mb"; board_features=(--no-default-features --features board-box) ;;
    cardputer-adv) out="${device}/build-cardputer-adv"; partitions="firmware/partitions-cardputer-adv.csv"; flash_size="8mb"; board_features=(--no-default-features --features board-cardputer-adv) ;;
    *) echo "BOARD must be box or cardputer-adv, not '${board}'" >&2; exit 2 ;;
esac

if ! command -v xtensa-esp32s3-elf-gcc >/dev/null 2>&1 && [[ -f "${HOME}/export-esp.sh" ]]; then
    # shellcheck disable=SC1091
    . "${HOME}/export-esp.sh"
fi
features=()
if [[ "${FAST_CLOCK:-0}" == "1" ]]; then
    features=(--features fast-clock)
fi
(cd "${device}" && cargo build --release "${board_features[@]}" ${features[@]+"${features[@]}"})
# The target directory comes from cargo, so a machine-wide target-dir setting (~/.cargo/config.toml) still finds the image.
target_dir="$(cd "${device}" && cargo metadata --format-version 1 --no-deps | python3 -c 'import json,sys; print(json.load(sys.stdin)["target_directory"])')"
elf="${target_dir}/xtensa-esp32s3-none-elf/release/vibebuddy-firmware"

mkdir -p "${out}"
# The ESP-IDF bootloader with app rollback enabled (ADR-0013), prebuilt by tools/build-bootloader.sh.
# The Cardputer-Adv has its own 8 MB bootloader (tools/build-bootloader.sh BOARD=cardputer-adv).
bootloader="${repo_root}/firmware-rs/bootloader/bootloader.bin"
if [[ "${board}" == "cardputer-adv" ]]; then
    bootloader="${repo_root}/firmware-rs/bootloader/bootloader-cardputer-adv.bin"
fi
cp "${bootloader}" "${out}/bootloader.bin"
# --flash-size writes the board's flash size into the app's image header, matching the bootloader's.
espflash save-image -S --chip esp32s3 --flash-size "${flash_size}" "${elf}" "${out}/vibebuddy-fw.bin"
# espflash can't parse a partition table with a custom subtype (voices is 0x40), so we build it ourselves.
python3 "${repo_root}/tools/make-partition-table.py" "${repo_root}/${partitions}" "${out}/partition-table.bin" >/dev/null

python3 - "${out}" <<'PY'
import sys
from pathlib import Path

out = Path(sys.argv[1])
(out / "ota-data-initial.bin").write_bytes(b"\xff" * 0x2000)
app = (out / "vibebuddy-fw.bin").read_bytes()
marker = b"VIBEBUDDY-BUILD:"
start = app.index(marker) + len(marker)
build = app[start:app.index(b"\0", start)].decode()
(out / "build.txt").write_text(build + "\n")
print(f"app {len(app)} B, build ID {build}")
PY
sed -n 's/^version = "\(.*\)"$/\1/p' "${device}/Cargo.toml" | head -n 1 > "${out}/version.txt"
echo "firmware version $(cat "${out}/version.txt")"
ls -l "${out}"
