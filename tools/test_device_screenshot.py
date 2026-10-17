"""Screen dimensions and PNG output for the standalone serial screenshot tool."""

import importlib.util
from pathlib import Path
import struct
import tempfile
import unittest
import zlib


spec = importlib.util.spec_from_file_location("device_screenshot", Path(__file__).with_name("device-screenshot.py"))
screenshot = importlib.util.module_from_spec(spec)
spec.loader.exec_module(screenshot)


class ScreenshotTests(unittest.TestCase):
    def test_headers_report_each_board_dimensions_and_backlight(self):
        self.assertEqual(screenshot.parse_header("SHOT BEGIN 240x135 BACKLIGHT OFF"), (240, 135, False))
        self.assertEqual(screenshot.parse_header("SHOT BEGIN 320x240 BACKLIGHT ON"), (320, 240, True))
        for size in ["", "axb", "0x240", "240x0", "320x241"]:
            with self.subTest(size=size), self.assertRaises(ValueError):
                screenshot.parse_header("SHOT BEGIN " + size)

    def test_png_dimensions_and_pixels_follow_the_frame_and_scale(self):
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / "screen.png"
            for width, height, scale in [(240, 135, 1), (240, 135, 2), (320, 240, 1)]:
                with self.subTest(width=width, height=height, scale=scale):
                    screenshot.write_png(str(path), [b"\xff\x00\x00"] * (width * height), scale, width, height)
                    png = path.read_bytes()
                    self.assertEqual(png[:8], b"\x89PNG\r\n\x1a\n")
                    self.assertEqual(struct.unpack(">II", png[16:24]), (width * scale, height * scale))
                    offset = 8
                    while png[offset + 4:offset + 8] != b"IDAT":
                        offset += 12 + struct.unpack(">I", png[offset:offset + 4])[0]
                    length = struct.unpack(">I", png[offset:offset + 4])[0]
                    raw = zlib.decompress(png[offset + 8:offset + 8 + length])
                    self.assertEqual(raw, (b"\x00" + b"\xff\x00\x00" * width * scale) * height * scale)
