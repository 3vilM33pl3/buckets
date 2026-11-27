#!/usr/bin/env python3
"""Build a small demo buckets repository for trying the desktop app.

Needs the `buckets` CLI on PATH (or BUCKETS_CLI), a database the CLI can reach
(DATABASE_URL or `buckets setup`), and Pillow for the textures.

    python3 make_demo_repo.py /tmp/demo   # creates /tmp/demo/canalgame
"""
import json
import math
import os
import random
import subprocess
import sys
from pathlib import Path

from PIL import Image, ImageDraw, ImageFilter

CLI = os.environ.get("BUCKETS_CLI", "buckets")
ENV = dict(os.environ, BUCKETS_NO_EMBEDDINGS="1")


def run(cwd, *args):
    out = subprocess.run([CLI, *args], cwd=cwd, env=ENV, capture_output=True, text=True)
    if out.returncode != 0:
        sys.exit(f"buckets {' '.join(args)} failed:\n{out.stderr}{out.stdout}")
    return out.stdout


def brick(path, hue_shift=0, seed=1, size=512, mortar=6):
    rnd = random.Random(seed)
    img = Image.new("RGB", (size, size), (150, 140, 128))
    d = ImageDraw.Draw(img)
    bw, bh = size // 4, size // 8
    for row in range(8):
        off = (bw // 2) * (row % 2)
        for col in range(-1, 5):
            x0, y0 = col * bw + off, row * bh
            r = 150 + rnd.randint(-25, 25) + hue_shift
            g = 62 + rnd.randint(-12, 12)
            b = 48 + rnd.randint(-10, 10)
            d.rectangle([x0 + mortar // 2, y0 + mortar // 2, x0 + bw - mortar // 2, y0 + bh - mortar // 2], fill=(r, g, b))
    img = img.filter(ImageFilter.GaussianBlur(0.8))
    px = img.load()
    for _ in range(size * 40):
        x, y = rnd.randrange(size), rnd.randrange(size)
        r, g, b = px[x, y]
        n = rnd.randint(-18, 18)
        px[x, y] = (max(0, min(255, r + n)), max(0, min(255, g + n)), max(0, min(255, b + n)))
    path.parent.mkdir(parents=True, exist_ok=True)
    img.save(path)


def normal(path, size=512, strength=1.0):
    img = Image.new("RGB", (size, size))
    px = img.load()
    for y in range(size):
        for x in range(size):
            nx = math.sin(x / size * math.pi * 8) * 0.3 * strength
            ny = math.sin(y / size * math.pi * 16) * 0.3 * strength
            px[x, y] = (int((nx * 0.5 + 0.5) * 255), int((ny * 0.5 + 0.5) * 255), 230)
    img.save(path)


def concept(path, seed):
    rnd = random.Random(seed)
    img = Image.new("RGB", (960, 540), (24, 30, 46))
    d = ImageDraw.Draw(img)
    for i in range(540):
        c = int(24 + i / 540 * 60)
        d.line([(0, i), (960, i)], fill=(c, int(c * 0.9), int(c * 1.2)))
    d.ellipse([700, 60, 790, 150], fill=(236, 228, 200))
    for i in range(14):
        x = i * 70 + rnd.randint(-10, 10)
        h = rnd.randint(140, 300)
        d.rectangle([x, 540 - h, x + 60, 540], fill=(18 + i, 20 + i, 30 + i))
        for wy in range(540 - h + 20, 520, 34):
            if rnd.random() < 0.4:
                d.rectangle([x + 12, wy, x + 22, wy + 14], fill=(240, 190, 90))
    d.rectangle([0, 470, 960, 540], fill=(20, 40, 60))
    path.parent.mkdir(parents=True, exist_ok=True)
    img.save(path)


def main():
    base = Path(sys.argv[1] if len(sys.argv) > 1 else "/tmp/buckets-demo").resolve()
    base.mkdir(parents=True, exist_ok=True)
    repo = base / "canalgame"
    if repo.exists():
        sys.exit(f"{repo} already exists")
    run(base, "init", "canalgame")
    for name in ["concept", "modelling", "texturing", "engine"]:
        run(repo, "create", name)

    c, m, t, e = (repo / n for n in ["concept", "modelling", "texturing", "engine"])
    concept(c / "island_market_dusk.png", 3)
    concept(c / "lock_keeper_cottage.png", 9)
    (c / "brief.md").write_text("# Island Market\n\nDusk, wet cobbles, warm windows.\n")
    run(c, "commit", "Island market mood paintings")

    (m / "props").mkdir()
    (m / "props" / "bollard.blend").write_bytes(os.urandom(64 * 1024))
    (m / "props" / "lock_gate.blend").write_bytes(os.urandom(96 * 1024))
    (m / "props" / "market_stall.hiplc").write_bytes(os.urandom(48 * 1024))
    run(m, "commit", "Blockout props for the lock")

    brick(t / "brick" / "T_Brick_BaseColor.png", seed=4)
    normal(t / "brick" / "T_Brick_Normal.png")
    brick(t / "brick" / "T_Brick_Dark_BaseColor.png", hue_shift=-50, seed=7)
    run(t, "commit", "London stock brick, first pass")
    brick(t / "brick" / "T_Brick_BaseColor.png", hue_shift=18, seed=5, mortar=9)
    run(t, "commit", "Warmer brick, wider mortar")

    (e / "Content" / "Maps").mkdir(parents=True)
    (e / "Content" / "Maps" / "L_IslandMarket.umap").write_bytes(os.urandom(256 * 1024))
    (e / "Content" / "Props").mkdir(parents=True)
    (e / "Content" / "Props" / "SM_Bollard.uasset").write_bytes(os.urandom(80 * 1024))
    (e / "Config").mkdir()
    (e / "Config" / "DefaultGame.ini").write_text("[/Script/EngineSettings.GeneralProjectSettings]\nProjectName=CanalGame\n")
    run(e, "commit", "Import first props")

    # The pipeline: each stage expects work from the one before it.
    run(repo, "expect", "Lock gate and bollard meshes, game ready", "--from", "modelling", "--in-bucket", "engine", "--no-similarity")
    run(repo, "expect", "Brick set tiling at 2 m, with dark variant", "--from", "texturing", "--in-bucket", "engine", "--no-similarity")
    run(repo, "expect", "Market stall carpentry details", "--from", "concept", "--in-bucket", "modelling", "--no-similarity")
    run(repo, "expect", "Moss and grime pass on the brick", "--from", "texturing", "--in-bucket", "modelling", "--no-similarity")
    run(repo, "expect", "Colour script for the dusk sequence", "--in-bucket", "concept", "--no-similarity")

    run(e, "lock", "Content/Maps/L_IslandMarket.umap")
    subprocess.run([CLI, "lock", "Content/Props/SM_Bollard.uasset"], cwd=e, capture_output=True,
                   env=dict(ENV, BUCKETS_USER="maya@studio-pc"))
    run(m, "lock", "props/lock_gate.blend")

    # Uncommitted work so the Changes view has something to show.
    brick(t / "brick" / "T_Brick_BaseColor.png", hue_shift=30, seed=6, mortar=5)
    brick(t / "brick" / "T_Brick_Moss_BaseColor.png", hue_shift=-20, seed=11)
    (m / "props" / "market_stall.hiplc").write_bytes(os.urandom(52 * 1024))
    (c / "brief.md").write_text("# Island Market\n\nDusk, wet cobbles, warm windows.\nAdd lanterns on the stalls.\n")
    print(repo)


if __name__ == "__main__":
    main()
