"""Bounded GIS ingest job stays opt-in, create-only, and public-Canberra."""
import json
from pathlib import Path

ROOT = Path(__file__).resolve().parents[3]
PUBLIC_BBOX = [149.06, -35.30, 149.08, -35.28]


def test_ingest_job_schema_place_bbox_opt_in_false():
    job = json.loads(
        (ROOT / "data/ewm/gis/ingest/job.json").read_text(encoding="utf-8")
    )
    assert job["schema"] == "webizen.ewm-gis-ingest-job/v1"
    assert job["place_id"] == "canberra-public-reference"
    assert job["bbox_wsen"] == PUBLIC_BBOX
    assert job["opt_in"] is False
    assert job["public_boot_calls_live_adapters"] is False
    assert job["streaming_default"] is False
    assert job["not_personal_campsite"] is True


def test_ewm_ingest_helper_refuses_without_opt_in():
    source = (ROOT / "tooling/ewm_ingest.py").read_text(encoding="utf-8")
    assert "--opt-in" in source
    assert '"--opt-in" not in args' in source
    assert "create-only" in source
    assert "native-receipt.json" in source
    assert "existing native receipt kept" in source
    assert "default-deny" in source


def test_gis_ingest_rs_default_deny_and_measured_member():
    source = (ROOT / "native/world-host/src/runtime/gis_ingest.rs").read_text(
        encoding="utf-8"
    )
    assert "pub fn public_boot_calls_live_adapters() -> bool" in source
    start = source.index("pub fn public_boot_calls_live_adapters() -> bool")
    body = source[start : start + 80]
    assert "false" in body
    assert "pub fn append_measured_member" in source
    assert 'lowered.contains("timeline.json")' in source
    assert 'lowered.contains("inferred_work")' in source
    assert 'lowered.contains("placeids")' in source
    assert "semanticSegments" not in source
    assert "INFERRED_WORK/home" not in source


def test_job_and_readme_have_no_timeline_payload_or_campsite_pins():
    job_text = (ROOT / "data/ewm/gis/ingest/job.json").read_text(encoding="utf-8")
    readme = (ROOT / "data/ewm/gis/ingest/README.md").read_text(encoding="utf-8")
    job_lower = job_text.lower()
    assert "timeline.json" not in job_lower
    assert "inferred_work" not in job_lower
    assert "placeids" not in job_lower
    job = json.loads(job_text)
    assert job["bbox_wsen"] == PUBLIC_BBOX
    assert job.get("timeline_json") == "never"
    # README may list the tokens as forbidden; it must not publish a pin.
    assert "Never" in readme
    assert "Timeline.json" in readme
    assert "INFERRED_WORK" in readme
    assert "149.07" not in readme
    assert "-35.29" not in readme
    assert "campsite pins" not in readme.lower()
