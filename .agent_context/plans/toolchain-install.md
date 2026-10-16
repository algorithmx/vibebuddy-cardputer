# ESP32-S3 Xtensa Rust toolchain install

Host: Linux x86_64, Linux 7.0.0-34-generic. Goal: build `firmware-rs/device` for `xtensa-esp32s3-none-elf`. Nothing was flashed and no device was touched. No tracked repo files were modified.

## What the project expects

- `firmware-rs/device/rust-toolchain.toml`: `channel = "esp"`.
- `firmware-rs/device/.cargo/config.toml`: target `xtensa-esp32s3-none-elf`, `build-std = ["alloc", "core"]`, no runner (flash goes through `just flash` / `tools/flash.sh`).
- `docs/firmware-bringup.md` and the `justfile` document no install command.

## Before

- `espup` was not installed.
- `rustup` toolchains present: `stable`, `1.92.0`, `nightly-2025-12-04`. No `esp` toolchain.
- Default `cargo` 1.93.0 and `rustc` 1.93.0 from `~/.cargo/bin`.

## Commands run

1. `cargo install espup --locked` (log: `/tmp/claude-1000/-home-dabajabaza-Nutstore-Work-Project-vibebuddy-cardputer/dc6dad58-1d12-401d-9802-6b0d70481ac8/scratchpad/espup-cargo-install.log`)
2. `espup install` with default options (log: `/tmp/claude-1000/-home-dabajabaza-Nutstore-Work-Project-vibebuddy-cardputer/dc6dad58-1d12-401d-9802-6b0d70481ac8/scratchpad/espup-install.log`)
3. `. ~/export-esp.sh && cargo build --release` from `firmware-rs/device` (log: `/tmp/claude-1000/-home-dabajabaza-Nutstore-Work-Project-vibebuddy-cardputer/dc6dad58-1d12-401d-9802-6b0d70481ac8/scratchpad/build.log`)

## Versions

- `espup --version`: 0.18.0
- `rustc +esp --version`: 1.97.0-nightly (8ea53bcd7 2026-07-08)
- `xtensa-esp-elf-gcc --version`: 16.2.0 (crosstool-NG esp-16.2.0_20260914)
- Xtensa clang: esp-21.1.3_20260408

## Install paths

- espup binary: `/home/dabajabaza/.cargo/bin/espup`
- espup data dir: `/home/dabajabaza/.espup/` (contains `esp-clang` symlink)
- Rust `esp` toolchain: `/home/dabajabaza/.rustup/toolchains/esp/`
- Xtensa GCC: `/home/dabajabaza/.rustup/toolchains/esp/xtensa-esp-elf/esp-16.2.0_20260914/xtensa-esp-elf/bin`
- Xtensa clang: `/home/dabajabaza/.rustup/toolchains/esp/xtensa-esp32-elf-clang/esp-21.1.3_20260408/esp-clang/lib`
- Export script: `/home/dabajabaza/export-esp.sh`

Espup also installed the RISC-V targets for `stable`.

## Environment file

Every new shell that builds the firmware needs:

```
. ~/export-esp.sh
```

The script sets `LIBCLANG_PATH` and prepends the Xtensa GCC `bin` directory to `PATH`.

## Build result

`cargo build --release` in `firmware-rs/device` finished with exit 0 in about 1m 28s.

- Output binary: `/opt/cargo/target/xtensa-esp32s3-none-elf/release/deps/vibebuddy_firmware-204b28733bd175c6`. The target dir is `/opt/cargo/target`, probably set by the cargo config or a global setting. `CARGO_TARGET_DIR` is not set in this shell.
- One linker warning: `has a LOAD segment with RWX permissions`. It is a warning, not an error, and it comes from the linker. It is not a source error.

## Errors

None.

## Notes

- `espup install` used the default `https_proxy` from the environment.
- A stray empty directory `/tmp/claude-1000/x` was created by mistake and removed.
