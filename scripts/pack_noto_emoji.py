#!/usr/bin/env python3
"""Pack single-codepoint Noto Emoji PNGs into atlas textures for egui."""
import argparse
import io
import math
import struct
import zipfile
from dataclasses import dataclass
from pathlib import Path

from PIL import Image

SHEET_WIDTH = 4096
PADDING = 2
PNG_SUBDIR = "png/128"
# Map of feature name -> target height in pixels
RESOLUTIONS = {
    "low": 32,
    "mid": 48,
    "high": 96,
}

@dataclass
class Placement:
    ch: str
    x: int
    y: int
    width: int
    height: int


def iter_single_codepoint_pngs(z: zipfile.ZipFile):
    entries = []
    for name in sorted(z.namelist()):
        if not name.endswith(".png"):
            continue
        if "/" not in name:
            continue
        if f"/{PNG_SUBDIR}/" not in name:
            continue
        stem = name.rsplit("/", 1)[-1]
        if not stem.startswith("emoji_u"):
            continue
        cp_part = stem[len("emoji_u") : -4]
        parts = cp_part.split("_")
        if len(parts) != 1:
            continue  # Skip multi-codepoint sequences for now.
        cp = int(parts[0], 16)
        ch = chr(cp)
        with z.open(name) as f:
            png_bytes = f.read()
        img = Image.open(io.BytesIO(png_bytes)).convert("RGBA")
        entries.append((ch, img))
    return entries


def pack_entries(entries, target_height):
    placements = []
    scaled = []
    x = 0
    y = 0
    row_height = 0
    for ch, img in entries:
        scale = target_height / img.height
        width = max(1, int(round(img.width * scale)))
        resized = img.resize((width, target_height), Image.LANCZOS)
        if x + width > SHEET_WIDTH:
            x = 0
            y += row_height + PADDING
            row_height = 0
        placements.append(Placement(ch, x, y, width, target_height))
        scaled.append(resized)
        x += width + PADDING
        row_height = max(row_height, target_height)
    sheet_height = y + row_height
    atlas = Image.new("RGBA", (SHEET_WIDTH, sheet_height), (0, 0, 0, 0))
    for placement, image in zip(placements, scaled):
        atlas.paste(image, (placement.x, placement.y))
    return atlas, placements


def write_outputs(out_dir: Path, name: str, atlas: Image.Image, placements: list[Placement]):
    png_path = out_dir / f"noto_{name}.png"
    bin_path = out_dir / f"noto_{name}.bin"
    atlas.save(png_path, optimize=True)
    with bin_path.open("wb") as f:
        f.write(struct.pack("<III", atlas.width, atlas.height, len(placements)))
        seen = set()
        for placement in placements:
            codepoint = ord(placement.ch)
            if codepoint in seen:
                continue  # Deduplicate, later entries are duplicates of earlier glyphs.
            seen.add(codepoint)
            f.write(
                struct.pack(
                    "<IHHHH",
                    codepoint,
                    placement.x,
                    placement.y,
                    placement.width,
                    placement.height,
                )
            )
    print(f"Wrote {png_path} and {bin_path} ({len(placements)} placements, {len(seen)} unique glyphs)")


def main(zip_path: Path, out_dir: Path, resolutions: list[str]):
    with zipfile.ZipFile(zip_path) as z:
        entries = iter_single_codepoint_pngs(z)
        print(f"Loaded {len(entries)} single-codepoint PNGs")
        for name in resolutions:
            target_height = RESOLUTIONS[name]
            atlas, placements = pack_entries(entries, target_height)
            write_outputs(out_dir, name, atlas, placements)


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument(
        "--zip",
        type=Path,
        default=Path("noto-emoji-2.051.zip"),
        help="Path to noto-emoji zip",
    )
    parser.add_argument(
        "--out",
        type=Path,
        default=Path("crates/epaint/assets/emoji"),
        help="Directory to write atlas files",
    )
    parser.add_argument(
        "--res",
        nargs="*",
        choices=sorted(RESOLUTIONS.keys()),
        default=list(RESOLUTIONS.keys()),
        help="Resolutions to generate",
    )
    args = parser.parse_args()
    args.out.mkdir(parents=True, exist_ok=True)
    main(args.zip, args.out, args.res)
