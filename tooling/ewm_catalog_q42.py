"""Compile the EWM catalog to Q42VOL v4 CBOR-LD SuperBlocks.

Turtle is the producer RDF. Visitors never read the Q42 file. UnifiedVolumeBuilder
writes CBOR-LD SuperBlocks of NQuins. GIS place IRIs join the woodland scene graph
so the host can SPARQL a public place onto the organs that populate that render.

Literal terms are RDF-typed (IRI, number, boolean, text). first-paint.json remains
a producer dump for schema tests. The native host must not fetch it.
place-index.json adds per-place Base (placeBase / hasBaseAsset). field-studio Base
is the woodland first-paint 24. This producer never writes first-paint.json.
streaming.default stays false. inner_ranges stays false. Four-tuple stays 24.
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
PLACE_INDEX = ROOT / "data/ewm/place-index.json"
OUT_DIR = ROOT / "data/ewm"
HELPER = ROOT / "adapters/qualiadb/q42_web.exe"
PUB_DIR = ROOT / "publication/generated"
ASSETS = ROOT / "production/native-world/assets"
BASE = "https://webizen.au/"
REL = BASE + "relation/"
CAT = BASE + "catalog/"
ASSET_NS = BASE + "scene/asset/"
JOURNEY_NS = BASE + "journey/"
GEO = "http://www.opengis.net/ont/geosparql#"
PACK = CAT + "canberra-public-place"
PLACE_KIND = CAT + "PublicPlace"
ENTRY_KIND = CAT + "Entry"
LAYER_KIND = CAT + "NoosphereLayer"
CATALOG_KIND = CAT + "Catalog"
WOODLAND_ORGANS = 85
AWNING_ID = "field-studio-operator-awning"
AWNING_VERTICES = 408
AWNING_TRIANGLES = 136
CAMPSITE_AABB = ([-30.0, 0.0, 20.0], [-10.0, 8.0, 40.0])
NUMBER = re.compile(r"^-?(?:0|[1-9]\d*)(?:\.\d+)?(?:[eE][+-]?\d+)?$")
FORBIDDEN = ("Timeline.json", "placeIds", "placeId", "INFERRED_WORK")


def sha256(path: Path) -> str:
    return hashlib.sha256(path.read_bytes()).hexdigest()


def lit_int(value) -> str:
    return str(int(value))


def lit_float(value) -> str:
    n = float(value)
    if n != n or n in (float("inf"), float("-inf")):
        raise SystemExit("Non-finite catalog number")
    return json.dumps(n, ensure_ascii=False)


def lit_bool(value) -> str:
    return "true" if value else "false"


def ttl_term(term: str) -> str:
    if term.startswith("http://") or term.startswith("https://"):
        return "<" + term + ">"
    if term in ("true", "false", "null") or NUMBER.match(term):
        return term
    return json.dumps(term, ensure_ascii=False)


def as_wkt(bbox: list[float]) -> str:
    w, s, e, n = (float(bbox[0]), float(bbox[1]), float(bbox[2]), float(bbox[3]))
    return f"POLYGON(({w} {s}, {e} {s}, {e} {n}, {w} {n}, {w} {s}))"


def refuse_forbidden(blob: object, path: str) -> None:
    text = json.dumps(blob, ensure_ascii=False)
    for token in FORBIDDEN:
        if token in text:
            raise SystemExit(f"{path} must not contain {token}")


def publish_world_q42() -> dict:
    pointer = json.loads((PUB_DIR / "current.json").read_text(encoding="utf-8"))
    named = PUB_DIR / pointer["q42"]
    if not named.is_file():
        raise SystemExit(f"Missing content-addressed publication {named}")
    served = PUB_DIR / "world.q42"
    served.write_bytes(named.read_bytes())
    digest = sha256(served)
    if digest != pointer["sha256"]:
        raise SystemExit("Well-known world.q42 digest differs from producer pointer")
    return {
        "q42": "publication/generated/world.q42",
        "source": pointer["q42"],
        "sha256": digest,
        "bytes": served.stat().st_size,
        "encoding": "q42vol-v4-cbor-ld",
    }


def load_place_index(first_paint: list[str]) -> dict:
    if not PLACE_INDEX.is_file():
        raise SystemExit(f"Missing place index {PLACE_INDEX}")
    index = json.loads(PLACE_INDEX.read_text(encoding="utf-8"))
    refuse_forbidden(index, "data/ewm/place-index.json")
    if index.get("schema") != "webizen.ewm-place-base/v1":
        raise SystemExit("Unsupported place-index schema")
    if index.get("inner_ranges") is not False:
        raise SystemExit("place-index must keep inner_ranges false")
    if index.get("streaming", {}).get("default") is not False:
        raise SystemExit("place-index must keep streaming.default false")
    places = index.get("places") or []
    ids = [p.get("id") for p in places]
    for required in ("field-studio", "river", "canopy", "commons"):
        if required not in ids:
            raise SystemExit(f"place-index missing PublicPlace {required}")
    studio = next(p for p in places if p["id"] == "field-studio")
    if list(studio.get("base") or []) != list(first_paint):
        raise SystemExit("field-studio base must equal woodland first-paint 24")
    if AWNING_ID in studio.get("base") or []:
        raise SystemExit("awning must not join the field-studio four-tuple")
    if AWNING_ID not in (studio.get("enhancement") or []):
        raise SystemExit("awning must be a field-studio Enhancement")
    return index


def extra_enhancement_row(scene_id: str, entry_id: int, place_id: str) -> dict:
    path = ASSETS / f"{scene_id}.10d"
    if not path.is_file():
        raise SystemExit(f"Missing enhancement organ {path}")
    raw = path.read_bytes()
    verts, tris = 0, 0
    if scene_id == AWNING_ID:
        verts, tris = AWNING_VERTICES, AWNING_TRIANGLES
    mn, mx = CAMPSITE_AABB
    return {
        "entry_id": entry_id,
        "scene_id": scene_id,
        "place_id": place_id,
        "epistemic_class": "authored-artwork",
        "family": "TenD",
        "family_from": "inner-magic",
        "role": "Enhancement",
        "obligation": "Optional",
        "sha256": hashlib.sha256(raw).hexdigest(),
        "bytes": len(raw),
        "vertices": verts,
        "triangles": tris,
        "url": "/production/native-world/assets/" + path.name,
        "shard": "enhancement",
        "aabb": {"min": list(mn), "max": list(mx)},
        "tiles": [[-1, 0]],
        "planner_region": None,
    }


def emit_place(fact, pack: str, place: dict, bases: dict, enhancements: dict) -> None:
    uri = CAT + "place/" + place["id"]
    fact(pack, "hasPlace", uri)
    fact(uri, "kind", PLACE_KIND)
    fact(uri, "id", place["id"])
    fact(uri, "title", place["title"])
    if place.get("journey_chapter"):
        fact(uri, "journeyChapter", JOURNEY_NS + place["journey_chapter"])
    fact(uri, "notPersonalCampsite", lit_bool(bool(place.get("not_personal_campsite", True))))
    fact(uri, "crs", place.get("crs", "EPSG:4326"))
    fact(uri, "datum", place.get("datum", "WGS84"))
    bbox = place["bbox_wsen"]
    fact(uri, "bboxW", lit_float(bbox[0]))
    fact(uri, "bboxS", lit_float(bbox[1]))
    fact(uri, "bboxE", lit_float(bbox[2]))
    fact(uri, "bboxN", lit_float(bbox[3]))
    fact(uri, "asWkt", as_wkt(bbox))
    elev = place.get("elevation_m") or [0, 0]
    fact(uri, "elevationMin", lit_float(elev[0]))
    fact(uri, "elevationMax", lit_float(elev[1]))
    if place.get("dem_zoom") is not None:
        fact(uri, "demZoom", lit_int(place["dem_zoom"]))
    if place.get("gis_terrain_id"):
        fact(uri, "gisTerrain", ASSET_NS + place["gis_terrain_id"])
    if place.get("authored_ground_id"):
        fact(uri, "authoredGround", ASSET_NS + place["authored_ground_id"])
    fact(uri, "usri", place["usri"])
    fact(uri, "realmClass", place["realm_class"])
    for layer in place.get("colocated") or []:
        l_uri = CAT + "layer/" + layer["id"]
        fact(uri, "hasColocated", l_uri)
        fact(l_uri, "kind", LAYER_KIND)
        fact(l_uri, "id", layer["id"])
        fact(l_uri, "label", layer.get("label", "infosphere"))
        fact(l_uri, "realmClass", layer["realm_class"])
        fact(l_uri, "usri", layer["usri"])
        fact(l_uri, "grounding", layer["grounding"])
        fact(l_uri, "anchorIri", layer["anchor_iri"])
        if layer.get("journey_iri"):
            fact(l_uri, "journeyIri", layer["journey_iri"])
    for scene_id in bases.get(place["id"]) or []:
        iri = ASSET_NS + scene_id
        fact(uri, "placeBase", iri)
        fact(uri, "hasBaseAsset", iri)
    for scene_id in enhancements.get(place["id"]) or []:
        fact(uri, "hasEnhancementAsset", ASSET_NS + scene_id)


def main() -> int:
    catalog = json.loads(CATALOG_JSON.read_text(encoding="utf-8"))
    refuse_forbidden(
        {k: v for k, v in catalog.items() if k not in ("entries", "places")},
        "data/ewm/first-paint.json header",
    )
    if catalog.get("schema") != "webizen.ewm-catalog/v1":
        raise SystemExit("Unsupported EWM catalog schema")
    if catalog.get("streaming", {}).get("default") is True:
        raise SystemExit("Producer refused streaming.default true")
    if catalog.get("inner_ranges") is True:
        raise SystemExit("Producer refused inner_ranges true")
    first_paint = list(catalog.get("first_paint") or [])
    if len(first_paint) != 24:
        raise SystemExit("Woodland first-paint membership must stay 24")
    place_index = load_place_index(first_paint)
    world = publish_world_q42()

    bases: dict[str, list[str]] = {}
    enhancements: dict[str, list[str]] = {}
    for place in place_index.get("places") or []:
        bases[place["id"]] = list(place.get("base") or [])
        enhancements[place["id"]] = list(place.get("enhancement") or [])
    if "canberra-public-reference" not in bases:
        bases["canberra-public-reference"] = list(first_paint)

    places = list(catalog.get("places") or [])
    seen = {p["id"] for p in places}
    for place in place_index.get("places") or []:
        if place["id"] in seen:
            continue
        places.append(place)
        seen.add(place["id"])

    entries = list(catalog.get("entries") or [])
    for row in entries:
        url = row.get("url") or ""
        if not url.startswith("/"):
            continue
        path = ROOT / url.lstrip("/")
        if not path.is_file():
            continue
        raw = path.read_bytes()
        row["sha256"] = hashlib.sha256(raw).hexdigest()
        row["bytes"] = len(raw)
    catalog["entries"] = entries
    CATALOG_JSON.write_text(
        json.dumps(catalog, ensure_ascii=False, indent=2) + "\n",
        encoding="utf-8",
    )
    present = {row["scene_id"] for row in entries}
    next_id = max((row["entry_id"] for row in entries), default=0) + 1
    extra = []
    for place in place_index.get("places") or []:
        for scene_id in place.get("enhancement") or []:
            if scene_id in present:
                continue
            row = extra_enhancement_row(scene_id, next_id, place["id"])
            entries.append(row)
            extra.append(row)
            present.add(scene_id)
            next_id += 1
    if len(entries) < WOODLAND_ORGANS:
        raise SystemExit("Catalog must describe the 85 woodland organs")

    triples: list[list[str]] = []

    def fact(s: str, p: str, o: str) -> None:
        triples.append([s, REL + p, o])

    fact(PACK, "kind", CATALOG_KIND)
    fact(PACK, "schema", "webizen.ewm-catalog/v1")
    fact(PACK, "packId", catalog["pack_id"])
    fact(PACK, "innerRanges", lit_bool(False))
    fact(PACK, "streamingDefault", lit_bool(False))
    fact(PACK, "queryToken", catalog.get("streaming", {}).get("query_token", "ewm=1"))

    cand = catalog["candidate"]
    fact(PACK, "descriptorSha384", cand["descriptor_sha384"])
    fact(PACK, "contextSha384", cand["context_sha384"])
    fact(PACK, "candidateGeneration", lit_int(cand.get("generation", 0)))
    fact(PACK, "candidateEntries", lit_int(cand["entries"]))

    declared = catalog["declared_subset"]
    fact(PACK, "declaredIds", lit_int(declared["ids"]))
    fact(PACK, "declaredBytes", lit_int(declared["bytes"]))
    fact(PACK, "declaredTriangles", lit_int(declared["triangles"]))
    fact(PACK, "declaredVertices", lit_int(declared["vertices"]))
    fact(PACK, "v1BudgetBytes", lit_int(declared["v1_budget_bytes"]))

    for place in places:
        emit_place(fact, PACK, place, bases, enhancements)

    for scene_id in first_paint:
        fact(PACK, "firstPaint", ASSET_NS + scene_id)

    for row in entries:
        uri = CAT + "entry/" + str(row["entry_id"])
        fact(PACK, "hasEntry", uri)
        fact(uri, "kind", ENTRY_KIND)
        fact(uri, "entryId", lit_int(row["entry_id"]))
        fact(uri, "id", row["scene_id"])
        fact(uri, "sceneAsset", ASSET_NS + row["scene_id"])
        fact(uri, "placeId", row.get("place_id", ""))
        if row.get("place_id"):
            fact(uri, "forPlace", CAT + "place/" + row["place_id"])
        fact(uri, "epistemicClass", row.get("epistemic_class", "authored-artwork"))
        fact(uri, "family", row.get("family", "TenD"))
        fact(uri, "role", row["role"])
        fact(uri, "obligation", row["obligation"])
        fact(uri, "sha256", row["sha256"])
        fact(uri, "bytes", lit_int(row["bytes"]))
        fact(uri, "vertices", lit_int(row.get("vertices") or 0))
        fact(uri, "triangles", lit_int(row.get("triangles") or 0))
        fact(uri, "url", row["url"])
        fact(uri, "shard", row.get("shard", "base"))
        aabb = row.get("aabb") or {}
        mn = aabb.get("min") or []
        mx = aabb.get("max") or []
        if len(mn) == 3 and len(mx) == 3:
            fact(uri, "aabbMinX", lit_float(mn[0]))
            fact(uri, "aabbMinY", lit_float(mn[1]))
            fact(uri, "aabbMinZ", lit_float(mn[2]))
            fact(uri, "aabbMaxX", lit_float(mx[0]))
            fact(uri, "aabbMaxY", lit_float(mx[1]))
            fact(uri, "aabbMaxZ", lit_float(mx[2]))
        for tile in row.get("tiles") or []:
            if not isinstance(tile, list) or len(tile) != 2:
                continue
            t_uri = CAT + f"tile/{row['entry_id']}/{int(tile[0])}/{int(tile[1])}"
            fact(uri, "hasTile", t_uri)
            fact(t_uri, "tileX", lit_int(tile[0]))
            fact(t_uri, "tileZ", lit_int(tile[1]))

    for edge in catalog.get("edges") or []:
        uri = CAT + f"edge/{edge['from_entry_id']}/{edge['to_entry_id']}"
        fact(PACK, "hasEdge", uri)
        fact(uri, "fromEntryId", lit_int(edge["from_entry_id"]))
        fact(uri, "toEntryId", lit_int(edge["to_entry_id"]))
        fact(uri, "reason", edge["reason"])
        fact(uri, "obligation", edge.get("obligation", "Mandatory"))

    for row in triples:
        if row[1] == REL + "asWkt":
            row[1] = GEO + "asWKT"

    triples = sorted({tuple(row) for row in triples})
    triples = [list(row) for row in triples]
    triple_path = OUT_DIR / "catalog-triples.json"
    triple_path.write_text(
        json.dumps(triples, ensure_ascii=False, indent=2) + "\n", encoding="utf-8"
    )
    ttl = "\n".join(" ".join(ttl_term(part) for part in row) + " ." for row in triples) + "\n"
    (OUT_DIR / "catalog.ttl").write_text(ttl, encoding="utf-8")

    digest = hashlib.sha256(
        json.dumps(triples, ensure_ascii=False, separators=(",", ":")).encode("utf-8")
    ).hexdigest()
    named = OUT_DIR / f"catalog-{digest[:16]}.q42"
    served = OUT_DIR / "catalog.q42"
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
        raise SystemExit("Q42 quin count differs from authored catalog triples")
    paints = [row[2] for row in loaded["rows"] if row[1].endswith("/firstPaint")]
    if len(paints) != 24:
        raise SystemExit("Catalog Q42 firstPaint membership must stay 24")
    streaming = [row[2] for row in loaded["rows"] if row[1].endswith("/streamingDefault")]
    if streaming != ["false"]:
        raise SystemExit("Catalog Q42 streamingDefault must stay false")
    inner = [row[2] for row in loaded["rows"] if row[1].endswith("/innerRanges")]
    if inner != ["false"]:
        raise SystemExit("Catalog Q42 innerRanges must stay false")
    print(
        json.dumps(
            {
                "q42": str(served.relative_to(ROOT)).replace("\\", "/"),
                "named": str(named.relative_to(ROOT)).replace("\\", "/"),
                "sha256": sha256(served),
                "bytes": served.stat().st_size,
                "quins": loaded["total_quins"],
                "first_paint": len(first_paint),
                "entries": len(entries),
                "extra_enhancement": [row["scene_id"] for row in extra],
                "places": {
                    p["id"]: {
                        "base": bases.get(p["id"]) or [],
                        "enhancement": enhancements.get(p["id"]) or [],
                    }
                    for p in places
                },
                "ttl": "data/ewm/catalog.ttl",
                "encoding": "q42vol-v4-cbor-ld",
                "streaming_default": False,
                "inner_ranges": False,
                "publication_world": world,
            },
            indent=2,
        )
    )
    return 0


if __name__ == "__main__":
    sys.exit(main())
