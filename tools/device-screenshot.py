#!/usr/bin/env python3
"""Screenshot the device: send `device.screenshot`, receive the run-length-encoded framebuffer, write a PNG.

Needs exclusive use of the serial port, so stop vibebuddyd first (see tools/screenshot.sh). Opening the port
leaves DTR/RTS alone, so the device doesn't reset and the screen stays as it is.
Usage: device-screenshot.py PORT OUTPUT.png [scale]
"""

from __future__ import annotations

import struct
import sys
import time
import zlib

MAX_PIXELS = 320 * 240


def parse_header(line: str) -> tuple[int, int, bool]:
    try:
        width, height = map(int, line.removeprefix("SHOT BEGIN").split()[0].split("x"))
        if width <= 0 or height <= 0 or width * height > MAX_PIXELS:
            raise ValueError
    except (ValueError, IndexError) as error:
        raise ValueError(f"invalid screenshot dimensions: {line}") from error
    return width, height, "BACKLIGHT OFF" not in line


def rgb565_to_rgb(color: int) -> bytes:
    red = (color >> 11) & 0x1F
    green = (color >> 5) & 0x3F
    blue = color & 0x1F
    return bytes((red * 255 // 31, green * 255 // 63, blue * 255 // 31))


def write_png(path: str, pixels: list[bytes], scale: int, width: int, height: int) -> None:
    def chunk(kind: bytes, payload: bytes) -> bytes:
        body = kind + payload
        return struct.pack(">I", len(payload)) + body + struct.pack(">I", zlib.crc32(body) & 0xFFFFFFFF)

    raw = bytearray()
    for y in range(height):
        row = b"".join(pixels[y * width + x] * scale for x in range(width))
        for _ in range(scale):
            raw += b"\x00" + row
    header = struct.pack(">IIBBBBB", width * scale, height * scale, 8, 2, 0, 0, 0)
    with open(path, "wb") as output:
        output.write(b"\x89PNG\r\n\x1a\n")
        output.write(chunk(b"IHDR", header))
        output.write(chunk(b"IDAT", zlib.compress(bytes(raw), 6)))
        output.write(chunk(b"IEND", b""))


def main(argv: list[str]) -> int:
    import serial

    if len(argv) not in (3, 4):
        print(__doc__, file=sys.stderr)
        return 2
    port_name, output_path = argv[1], argv[2]
    scale = int(argv[3]) if len(argv) == 4 else 2
    port = serial.Serial(port_name, 115200, timeout=0.5)
    port.reset_input_buffer()
    port.write(b'{"version":1,"event":"device.screenshot"}\n')

    pixels: list[bytes] = []
    width, height = 0, 0
    backlight_on = True
    deadline = time.time() + 60
    started = False
    while time.time() < deadline:
        line = port.readline().decode("utf-8", "replace").strip()
        if not line:
            continue
        if line.startswith("SHOT BEGIN"):
            width, height, backlight_on = parse_header(line)
            started = True
            pixels.clear()
            continue
        if line == "SHOT END":
            break
        if not started or not line.startswith("SHOT "):
            continue
        for run in line[5:].split():
            color, count = run.split(":")
            length = int(count)
            if length < 0 or length > width * height - len(pixels):
                raise ValueError("more pixels than one screen")
            pixels.extend([rgb565_to_rgb(int(color, 16))] * length)
    port.close()

    if not started or len(pixels) != width * height:
        print(f"incomplete frame: got {len(pixels)} pixels, expected {width * height}", file=sys.stderr)
        return 1
    if not backlight_on:
        # With the backlight off, what a person sees is a black screen; dim the image until it is barely visible.
        pixels = [bytes(channel // 8 for channel in pixel) for pixel in pixels]
    write_png(output_path, pixels, scale, width, height)
    print(f"{output_path} backlight {'on' if backlight_on else 'off'}")
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv))
