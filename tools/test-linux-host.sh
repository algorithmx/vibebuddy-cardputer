#!/usr/bin/env bash
# Exercise the real Linux daemon against the Cardputer-ADV firmware core without a box or desktop.
set -euo pipefail
cd "$(dirname "$0")/.."

if [[ "$(uname -s)" != Linux ]]; then
    echo "These host integration tests require Linux."
    exit 1
fi

cargo build --locked -p vibebuddyd --target-dir target
cargo build --locked -p vibebuddy-desktop --example host_status_probe --target-dir target
# Keep Adv and BOX features in separate builds: workspace tests select BOX through daemon dev-dependencies.
cargo build --locked -p vibebuddy-firmware-core --example simulator \
    --no-default-features --features board-cardputer-adv --target-dir target/cardputer-adv

export VIBEBUDDY_HOST_TESTS=1
python3 -m unittest discover -s tools -p test_linux_host.py -v "$@"
