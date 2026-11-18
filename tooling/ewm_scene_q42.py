"""Compile the lush-woodland scene graph to Q42VOL v4 CBOR-LD SuperBlocks.

Turtle is the producer RDF. Visitors never read the Q42 file. UnifiedVolumeBuilder
writes CBOR-LD SuperBlocks of NQuins. Water geometry is an exact binary sidecar
(WTR1). scene.json is a producer-only historical assembler dump.

Literal terms are RDF-typed (IRI, number, boolean, text). Vectors are numeric
components, not JSON arrays. streaming.default stays false. The InventoryVerifier
four-tuple stays 24.
"""
from __future__ import annotations

import hashlib
import json
import re
import struct
import subprocess
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
SCENE_JSON = ROOT / "production/lush-woodland/scene.json"
CATALOG = ROOT / "data/ewm/first-paint.json"
OUT_DIR = ROOT / "production/lush-woodland"
HELPER = ROOT / "adapters/qualiadb/q42_web.exe"
BASE = "https://webizen.au/"
REL = BASE + "relation/"
SCENE_NS = BASE + "scene/"
ASSET_NS = SCENE_NS + "asset/"
PLACE_NS = SCENE_NS + "placement/"
TEX_NS = SCENE_NS + "texture/"
WORLD = SCENE_NS + "lush-woodland"
NUMBER = re.compile(r"^-?(?:0|[1-9]\d*)(?:\.\d+)?(?:[eE][+-]?\d+)?$")


def sha256(path: Path) -> str:
    return hashlib.sha256(path.read_bytes()).hexdigest()


def lit_int(value) -> str:
    return str(int(value))


def lit_float(value) -> str:
    n = float(value)
    if n != n or n in (float("inf"), float("-inf")):
        raise SystemExit("Non-finite scene number")
    return json.dumps(n, ensure_ascii=False)


def write_water(path: Path, positions: list, indices: list) -> None:
    buf = bytearray()
    buf += b"WTR1"
    buf += struct.pack("<I", len(positions))
    buf += struct.pack("<" + "f" * len(positions), *[float(v) for v in positions])
    buf += struct.pack("<I", len(indices))
    buf += struct.pack("<" + "I" * len(indices), *[int(v) for v in indices])
    path.write_bytes(buf)


def ttl_term(term: str) -> str:
    if term.startswith("http://") or term.startswith("https://"):
        return "<" + term + ">"
    if term in ("true", "false", "null") or NUMBER.match(term):
        return term
    return json.dumps(term, ensure_ascii=False)


def vec_components(prefix: str, values: list, axes: str) -> list[tuple[str, str]]:
    if len(values) != len(axes):
        raise SystemExit(f"{prefix} needs {len(axes)} components")
    return [(prefix + axis, lit_float(values[i])) for i, axis in enumerate(axes)]


