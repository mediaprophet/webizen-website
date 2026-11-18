"""Self-check for EWM-04b / LW-08 ingest jobs. Does not call live adapters."""
from __future__ import annotations

import hashlib
import json
import sys
from pathlib import Path

HERE = Path(__file__).resolve().parent
ROOT = HERE.parents[3]
JOB = HERE / "job.json"
RIVER_JOB = HERE / "jobs" / "river-osm.json"
PUBLIC_BBOX = [149.06, -35.30, 149.08, -35.28]
FORBIDDEN = ("timeline.json", "inferred_work", "placeids", "gbif.org")
LIVE_FETCH_HOSTS = (
    "overpass-api.de",
    "api.gbif.org",
    "elevation-tiles-prod",
    "earth-search.aws.element84.com",
)


def fail(msg: str) -> int:
    print(f"EWM ingest job check failed: {msg}", file=sys.stderr)
    return 1


def sha256_file(path: Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as handle:
        for chunk in iter(lambda: handle.read(1024 * 1024), b""):
            digest.update(chunk)
    return digest.hexdigest()


def check_common(job: dict, raw: str, label: str) -> str | None:
    lowered = raw.lower()
    for token in FORBIDDEN:
        if token in lowered:
            return f"{label} forbidden token {token}"
    if job.get("schema") != "webizen.ewm-gis-ingest-job/v1":
        return f"{label} schema"
    if job.get("bbox_wsen") != PUBLIC_BBOX:
        return f"{label} bbox_wsen must be the public Canberra window"
    if job.get("opt_in") is not False:
        return f"{label} opt_in must stay false in the checked-in job"
    if job.get("create_only") is not True:
        return f"{label} create_only"
    if job.get("public_boot_calls_live_adapters") is not False:
        return f"{label} public_boot_calls_live_adapters"
    if job.get("streaming_default") is not False:
        return f"{label} streaming_default"
    if job.get("epistemic_class") != "measured-gis":
        return f"{label} epistemic_class"
    if job.get("not_personal_campsite") is not True:
        return f"{label} not_personal_campsite"
    if job.get("gbif_live_parse") != "deferred":
        return f"{label} gbif_live_parse"
    if job.get("homepage_runs_ingest") is not False:
        return f"{label} homepage_runs_ingest"
    if job.get("camera_tick_scrape") is not False:
        return f"{label} camera_tick_scrape"
    if job.get("timeline_json") != "never":
        return f"{label} timeline_json"
    if job.get("wasm_ingest_opt_in_allowed") is not False:
        return f"{label} wasm_ingest_opt_in_allowed"
    out = job.get("output_dir")
    if not isinstance(out, str) or not out.startswith("data/ewm/gis/ingest/out/"):
        return f"{label} output_dir"
    adapters = job.get("adapters")
    if not isinstance(adapters, list):
        return f"{label} adapters"
    return None


def adapters_by_id(job: dict) -> dict:
    return {row.get("id"): row for row in job.get("adapters") if isinstance(row, dict)}


def check_default_job(job: dict) -> str | None:
    if job.get("place_id") != "canberra-public-reference":
        return "place_id"
    by_id = adapters_by_id(job)
    dem = by_id.get("DemAdapter")
    if not dem or dem.get("required") is not True or dem.get("live") is not False:
        return "DemAdapter must be required and not live"
    for later in ("StacAdapter", "OsmAdapter"):
        row = by_id.get(later)
        if not row or row.get("live") is not False or row.get("role") != "same-pattern-later":
            return f"{later} must be same-pattern-later and not live"
    if "GbifAdapter" in by_id and by_id["GbifAdapter"].get("live") is True:
        return "GBIF must not be live"
    return None


def check_river_job(job: dict) -> str | None:
    if job.get("place_id") != "river":
        return "river place_id"
    if job.get("scene_id") != "river-osm-waterway":
        return "river scene_id"
    by_id = adapters_by_id(job)
    osm = by_id.get("OsmAdapter")
    if not osm or osm.get("required") is not True or osm.get("live") is not False:
        return "OsmAdapter must be required and not live"
    for later in ("DemAdapter", "StacAdapter"):
        row = by_id.get(later)
        if not row or row.get("live") is not False:
            return f"{later} must not be live"
    if "GbifAdapter" in by_id and (
        by_id["GbifAdapter"].get("live") is True or by_id["GbifAdapter"].get("required") is True
    ):
        return "GBIF must not be live"
    fixture = job.get("consent_fixture")
    if not isinstance(fixture, str) or not fixture.startswith("data/ewm/gis/ingest/fixtures/"):
        return "river consent_fixture"
    path = ROOT / fixture
    if not path.is_file():
        return "river consent fixture missing"
    raw = path.read_text(encoding="utf-8")
    lowered = raw.lower()
    for token in FORBIDDEN:
        if token in lowered:
            return f"fixture forbidden token {token}"
    for host in LIVE_FETCH_HOSTS:
        if host in lowered:
            return f"fixture live host {host}"
    document = json.loads(raw)
    if document.get("generator") != "webizen.ewm-gis-consent-fixture/v1":
        return "fixture generator"
    elements = document.get("elements")
    if not isinstance(elements, list) or not elements:
        return "fixture elements"
    return None


def check_river_member() -> str | None:
    member = HERE / "out" / "river" / "measured-member.json"
    catalog = HERE / "out" / "river" / "catalog-row.json"
    receipt = HERE / "out" / "river" / "ingest-receipt.json"
    if not member.is_file() or not catalog.is_file() or not receipt.is_file():
        return "river create-only measured member missing; run --opt-in with the consent fixture"
    row = json.loads(catalog.read_text(encoding="utf-8"))
    digest = sha256_file(member)
    if row.get("epistemic_class") != "measured-gis":
        return "catalog-row epistemic_class"
    if row.get("scene_id") != "river-osm-waterway":
        return "catalog-row scene_id"
    if row.get("sha256") != digest:
        return "catalog-row sha256 must match measured-member.json"
    if row.get("url") != f"/packages/by-sha256/{digest}":
        return "catalog-row url must be /packages/by-sha256/<sha256>"
    if len(digest) != 64 or any(c not in "0123456789abcdef" for c in digest):
        return "measured member sha256"
    if digest == "0" * 64:
        return "measured member sha256 must be non-zero"
    payload = json.loads(member.read_text(encoding="utf-8"))
    if payload.get("gbif_live_parse") != "deferred":
        return "measured member gbif_live_parse"
    if payload.get("wasm_ingest_opt_in_allowed") is not False:
        return "measured member wasm_ingest_opt_in_allowed"
    if payload.get("source") != "consent-fixture":
        return "measured member source"
    rec = json.loads(receipt.read_text(encoding="utf-8"))
    if rec.get("gbif_live_parse") != "deferred":
        return "receipt gbif_live_parse"
    if rec.get("create_only") is not True:
        return "receipt create_only"
    return None


def main() -> int:
    raw = JOB.read_text(encoding="utf-8")
    job = json.loads(raw)
    err = check_common(job, raw, "job.json") or check_default_job(job)
    if err:
        return fail(err)
    if not RIVER_JOB.is_file():
        return fail("missing jobs/river-osm.json")
    river_raw = RIVER_JOB.read_text(encoding="utf-8")
    river = json.loads(river_raw)
    err = check_common(river, river_raw, "river-osm.json") or check_river_job(river)
    if err:
        return fail(err)
    err = check_river_member()
    if err:
        return fail(err)
    print("EWM ingest job check passed")
    return 0


if __name__ == "__main__":
    sys.exit(main())
