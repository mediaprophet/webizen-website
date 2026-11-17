"""Download pack identities match the streamed first-paint catalog."""
import hashlib
import json
from pathlib import Path

ROOT = Path(__file__).resolve().parents[3]


def test_download_identities_match_catalog():
    catalog = json.loads((ROOT / "data/ewm/first-paint.json").read_text(encoding="utf-8"))
    pack = json.loads(
        (ROOT / "packages/download/first-paint.json").read_text(encoding="utf-8")
    )
    assert pack["streaming_default"] is False
    assert pack["same_as_stream"] is True
    by_id = {row["scene_id"]: row for row in catalog["entries"]}
    assert len(pack["identities"]) == 10
    for row in pack["identities"]:
        cat = by_id[row["scene_id"]]
        assert row["sha256"] == cat["sha256"]
        assert row["bytes"] == cat["bytes"]
        src = ROOT / row["url"].lstrip("/")
        assert hashlib.sha256(src.read_bytes()).hexdigest() == row["sha256"]


def test_consent_prefixes():
    consent = json.loads(
        (ROOT / "data/ewm/consent/ewm_static.json").read_text(encoding="utf-8")
    )
    assert consent["prefixes"] == ["/packages/", "/production/"]
    assert consent["live_gis_default"] == "deny"


def test_pointer_keeps_streaming_off():
    pointer = json.loads((ROOT / "packages/current.json").read_text(encoding="utf-8"))
    assert pointer["streaming"]["default"] is False
    assert pointer["catalog"] == "/data/ewm/first-paint.json"


def test_qualification_stays_honest():
    evidence = json.loads((ROOT / "evidence/ewm/qualification.json").read_text(encoding="utf-8"))
    assert evidence["streaming_default"] is False
    assert evidence["inner_ranges"] is False
    assert evidence["first_paint"]["bytes"] == 14_632_064
    assert evidence["wasm"]["measured_after_ewm_rebuild"] is True
    assert evidence["wasm"]["bytes_today"] < evidence["wasm"]["ci_budget_bytes"]
    assert evidence["download"]["opfs_wired"] is False
    assert evidence["candidate_origin"]["kind"] == "pending-producer"
    assert "honest_gaps" in evidence


def test_download_button_locks_in_flight():
    source = (ROOT / "native/world-host/src/runtime/ui/download.rs").read_text(encoding="utf-8")
    assert "DOWNLOAD_BUSY" in source
    assert "aria-busy" in source
    assert "OPFS is not yet wired" in source
