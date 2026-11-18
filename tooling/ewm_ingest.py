"""EWM-04b / LW-08 native-first bounded GIS ingest. Opt-in only. Create-only."""
from __future__ import annotations

import hashlib
import json
import subprocess
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
DEFAULT_JOB = ROOT / "data" / "ewm" / "gis" / "ingest" / "job.json"
OUT_ROOT = ROOT / "data" / "ewm" / "gis" / "ingest" / "out"
PUBLIC_BBOX = [149.06, -35.30, 149.08, -35.28]
PUBLIC_PLACES = {
    "canberra-public-reference": "DemAdapter",
    "river": "OsmAdapter",
}
ARTEFACTS = (
    "native-receipt.json",
    "reference-terrain.10d",
    "source-tiles.qdem",
    "heightfield.json",
    "requests.json",
    "measured-member.json",
    "catalog-row.json",
    "ingest-receipt.json",
)
FORBIDDEN = ("timeline.json", "inferred_work", "placeids", "gbif.org")
LIVE_FETCH_HOSTS = (
    "overpass-api.de",
    "api.gbif.org",
    "elevation-tiles-prod",
    "earth-search.aws.element84.com",
)


def die(msg: str, code: int = 2) -> int:
    print(msg, file=sys.stderr)
    return code


def refuse_forbidden(text: str, label: str) -> None:
    lowered = text.lower()
    for token in FORBIDDEN:
        if token in lowered:
            raise SystemExit(f"EWM GIS ingest refuses {label} containing {token}")


def data_pack_binary() -> Path | None:
    builder = ROOT / "native" / "data-pack-builder"
    names = ("data-pack.exe",) if sys.platform == "win32" else ("data-pack", "data-pack.exe")
    for name in names:
        candidate = builder / name
        if candidate.is_file():
            return candidate
    return None


