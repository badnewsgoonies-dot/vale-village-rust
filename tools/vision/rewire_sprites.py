#!/usr/bin/env python3
"""Rewire enemy_sprite_mappings() in sprites.rs to point to generated sprites.

Reads the sprite manifest, checks which PNGs exist in generated/,
and updates the mapping table in src/plugins/sprites.rs.

Usage:
    python3 tools/vision/rewire_sprites.py --dry-run  # preview changes
    python3 tools/vision/rewire_sprites.py              # apply changes
"""

import json
import os
import re
import sys

MANIFEST = "status/sprite-manifest.json"
SPRITES_RS = "src/plugins/sprites.rs"
GENERATED_DIR = "assets/sprites/battle/enemies/generated"
DJINN_DIR = "assets/sprites/battle/djinn/generated"


def main():
    dry_run = "--dry-run" in sys.argv

    with open(MANIFEST) as f:
        manifest = json.load(f)

    # Build mapping: enemy_id -> generated sprite path (relative to assets/)
    rewire_map = {}
    for entry in manifest["enemies"]:
        filename = entry["id"].replace("-", "_") + ".png"
        full_path = os.path.join(GENERATED_DIR, filename)
        if os.path.exists(full_path):
            # Bevy paths are relative to assets/
            bevy_path = full_path.replace("assets/", "")
            rewire_map[entry["id"]] = bevy_path

    print(f"Generated sprites found: {len(rewire_map)}/{len(manifest['enemies'])}")

    if not rewire_map:
        print("No sprites to rewire.")
        return

    # Read sprites.rs
    with open(SPRITES_RS) as f:
        content = f.read()

    # Find and replace each mapping line
    changes = 0
    for enemy_id, new_path in rewire_map.items():
        # Match patterns like: ("mercury-slime", "sprites/placeholders/slime.png"),
        pattern = rf'(\(\s*"{re.escape(enemy_id)}"\s*,\s*)"[^"]*"(\s*\))'
        replacement = rf'\1"{new_path}"\2'
        new_content = re.sub(pattern, replacement, content)
        if new_content != content:
            changes += 1
            if dry_run:
                print(f"  {enemy_id} -> {new_path}")
            content = new_content

    if dry_run:
        print(f"\n{changes} mappings would be updated (dry run)")
    else:
        with open(SPRITES_RS, "w") as f:
            f.write(content)
        print(f"{changes} mappings updated in {SPRITES_RS}")

    # Report missing
    missing = [e["name"] for e in manifest["enemies"]
               if e["id"] not in rewire_map]
    if missing:
        print(f"\nMissing sprites ({len(missing)}):")
        for name in missing[:10]:
            print(f"  - {name}")
        if len(missing) > 10:
            print(f"  ... and {len(missing) - 10} more")


if __name__ == "__main__":
    main()
