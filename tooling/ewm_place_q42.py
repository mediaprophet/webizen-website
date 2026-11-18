"""Compile per-place static scene volumes to Q42VOL v4 CBOR-LD SuperBlocks.

N3 is the producer RDF. Visitors never read N3. UnifiedVolumeBuilder writes
CBOR-LD SuperBlocks of NQuins from triples.json. Each PublicPlace lists a
hasAsset subset of existing woodland identities (URLs already under
/production/ or /packages/by-sha256/). Bytes of .10d organs are not copied.
Field-studio may point at /data/ewm/camp.q42; other places omit camp logic.

Static files under data/ewm/places/ are HTTP Range whole-entry (16 MiB cap).
No streaming server. inner_ranges stays false. streaming.default stays false.
Four-tuple first-paint stays 24. Timeline.json is never copied.
"""
from __future__ import annotations

import hashlib
import json
import re
import subprocess
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
CATALOG_JSON = ROOT / "data/ewm/first-paint.json"
OUT_DIR = ROOT / "data/ewm/places"
HELPER = ROOT / "adapters/qualiadb/q42_web.exe"
BASE = "https://webizen.au/"
REL = BASE + "relation/"
CAT = BASE + "catalog/"
ASSET_NS = BASE + "scene/asset/"
SCENE_NS = BASE + "scene/"
PLACE_KIND = CAT + "PublicPlace"
ASSET_KIND = SCENE_NS + "asset"
VOLUME_CAP = 16 * 1024 * 1024
ALLOWED_URL_PREFIXES = ("/production/", "/packages/by-sha256/")
NUMBER = re.compile(r"^-?(?:0|[1-9]\d*)(?:\.\d+)?(?:[eE][+-]?\d+)?$")
FORBIDDEN = ("Timeline.json", "INFERRED_WORK", "placeIds")

# Public exhibit origins are woodland-frame metres from world.json, not GPS.
PLACES = (
    {
        "id": "field-studio",
        "title": "The field studio",
        "exhibit_kind": "campsite",
        "origin": [-22.0, 2.0, 28.0],
        "logic_volume": "/data/ewm/camp.q42",
        "include": (
            "canberra-reference-terrain",
            "habitat-terrain",
            "prototype-eucalyptus-lush-mid",
            "car-passat-b7",
            "vintage-windup-camper",
            "solar-array",
            "starlink-mini",
            "field-studio-operator",
        ),
        "base": "first_paint",
    },
    {
        "id": "river",
        "title": "The living river",
        "exhibit_kind": "terrain",
        "origin": [0.0, 2.0, 16.0],
        "logic_volume": None,
        "include": (
            "canberra-reference-terrain",
            "habitat-terrain",
            "tidal-vessel",
            "prototype-bank-rock",
            "prototype-grass",
        ),
        "base": (
            "canberra-reference-terrain",
            "habitat-terrain",
            "tidal-vessel",
            "prototype-bank-rock",
            "prototype-grass",
        ),
    },
    {
        "id": "canopy",
        "title": "The forest canopy",
        "exhibit_kind": "relationships",
        "origin": [-20.0, 6.0, -16.0],
        "logic_volume": None,
        "include": (
            "canberra-reference-terrain",
            "habitat-terrain",
            "canopy--",
            "memory-frond",
            "prototype-eucalyptus",
            "prototype-shrub",
            "prototype-leaf-litter",
        ),
        "base": (
            "canberra-reference-terrain",
            "habitat-terrain",
            "canopy--",
        ),
    },
    {
        "id": "commons",
        "title": "The commons",
        "exhibit_kind": "rights",
        "origin": [-17.0, 5.0, 6.0],
        "logic_volume": None,
        "include": (
            "canberra-reference-terrain",
            "habitat-terrain",
            "footbridge",
        ),
        "base": (
            "canberra-reference-terrain",
            "habitat-terrain",
            "footbridge",
        ),
    },
)


def sha256(path: Path) -> str:
    return hashlib.sha256(path.read_bytes()).hexdigest()


def lit_int(value) -> str:
    return str(int(value))


def lit_float(value) -> str:
    n = float(value)
    if n != n or n in (float("inf"), float("-inf")):
        raise SystemExit("Non-finite place number")
    return json.dumps(n, ensure_ascii=False)


def lit_bool(value) -> str:
    return "true" if value else "false"


def id_matches(scene_id: str, prefixes: tuple[str, ...] | list[str]) -> bool:
    return any(scene_id == prefix or scene_id.startswith(prefix) for prefix in prefixes)


def n3_term(term: str) -> str:
    prefixes = (
        (CAT + "place/", "place:"),
        (ASSET_NS, "asset:"),
        (SCENE_NS, "scene:"),
        (CAT, "cat:"),
        (REL, "rel:"),
        (BASE, "webizen:"),
    )
    if term.startswith("http://") or term.startswith("https://"):
        for iri, prefix in prefixes:
            if term.startswith(iri):
                rest = term[len(iri) :]
                if rest and all(ch.isalnum() or ch in "-_" for ch in rest):
                    return prefix + rest
        return "<" + term + ">"
    if term in ("true", "false", "null") or NUMBER.match(term):
        return term
    return json.dumps(term, ensure_ascii=False)


