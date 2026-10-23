#!/usr/bin/env python3
"""Build the fonts the korekore apps bundle, fonts/korekore/ (docs/design/korekore/README.md, "Fonts").

M PLUS Rounded 1c (weights 400, 700, 800) for what people read and DotGothic16 for the machine-ish bits, both
SIL OFL 1.1, taken from the Google Fonts repository at one commit and pinned by SHA-256, so the output is the same
on every machine. The full files are 2 to 3.6 MB each because of their Japanese glyphs, so they are cut down to:
Latin, punctuation, symbols, kana, and the Chinese characters the box's 12 px table carries
(firmware-rs/core/fonts/cjk-12px.bin: level one of the 2013 general standard table, 3,500 characters) plus any
other character the zh-Hans UI strings use. A character outside that set falls back to the system font.

Subsetting is a modification under the OFL; neither font declares a Reserved Font Name, so the names stay.
M PLUS Rounded 1c ships without a licence file in the Google Fonts repository; its OFL.txt is written from the
copyright notice in the font itself and the licence text DotGothic16 ships.

Downloads are cached in ~/.cache/vibebuddy-fonts. Needs fontTools (`pip install fonttools`).

Usage: tools/korekore-fonts.py
"""
import hashlib
import struct
import sys
import urllib.request
from pathlib import Path

from fontTools import subset
from fontTools.ttLib import TTFont

ROOT = Path(__file__).resolve().parents[1]
OUTPUT = ROOT / "fonts/korekore"
CACHE = Path.home() / ".cache/vibebuddy-fonts"

GOOGLE_FONTS = "https://raw.githubusercontent.com/google/fonts/e7db6ca603fd4862010383ff17ffd6cc4ffac1c0/ofl"
SOURCES = {
    "MPLUSRounded1c-Regular.ttf": ("mplusrounded1c", "b75708b53e45b06d17d470aeeca5b766e3d1b3999f03f13ec4eb863ca846c14c"),
    "MPLUSRounded1c-Bold.ttf": ("mplusrounded1c", "c358630584e8e2d8fbd6121d0f4693255ffef6d1e6d4f3441fd6e5a963a11f9e"),
    "MPLUSRounded1c-ExtraBold.ttf": ("mplusrounded1c", "8e7c15901dca87f1451b356dda594f7d092ba252a5dcc47da74523a242493c36"),
    "DotGothic16-Regular.ttf": ("dotgothic16", "3ad9af88726d42b40f7f365f0dcac785af73cf20ea6f1d5b44e57cc21150b8f1"),
}
DOTGOTHIC_OFL = ("dotgothic16", "OFL.txt", "b6630c61ea078cacd7fabe37d14ffe557a0b45b06683374a9aa9e24262993e33")

CJK_TABLE = ROOT / "firmware-rs/core/fonts/cjk-12px.bin"
# Every character in these is kept, whatever block it is in.
UI_STRINGS = [ROOT / "app/Localization/zh-Hans.lproj/Localizable.strings", ROOT / "desktop/src/i18n.rs"]
RANGES = [
    (0x0020, 0x007E),  # ASCII
    (0x00A0, 0x017F),  # Latin-1 and Latin Extended-A
    (0x2000, 0x206F),  # general punctuation: dashes, quotes, the ellipsis, the bullet
    (0x2190, 0x21FF),  # arrows
    (0x2500, 0x25FF),  # box drawing, blocks and geometric shapes: ▸ ▷ ●
    (0x2600, 0x26FF),  # miscellaneous symbols: ★ ☆
    (0x3000, 0x30FF),  # CJK punctuation, hiragana, katakana
    (0xFF00, 0xFFEF),  # full-width forms
]


def fetch(directory, name, sha256):
    cached = CACHE / name
    if not cached.exists():
        CACHE.mkdir(parents=True, exist_ok=True)
        url = f"{GOOGLE_FONTS}/{directory}/{name}"
        print(f"downloading {url}", file=sys.stderr)
        with urllib.request.urlopen(url) as response:
            cached.write_bytes(response.read())
    data = cached.read_bytes()
    digest = hashlib.sha256(data).hexdigest()
    if digest != sha256:
        sys.exit(f"{cached}: SHA-256 {digest}, expected {sha256}")
    return cached


def characters():
    codepoints = set()
    for first, last in RANGES:
        codepoints.update(range(first, last + 1))
    table = CJK_TABLE.read_bytes()
    (count,) = struct.unpack_from("<H", table)
    codepoints.update(struct.unpack_from(f"<{count}H", table, 2))
    for path in UI_STRINGS:
        codepoints.update(ord(c) for c in path.read_text() if ord(c) > 0x7F)
    return sorted(codepoints)


def write_subset(source, target, codepoints):
    options = subset.Options()
    options.layout_features = ["*"]
    options.name_IDs = ["*"]
    options.name_languages = ["*"]
    options.hinting = False
    font = TTFont(source)
    subsetter = subset.Subsetter(options)
    subsetter.populate(unicodes=codepoints)
    subsetter.subset(font)
    font.save(target)


def licence(copyright_notice, ofl):
    """The OFL text DotGothic16 ships, under another font's copyright line."""
    body = ofl[ofl.index("This Font Software is licensed"):]
    return f"{copyright_notice}\n\n{body}"


def main():
    OUTPUT.mkdir(parents=True, exist_ok=True)
    codepoints = characters()
    for name, (directory, sha256) in SOURCES.items():
        source = fetch(directory, name, sha256)
        write_subset(source, OUTPUT / name, codepoints)
        print(f"{name}: {source.stat().st_size} -> {(OUTPUT / name).stat().st_size} bytes", file=sys.stderr)
    ofl = fetch(*DOTGOTHIC_OFL).read_text()
    (OUTPUT / "OFL-DotGothic16.txt").write_text(ofl)
    rounded = TTFont(CACHE / "MPLUSRounded1c-Regular.ttf")["name"].getDebugName(0)
    (OUTPUT / "OFL-MPLUSRounded1c.txt").write_text(licence(rounded, ofl))


if __name__ == "__main__":
    main()
