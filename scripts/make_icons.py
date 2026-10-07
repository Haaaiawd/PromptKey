#!/usr/bin/env python3
"""Generate all bundle icon assets from the brand master.

Inputs : PromptKey_aiextract.png (1024x1024 RGBA master), PromptKey.ico
Outputs: icons/icon.icns, icons/32x32.png, icons/128x128.png,
         icons/128x128@2x.png, icons/icon.png,
         src/icons/brand/tray-template.png  (macOS template image)

.icns is written directly (PNG-payload entries) so the script runs on any
OS — on macOS, `iconutil -c icns` on the emitted iconset is the canonical
alternative; both produce the same bytes-layout. See docs/PLATFORMS.md.
"""
import struct
import sys
from pathlib import Path

from PIL import Image

ROOT = Path(__file__).resolve().parent.parent
MASTER = ROOT / "PromptKey_aiextract.png"
OUT = ROOT / "icons"
TRAY = ROOT / "src" / "icons" / "brand" / "tray-template.png"

# icns OSType -> pixel size of the PNG payload
ICNS_SIZES = {
    b"ic07": 128,   # 128x128
    b"ic08": 256,   # 256x256
    b"ic09": 512,   # 512x512
    b"ic10": 1024,  # 512x512@2x
    b"ic11": 32,    # 16x16@2x
    b"ic12": 64,    # 32x32@2x
    b"ic13": 256,   # 128x128@2x
    b"ic14": 512,   # 256x256@2x
}


def png_bytes(img: Image.Image, size: int) -> bytes:
    import io

    buf = io.BytesIO()
    img.resize((size, size), Image.LANCZOS).save(buf, "PNG")
    return buf.getvalue()


def write_icns(master: Image.Image, dest: Path) -> None:
    entries = []
    for ostype, size in ICNS_SIZES.items():
        payload = png_bytes(master, size)
        entries.append(ostype + struct.pack(">I", 8 + len(payload)) + payload)
    body = b"".join(entries)
    dest.write_bytes(b"icns" + struct.pack(">I", 8 + len(body)) + body)


def write_tray_template(master: Image.Image, dest: Path, size: int = 44) -> None:
    """macOS menu-bar template icon: black silhouette, alpha = shape mask."""
    rgba = master.resize((size, size), Image.LANCZOS).convert("RGBA")
    alpha = rgba.getchannel("A")
    black = Image.new("RGBA", (size, size), (0, 0, 0, 255))
    black.putalpha(alpha)
    dest.parent.mkdir(parents=True, exist_ok=True)
    black.save(dest, "PNG")


def main() -> int:
    if not MASTER.exists():
        print(f"missing master: {MASTER}", file=sys.stderr)
        return 1
    master = Image.open(MASTER).convert("RGBA")
    OUT.mkdir(exist_ok=True)

    write_icns(master, OUT / "icon.icns")
    for name, size in [
        ("32x32.png", 32),
        ("128x128.png", 128),
        ("128x128@2x.png", 256),
        ("icon.png", 512),
    ]:
        master.resize((size, size), Image.LANCZOS).save(OUT / name, "PNG")
    write_tray_template(master, TRAY)

    for p in sorted(OUT.iterdir()):
        print(f"wrote {p.relative_to(ROOT)} ({p.stat().st_size} bytes)")
    print(f"wrote {TRAY.relative_to(ROOT)} ({TRAY.stat().st_size} bytes)")
    return 0


if __name__ == "__main__":
    sys.exit(main())