def n3_dump(triples: list[list[str]]) -> str:
    lines = [
        "@prefix rel: <https://webizen.au/relation/> .",
        "@prefix cat: <https://webizen.au/catalog/> .",
        "@prefix place: <https://webizen.au/catalog/place/> .",
        "@prefix asset: <https://webizen.au/scene/asset/> .",
        "@prefix scene: <https://webizen.au/scene/> .",
        "@prefix webizen: <https://webizen.au/> .",
        "",
    ]
    for row in triples:
        lines.append(" ".join(n3_term(part) for part in row) + " .")
    return "\n".join(lines) + "\n"


def refuse_forbidden(text: str, label: str) -> None:
    for token in FORBIDDEN:
        if token in text:
            raise SystemExit(f"{label} contains forbidden token {token}")


def select_entries(entries: list[dict], prefixes: tuple[str, ...], extra_ids: set[str]) -> list[dict]:
    selected = []
    seen: set[str] = set()
    for row in entries:
        scene_id = row.get("scene_id")
        if not scene_id or scene_id in seen:
            continue
        if scene_id in extra_ids or id_matches(scene_id, prefixes):
            url = row.get("url") or ""
            if not url.startswith(ALLOWED_URL_PREFIXES):
                raise SystemExit(f"Place asset URL is not a reused woodland identity: {url}")
            selected.append(row)
            seen.add(scene_id)
    return selected


def compile_volume(triple_path: Path, named: Path, served: Path) -> dict:
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
    if served.stat().st_size > VOLUME_CAP:
        raise SystemExit(f"{served.name} exceeds 16 MiB whole-volume cap")
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
    return loaded


def build_place(spec: dict, catalog: dict) -> dict:
    entries = list(catalog.get("entries") or [])
    first_paint = list(catalog.get("first_paint") or [])
    extra_ids: set[str] = set()
    if spec["base"] == "first_paint":
        extra_ids.update(first_paint)
    selected = select_entries(entries, spec["include"], extra_ids)
    if spec["id"] == "field-studio":
        selected_ids = {row["scene_id"] for row in selected}
        missing = [sid for sid in first_paint if sid not in selected_ids]
        if missing:
            raise SystemExit(f"field-studio omitted first-paint identities: {missing}")
    else:
        if any(id_matches(row["scene_id"], ("vintage-windup-camper",)) for row in selected):
            raise SystemExit(f"{spec['id']} must omit vintage-windup-camper organs")

    if spec["base"] == "first_paint":
        base_ids = set(first_paint)
    else:
        base_ids = {row["scene_id"] for row in selected if id_matches(row["scene_id"], spec["base"])}

    place_uri = CAT + "place/" + spec["id"]
    volume_url = f"/data/ewm/places/{spec['id']}.q42"
    triples: list[list[str]] = []

    def fact(s: str, p: str, o: str) -> None:
        triples.append([s, REL + p, o])

    fact(place_uri, "kind", PLACE_KIND)
    fact(place_uri, "schema", "webizen.ewm-place-scene/v1")
    fact(place_uri, "id", spec["id"])
    fact(place_uri, "title", spec["title"])
    fact(place_uri, "exhibitKind", spec["exhibit_kind"])
    fact(place_uri, "notPersonalCampsite", lit_bool(True))
    fact(place_uri, "innerRanges", lit_bool(False))
    fact(place_uri, "streamingDefault", lit_bool(False))
    fact(place_uri, "hasSceneVolume", volume_url)
    fact(place_uri, "originX", lit_float(spec["origin"][0]))
    fact(place_uri, "originY", lit_float(spec["origin"][1]))
    fact(place_uri, "originZ", lit_float(spec["origin"][2]))
    if spec.get("logic_volume"):
        fact(place_uri, "hasLogicVolume", spec["logic_volume"])

    for index, row in enumerate(selected):
        scene_id = row["scene_id"]
        uri = ASSET_NS + scene_id
        fact(place_uri, "hasAsset", uri)
        if scene_id in base_ids:
            fact(place_uri, "hasBaseAsset", uri)
        fact(uri, "kind", ASSET_KIND)
        fact(uri, "index", lit_int(index))
        fact(uri, "id", scene_id)
        fact(uri, "url", row["url"])
        fact(uri, "sha256", row["sha256"])
        fact(uri, "bytes", lit_int(row.get("bytes") or 0))
        role = "Base" if scene_id in base_ids else "Enhancement"
        fact(uri, "role", role)
        fact(uri, "forPlace", place_uri)

    triples = sorted({tuple(row) for row in triples})
    triples = [list(row) for row in triples]
    n3 = n3_dump(triples)
    refuse_forbidden(n3, spec["id"] + ".n3")
    refuse_forbidden(json.dumps(triples), spec["id"] + " triples")

    OUT_DIR.mkdir(parents=True, exist_ok=True)
    n3_path = OUT_DIR / f"{spec['id']}.n3"
    n3_path.write_text(n3, encoding="utf-8")
    triple_path = OUT_DIR / f"{spec['id']}-triples.json"
    triple_path.write_text(
        json.dumps(triples, ensure_ascii=False, indent=2) + "\n", encoding="utf-8"
    )
    digest = hashlib.sha256(
        json.dumps(triples, ensure_ascii=False, separators=(",", ":")).encode("utf-8")
    ).hexdigest()
    named = OUT_DIR / f"{spec['id']}-{digest[:16]}.q42"
    served = OUT_DIR / f"{spec['id']}.q42"
    loaded = compile_volume(triple_path, named, served)
    if loaded["version"] != 4:
        raise SystemExit(f"{spec['id']} is not Q42VOL v4")
    if loaded["total_quins"] != len(triples):
        raise SystemExit(f"{spec['id']} quin count differs from authored triples")
    if served.read_bytes()[:3] != b"Q42":
        raise SystemExit(f"{spec['id']}.q42 is not a Q42 volume")
    kinds = {row[2] for row in loaded["rows"] if str(row[1]).endswith("/kind")}
    if PLACE_KIND not in kinds:
        raise SystemExit(f"{spec['id']} is missing PublicPlace")
    logic_rows = [row for row in loaded["rows"] if str(row[1]).endswith("/hasLogicVolume")]
    if spec.get("logic_volume"):
        if not any(spec["logic_volume"] in str(row[2]) for row in logic_rows):
            raise SystemExit(f"{spec['id']} missing hasLogicVolume {spec['logic_volume']}")
    elif logic_rows:
        raise SystemExit(f"{spec['id']} must omit camp logic")
    refuse_forbidden(served.read_bytes().decode("latin-1", errors="ignore"), spec["id"] + ".q42")
    return {
        "id": spec["id"],
        "q42": str(served.relative_to(ROOT)).replace("\\", "/"),
        "n3": str(n3_path.relative_to(ROOT)).replace("\\", "/"),
        "sha256": sha256(served),
        "bytes": served.stat().st_size,
        "quins": loaded["total_quins"],
        "assets": [row["scene_id"] for row in selected],
        "base": sorted(base_ids),
        "hasLogicVolume": spec.get("logic_volume"),
        "hasSceneVolume": volume_url,
        "encoding": "q42vol-v4-cbor-ld",
        "streaming_default": False,
        "inner_ranges": False,
    }


