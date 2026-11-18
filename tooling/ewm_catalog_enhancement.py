"""Append woodland scene.json assets as Enhancement catalog rows.

First-paint membership and the InventoryVerifier four-tuple stay 24.
Woodland runtime scene graph is Q42. This producer still reads scene.json
as a historical assembler dump. Does not enable
streaming.default. Run from the website root.
"""
from __future__ import annotations

import json
import math
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
CATALOG = ROOT / "data/ewm/first-paint.json"
SCENE = ROOT / "production/lush-woodland/scene.json"

CAMPSITE_AABB = ([-30.0, 0.0, 20.0], [-10.0, 8.0, 40.0])
PLACE_ID = "canberra-public-reference"
PAD = 8.0

PREFIX_RANK = [
    "car-passat-b7--",
    "solar-array-",
    "starlink-mini--",
    "canopy--",
    "tidal-vessel--",
    "memory-frond--",
    "footbridge--",
    "observatory--",
    "biology--",
]


def rank(scene_id: str) -> tuple[int, str]:
    for i, prefix in enumerate(PREFIX_RANK):
        if scene_id.startswith(prefix):
            return (i, scene_id)
    if scene_id == "prototype-eucalyptus-lush-near":
        return (100, scene_id)
    return (50, scene_id)


def tiles_for(mn: list[float], mx: list[float]) -> list[list[int]]:
    ix0 = math.floor(mn[0] / 100.0)
    iz0 = math.floor(mn[2] / 100.0)
    ix1 = math.floor(mx[0] / 100.0)
    iz1 = math.floor(mx[2] / 100.0)
    out = []
    for ix in range(int(ix0), int(ix1) + 1):
        for iz in range(int(iz0), int(iz1) + 1):
            out.append([ix, iz])
    return out


def aabb_of(asset: dict) -> tuple[list[float], list[float]]:
    places = asset.get("placements") or []
    if not places:
        return ([v for v in CAMPSITE_AABB[0]], [v for v in CAMPSITE_AABB[1]])
    xs = [p["translation"][0] for p in places]
    ys = [p["translation"][1] for p in places]
    zs = [p["translation"][2] for p in places]
    mn = [min(xs) - PAD, min(0.0, min(ys) - PAD), min(zs) - PAD]
    mx = [max(xs) + PAD, max(ys) + PAD + 4.0, max(zs) + PAD]
    return mn, mx


def main() -> None:
    catalog = json.loads(CATALOG.read_text(encoding="utf-8"))
    scene = json.loads(SCENE.read_text(encoding="utf-8"))
    first_paint = list(catalog["first_paint"])
    present = {row["scene_id"] for row in catalog["entries"]}
    next_id = max(row["entry_id"] for row in catalog["entries"]) + 1
    remaining = [a for a in scene["assets"] if a["id"] not in first_paint]
    remaining.sort(key=lambda a: rank(a["id"]))
    added = []
    for asset in remaining:
        if asset["id"] in present:
            continue
        mn, mx = aabb_of(asset)
        row = {
            "entry_id": next_id,
            "scene_id": asset["id"],
            "place_id": PLACE_ID,
            "epistemic_class": "authored-artwork",
            "family": "TenD",
            "family_from": "inner-magic",
            "role": "Enhancement",
            "obligation": "Optional",
            "sha256": asset["sha256"],
            "bytes": asset["bytes"],
            "vertices": asset.get("vertices", 0),
            "triangles": asset.get("triangles", 0),
            "url": asset["url"],
            "shard": "enhancement",
            "aabb": {"min": mn, "max": mx},
            "tiles": tiles_for(mn, mx),
            "planner_region": None,
        }
        catalog["entries"].append(row)
        added.append(asset)
        present.add(asset["id"])
        next_id += 1
    catalog["enhancement_not_first_paint"] = [
        {
            "id": a["id"],
            "bytes": a["bytes"],
            "epistemic_class": "authored-artwork",
            "role": "Enhancement after first paint",
        }
        for a in added
    ]
    CATALOG.write_text(
        json.dumps(catalog, indent=2, ensure_ascii=False) + "\n",
        encoding="utf-8",
    )
    print(
        json.dumps(
            {
                "first_paint": len(first_paint),
                "enhancement": len(added),
                "catalog_entries": len(catalog["entries"]),
                "enhancement_bytes": sum(a["bytes"] for a in added),
                "candidate_entries": catalog["candidate"]["entries"],
                "streaming_default": catalog["streaming"]["default"],
            },
            indent=2,
        )
    )


if __name__ == "__main__":
    main()