def sha256_file(path: Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as handle:
        for chunk in iter(lambda: handle.read(1024 * 1024), b""):
            digest.update(chunk)
    return digest.hexdigest()


def resolve_output_dir(job: dict) -> Path:
    rel = job.get("output_dir")
    if not isinstance(rel, str) or not rel.strip() or Path(rel).is_absolute():
        raise SystemExit("job output_dir must be a relative create-only path")
    posix = rel.replace("\\", "/")
    if not posix.startswith("data/ewm/gis/ingest/out/"):
        raise SystemExit("job output_dir must stay under data/ewm/gis/ingest/out/")
    out = (ROOT / posix).resolve()
    root_out = OUT_ROOT.resolve()
    if not out.is_relative_to(root_out) or out == root_out:
        raise SystemExit("job output_dir must be a create-only child of ingest/out/")
    return out


def adapters_by_id(job: dict) -> dict:
    adapters = job.get("adapters")
    if not isinstance(adapters, list):
        raise SystemExit("job adapters must be a list")
    by_id: dict = {}
    for row in adapters:
        if not isinstance(row, dict):
            continue
        adapter_id = row.get("id")
        if not isinstance(adapter_id, str) or not adapter_id:
            continue
        if row.get("live") is True:
            raise SystemExit(f"adapter {adapter_id} must not be live in v1")
        if adapter_id == "GbifAdapter" and (
            row.get("live") is True or row.get("required") is True
        ):
            raise SystemExit("Live GBIF stays deferred")
        by_id[adapter_id] = row
    return by_id


def required_adapter(job: dict) -> dict:
    by_id = adapters_by_id(job)
    place_id = job.get("place_id")
    expected = PUBLIC_PLACES.get(place_id)
    if expected is None:
        raise SystemExit("place_id must be a declared public ingest place")
    adapter = by_id.get(expected)
    if not adapter or adapter.get("required") is not True:
        raise SystemExit(f"{expected} is required")
    if adapter.get("live") is not False:
        raise SystemExit(f"{expected} must not be marked live in the checked-in job")
    return adapter


def validate_job(job: dict) -> None:
    if job.get("schema") != "webizen.ewm-gis-ingest-job/v1":
        raise SystemExit("schema must be webizen.ewm-gis-ingest-job/v1")
    if job.get("place_id") not in PUBLIC_PLACES:
        raise SystemExit("place_id must be a declared public ingest place")
    if job.get("bbox_wsen") != PUBLIC_BBOX:
        raise SystemExit("bbox_wsen must be the public Canberra window")
    if job.get("create_only") is not True:
        raise SystemExit("job must be create_only")
    if job.get("public_boot_calls_live_adapters") is not False:
        raise SystemExit("public boot must not call live adapters")
    if job.get("streaming_default") is not False:
        raise SystemExit("streaming.default stays false")
    if job.get("epistemic_class") != "measured-gis":
        raise SystemExit("epistemic_class must be measured-gis")
    if job.get("not_personal_campsite") is not True:
        raise SystemExit("job must declare not_personal_campsite")
    if job.get("gbif_live_parse") != "deferred":
        raise SystemExit("GBIF live parse stays deferred")
    if job.get("camera_tick_scrape") is not False:
        raise SystemExit("ingest is not a camera-tick scrape")
    if job.get("timeline_json") != "never":
        raise SystemExit("Timeline.json is never stored")
    if job.get("wasm_ingest_opt_in_allowed") is not False:
        raise SystemExit("wasm_ingest_opt_in_allowed remains false")
    required_adapter(job)


def collect_paths(out: Path) -> dict:
    paths = {}
    for name in ARTEFACTS:
        path = out / name
        if not path.is_file():
            continue
        rel = path.relative_to(ROOT).as_posix()
        paths[name] = {
            "path": rel,
            "sha256": sha256_file(path),
            "bytes": path.stat().st_size,
        }
    return paths


def write_ingest_receipt(job: dict, out: Path, reused: bool) -> Path:
    receipt_name = job.get("receipt_name") or "ingest-receipt.json"
    dest = out / receipt_name
    if dest.is_file():
        return dest
    native_name = job.get("native_receipt_name") or "native-receipt.json"
    native_path = out / native_name
    member_name = job.get("member_name") or "measured-member.json"
    member_path = out / member_name
    attribution = None
    licence = None
    adapter = required_adapter(job).get("id") or "DemAdapter"
    if native_path.is_file():
        native = json.loads(native_path.read_text(encoding="utf-8"))
        attribution = native.get("attribution")
        licence = native.get("licence_reference")
        adapter = native.get("adapter") or adapter
        if native.get("not_personal_campsite") is not True:
            raise SystemExit("native receipt must declare not_personal_campsite")
    elif member_path.is_file():
        member = json.loads(member_path.read_text(encoding="utf-8"))
        attribution = member.get("attribution")
        licence = member.get("licence_reference")
        adapter = member.get("adapter") or adapter
        if member.get("not_personal_campsite") is not True:
            raise SystemExit("measured member must declare not_personal_campsite")
        if member.get("gbif_live_parse") != "deferred":
            raise SystemExit("measured member GBIF live parse stays deferred")
    catalog_row = {
        "epistemic_class": "measured-gis",
        "url_must_start_with": ["/packages/", "/production/"],
        "note": "append_measured_member validates catalog rows; this job does not fetch HTTP",
    }
    catalog_name = job.get("catalog_row_name") or "catalog-row.json"
    catalog_path = out / catalog_name
    if catalog_path.is_file():
        catalog_row = json.loads(catalog_path.read_text(encoding="utf-8"))
        if catalog_row.get("epistemic_class") != "measured-gis":
            raise SystemExit("catalog-row epistemic_class must be measured-gis")
    receipt = {
        "schema": "webizen.ewm-gis-ingest-receipt/v1",
        "place_id": job["place_id"],
        "bbox_wsen": job["bbox_wsen"],
        "not_personal_campsite": True,
        "epistemic_class": "measured-gis",
        "create_only": True,
        "opt_in": True,
        "reused_existing": reused,
        "public_boot_calls_live_adapters": False,
        "wasm_ingest_opt_in_allowed": False,
        "gbif_live_parse": "deferred",
        "adapter": adapter,
        "attribution": attribution,
        "licence_reference": licence,
        "output_dir": out.relative_to(ROOT).as_posix(),
        "paths": collect_paths(out),
        "catalog_row": catalog_row,
    }
    dest.write_text(
        json.dumps(receipt, indent=2, ensure_ascii=False) + "\n", encoding="utf-8"
    )
    return dest


def resolve_consent_fixture(job: dict, args: list[str]) -> Path | None:
    fixture: Path | None = None
    if "--consent-fixture" in args:
        idx = args.index("--consent-fixture")
        if idx + 1 >= len(args):
            raise SystemExit("--consent-fixture requires a path")
        fixture = Path(args[idx + 1])
        if not fixture.is_absolute():
            fixture = (Path.cwd() / fixture).resolve()
    elif isinstance(job.get("consent_fixture"), str) and job["consent_fixture"].strip():
        rel = job["consent_fixture"].replace("\\", "/")
        if Path(rel).is_absolute() or ".." in Path(rel).parts:
            raise SystemExit("job consent_fixture must be a relative path")
        fixture = (ROOT / rel).resolve()
        if not fixture.is_relative_to(ROOT.resolve()):
            raise SystemExit("job consent_fixture must stay inside the website root")
    return fixture


def pack_osm_features(document: dict) -> list[dict]:
    elements = document.get("elements")
    if not isinstance(elements, list) or not elements:
        raise SystemExit("consent fixture must be a non-empty Overpass elements array")
    features = []
    for element in elements:
        if not isinstance(element, dict):
            continue
        kind = element.get("type")
        if kind not in ("node", "way", "relation"):
            continue
        feature = {
            "type": kind,
            "id": element.get("id"),
            "tags": element.get("tags") or {},
        }
        if "geometry" in element:
            feature["geometry"] = element.get("geometry")
        if "lat" in element and "lon" in element:
            feature["lat"] = element.get("lat")
            feature["lon"] = element.get("lon")
        if "nodes" in element:
            feature["nodes"] = element.get("nodes")
        features.append(feature)
    if not features:
        raise SystemExit("consent fixture produced no OSM features")
    return features


def write_osm_member(job: dict, out: Path, fixture: Path) -> Path:
    raw = fixture.read_text(encoding="utf-8")
    refuse_forbidden(raw, str(fixture))
    lowered = raw.lower()
    for host in LIVE_FETCH_HOSTS:
        if host in lowered:
            raise SystemExit(f"consent fixture must not target live host {host}")
    document = json.loads(raw)
    if document.get("generator") != "webizen.ewm-gis-consent-fixture/v1":
        raise SystemExit("consent fixture generator must be webizen.ewm-gis-consent-fixture/v1")
    features = pack_osm_features(document)
    scene_id = job.get("scene_id") or "river-osm-waterway"
    member = {
        "schema": "webizen.ewm-gis-measured-member/v1",
        "scene_id": scene_id,
        "place_id": job["place_id"],
        "epistemic_class": "measured-gis",
        "adapter": "OsmAdapter",
        "source": "consent-fixture",
        "not_personal_campsite": True,
        "bbox_wsen": job["bbox_wsen"],
        "inner_ranges": False,
        "gbif_live_parse": "deferred",
        "wasm_ingest_opt_in_allowed": False,
        "public_boot_calls_live_adapters": False,
        "licence_reference": "https://opendatacommons.org/licenses/odbl/1-0/",
        "attribution": "OpenStreetMap contributors",
        "feature_count": len(features),
        "features": features,
        "consent_fixture": fixture.relative_to(ROOT).as_posix(),
    }
    member_name = job.get("member_name") or "measured-member.json"
    member_path = out / member_name
    out.mkdir(parents=True, exist_ok=True)
    payload = json.dumps(member, indent=2, ensure_ascii=False) + "\n"
    member_path.write_text(payload, encoding="utf-8")
    digest = sha256_file(member_path)
    catalog = {
        "scene_id": scene_id,
        "epistemic_class": "measured-gis",
        "url": f"/packages/by-sha256/{digest}",
        "sha256": digest,
        "bytes": member_path.stat().st_size,
        "role": "Enhancement",
        "obligation": "opt-in-ingest",
        "inner_ranges": False,
    }
    catalog_name = job.get("catalog_row_name") or "catalog-row.json"
    (out / catalog_name).write_text(
        json.dumps(catalog, indent=2, ensure_ascii=False) + "\n", encoding="utf-8"
    )
    return member_path


def run_osm(job: dict, out: Path, fixture: Path | None) -> Path:
    if fixture is None:
        raise SystemExit(
            "OSM ingest requires --consent-fixture or job.consent_fixture; "
            "live Overpass hosts are not fetched"
        )
    if not fixture.is_file():
        raise SystemExit(f"missing consent fixture {fixture}")
    receipt_name = job.get("receipt_name") or "ingest-receipt.json"
    receipt = out / receipt_name
    member_name = job.get("member_name") or "measured-member.json"
    if receipt.is_file():
        return write_ingest_receipt(job, out, reused=True)
    if (out / member_name).is_file():
        raise SystemExit("create-only output exists without ingest-receipt.json; refusing overwrite")
    write_osm_member(job, out, fixture)
    return write_ingest_receipt(job, out, reused=False)


def run_dem(binary: Path, out: Path) -> None:
    out.mkdir(parents=True, exist_ok=True)
    proc = subprocess.run(
        [str(binary), "dem", str(out)],
        cwd=str(binary.parent),
        capture_output=True,
        text=True,
        encoding="utf-8",
        errors="replace",
        check=False,
    )
    if proc.returncode != 0:
        err = (proc.stderr or proc.stdout or "data-pack dem failed").strip()
        raise SystemExit(f"data-pack dem refused: {err}")


def main(argv: list[str] | None = None) -> int:
    args = list(sys.argv[1:] if argv is None else argv)
    if any(a in ("-h", "--help") for a in args):
        print(
            "EWM GIS ingest is default-deny.\n"
            "Usage: python tooling/ewm_ingest.py --opt-in [--job PATH] [--consent-fixture PATH]\n"
            "Homepage never runs this. Live GBIF is deferred. Timeline.json is never read.\n"
            "CI packs OSM from a consent fixture; it does not fetch live hosts."
        )
        return 0
    refuse_forbidden(" ".join(args), "argv")
    if "--opt-in" not in args:
        return die(
            "EWM GIS ingest is default-deny until --opt-in. "
            "Public boot does not call live adapters."
        )
    job_path = DEFAULT_JOB
    if "--job" in args:
        idx = args.index("--job")
        if idx + 1 >= len(args):
            return die("--job requires a path")
        job_path = Path(args[idx + 1])
        if not job_path.is_absolute():
            job_path = (Path.cwd() / job_path).resolve()
    if not job_path.is_file():
        return die(f"missing job {job_path}")
    raw = job_path.read_text(encoding="utf-8")
    refuse_forbidden(raw, str(job_path))
    job = json.loads(raw)
    validate_job(job)
    out = resolve_output_dir(job)
    adapter = required_adapter(job)
    if adapter.get("id") == "OsmAdapter":
        fixture = resolve_consent_fixture(job, args)
        receipt_name = job.get("receipt_name") or "ingest-receipt.json"
        existed = (out / receipt_name).is_file()
        receipt = run_osm(job, out, fixture)
        if existed:
            print(
                f"create-only: existing native receipt kept; {receipt.relative_to(ROOT).as_posix()}"
            )
        else:
            print(f"ingest receipt {receipt.relative_to(ROOT).as_posix()}")
        return 0
    native_receipt = out / (job.get("native_receipt_name") or "native-receipt.json")
    if native_receipt.is_file():
        receipt = write_ingest_receipt(job, out, reused=True)
        print(f"create-only: existing native receipt kept; {receipt.relative_to(ROOT).as_posix()}")
        return 0
    if (out / "reference-terrain.10d").is_file():
        return die("create-only output exists without native-receipt.json; refusing overwrite")
    binary = data_pack_binary()
    if binary is None:
        return die(
            "native data-pack binary is missing; wasm public boot does not ingest. "
            "Build data-pack separately; this helper does not cargo."
        )
    run_dem(binary, out)
    if not native_receipt.is_file():
        return die("data-pack dem did not write native-receipt.json")
    receipt = write_ingest_receipt(job, out, reused=False)
    print(f"ingest receipt {receipt.relative_to(ROOT).as_posix()}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