def main() -> int:
    catalog = json.loads(CATALOG_JSON.read_text(encoding="utf-8"))
    if catalog.get("schema") != "webizen.ewm-catalog/v1":
        raise SystemExit("Unsupported EWM catalog schema")
    if catalog.get("streaming", {}).get("default") is True:
        raise SystemExit("Producer refused streaming.default true")
    if catalog.get("inner_ranges") is True:
        raise SystemExit("Producer refused inner_ranges true")
    first_paint = catalog.get("first_paint") or []
    if len(first_paint) != 24:
        raise SystemExit("Four-tuple first-paint must stay 24")

    OUT_DIR.mkdir(parents=True, exist_ok=True)
    receipt = []
    for spec in PLACES:
        receipt.append(build_place(spec, catalog))

    river_assets = next(row["assets"] for row in receipt if row["id"] == "river")
    studio_assets = next(row["assets"] for row in receipt if row["id"] == "field-studio")
    if not any(id_matches(sid, ("vintage-windup-camper",)) for sid in studio_assets):
        raise SystemExit("field-studio must keep vintage-windup-camper identities")
    if any(id_matches(sid, ("vintage-windup-camper",)) for sid in river_assets):
        raise SystemExit("river must omit vintage-windup-camper identities")
    studio_logic = next(row["hasLogicVolume"] for row in receipt if row["id"] == "field-studio")
    if studio_logic != "/data/ewm/camp.q42":
        raise SystemExit("field-studio must point at /data/ewm/camp.q42")

    payload = {
        "schema": "webizen.ewm-place-scene/v1",
        "encoding": "q42vol-v4-cbor-ld",
        "streaming_default": False,
        "inner_ranges": False,
        "volume_cap_bytes": VOLUME_CAP,
        "first_paint": 24,
        "places": receipt,
        "asset_url_reuse": list(ALLOWED_URL_PREFIXES),
        "copied_10d_bytes": False,
        "timeline_copied": False,
        "woodland_scene_rewritten": False,
        "server_process": None,
    }
    (OUT_DIR / "receipt.json").write_text(
        json.dumps(payload, ensure_ascii=False, indent=2) + "\n", encoding="utf-8"
    )
    print(json.dumps(payload, indent=2))
    return 0


if __name__ == "__main__":
    sys.exit(main())