def main() -> int:
    scene = json.loads(SCENE_JSON.read_text(encoding="utf-8"))
    catalog = json.loads(CATALOG.read_text(encoding="utf-8"))
    if scene.get("schema") != "webizen.native-world/v1":
        raise SystemExit("Unsupported woodland scene schema")
    water = scene.get("water") or {}
    positions = water.get("positions") or []
    indices = water.get("indices") or []
    if not positions or not indices:
        raise SystemExit("Woodland scene is missing water geometry")

    water_path = OUT_DIR / "water.bin"
    write_water(water_path, positions, indices)
    water_sha = sha256(water_path)

    triples: list[list[str]] = []

    def fact(s: str, p: str, o: str) -> None:
        triples.append([s, REL + p, o])

    fact(WORLD, "kind", SCENE_NS + "world")
    fact(WORLD, "schema", "webizen.native-world/v1")
    fact(WORLD, "units", "metres")
    fact(WORLD, "up", "+Y")
    fact(WORLD, "waterGeometry", "/production/lush-woodland/water.bin")
    fact(WORLD, "waterSha256", water_sha)
    fact(WORLD, "hasLogicVolume", "/data/ewm/camp.q42")
    if scene.get("triangle_count") is not None:
        fact(WORLD, "triangleCount", lit_int(scene["triangle_count"]))
    if scene.get("vertex_count") is not None:
        fact(WORLD, "vertexCount", lit_int(scene["vertex_count"]))

    assets = list(scene["assets"])
    seen = {a["id"] for a in assets}
    camp_places = {}
    camp_mod = ROOT / "tooling" / "ewm_camp_q42.py"
    if camp_mod.is_file():
        import importlib.util

        spec = importlib.util.spec_from_file_location("ewm_camp_q42", camp_mod)
        camp = importlib.util.module_from_spec(spec)
        spec.loader.exec_module(camp)
        camp_places = camp.authored_placements()
    for row in catalog.get("entries") or []:
        sid = row.get("scene_id")
        if not sid or sid in seen:
            continue
        place = camp_places.get(sid)
        assets.append(
            {
                "id": sid,
                "url": row["url"],
                "sha256": row["sha256"],
                "colour": [1, 1, 1, 1],
                "placements": [place] if place else [],
            }
        )
        seen.add(sid)

    for index, asset in enumerate(assets):
        uri = ASSET_NS + asset["id"]
        fact(WORLD, "hasAsset", uri)
        fact(uri, "kind", SCENE_NS + "asset")
        fact(uri, "index", lit_int(index))
        fact(uri, "id", asset["id"])
        fact(uri, "url", asset["url"])
        fact(uri, "sha256", asset["sha256"])
        colour = list(asset.get("colour") or [1, 1, 1, 1])
        while len(colour) < 4:
            colour.append(1)
        for pred, value in vec_components("colour", colour[:4], "RGBA"):
            fact(uri, pred, value)
        for p_index, placement in enumerate(asset.get("placements") or []):
            p_uri = PLACE_NS + placement["id"]
            fact(uri, "hasPlacement", p_uri)
            fact(p_uri, "kind", SCENE_NS + "placement")
            fact(p_uri, "index", lit_int(p_index))
            fact(p_uri, "id", placement["id"])
            for pred, value in vec_components("translation", placement["translation"], "XYZ"):
                fact(p_uri, pred, value)
            for pred, value in vec_components("rotation", placement["rotation"], "XYZW"):
                fact(p_uri, pred, value)
            for pred, value in vec_components("scale", placement["scale"], "XYZ"):
                fact(p_uri, pred, value)

    for index, texture in enumerate(scene.get("textures") or []):
        uri = TEX_NS + f"{texture['sha256']}-{texture['role']}"
        fact(WORLD, "hasTexture", uri)
        fact(uri, "kind", SCENE_NS + "texture")
        fact(uri, "index", lit_int(index))
        fact(uri, "url", texture["url"])
        fact(uri, "sha256", texture["sha256"])
        fact(uri, "mimeType", texture["mime_type"])
        fact(uri, "role", lit_int(texture["role"]))
        fact(uri, "cutoff", lit_float(texture.get("cutoff", 0.5)))

    triples = sorted({tuple(row) for row in triples})
    triples = [list(row) for row in triples]
    triple_path = OUT_DIR / "scene-triples.json"
    triple_path.write_text(
        json.dumps(triples, ensure_ascii=False, indent=2) + "\n", encoding="utf-8"
    )
    ttl = "\n".join(" ".join(ttl_term(part) for part in row) + " ." for row in triples) + "\n"
    (OUT_DIR / "scene.ttl").write_text(ttl, encoding="utf-8")

    digest = hashlib.sha256(
        json.dumps(triples, ensure_ascii=False, separators=(",", ":")).encode("utf-8")
    ).hexdigest()
    named = OUT_DIR / f"scene-{digest[:16]}.q42"
    served = OUT_DIR / "scene.q42"
    if not named.exists():
        if not HELPER.exists():
            raise SystemExit(f"Missing Q42 helper {HELPER}")
        run = subprocess.run(
            [str(HELPER), "build", str(triple_path), str(named)],
            capture_output=True,
            text=True,
            encoding="utf-8",
            timeout=90,
        )
        if run.returncode:
            raise SystemExit(run.stderr.strip() or run.stdout.strip() or "q42_web build failed")
        print(run.stdout.strip())
    served.write_bytes(named.read_bytes())
    inspect = subprocess.run(
        [str(HELPER), "inspect", str(served)],
        capture_output=True,
        text=True,
        encoding="utf-8",
        timeout=90,
    )
    if inspect.returncode:
        raise SystemExit(inspect.stderr.strip() or inspect.stdout.strip() or "q42_web inspect failed")
    loaded = json.loads(inspect.stdout)
    if loaded["total_quins"] != len(triples):
        raise SystemExit("Q42 quin count differs from authored scene triples")
    print(
        json.dumps(
            {
                "q42": str(served.relative_to(ROOT)).replace("\\", "/"),
                "sha256": sha256(served),
                "bytes": served.stat().st_size,
                "quins": loaded["total_quins"],
                "assets": len(assets),
                "placements": sum(len(a.get("placements") or []) for a in assets),
                "textures": len(scene.get("textures") or []),
                "water_sha256": water_sha,
                "water_bytes": water_path.stat().st_size,
                "ttl": "production/lush-woodland/scene.ttl",
                "encoding": "q42vol-v4-cbor-ld",
                "streaming_default": False,
            },
            indent=2,
        )
    )
    return 0


if __name__ == "__main__":
    sys.exit(main())
