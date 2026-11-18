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
    assert len(pack["identities"]) == 24
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
    catalog = json.loads((ROOT / "data/ewm/first-paint.json").read_text(encoding="utf-8"))
    assert catalog["streaming"]["default"] is False
    q42 = ROOT / "data/ewm/catalog.q42"
    assert q42.is_file()
    assert q42.read_bytes()[:3] == b"Q42"


def test_host_does_not_fetch_json_catalog_files():
    boot = (ROOT / "native/world-host/src/runtime/boot.rs").read_text(encoding="utf-8")
    stream = (ROOT / "native/world-host/src/runtime/stream/mod.rs").read_text(encoding="utf-8")
    download = (ROOT / "native/world-host/src/runtime/ui/download.rs").read_text(encoding="utf-8")
    assert "json_file(\"/packages/current.json\"" not in stream
    assert "json_file(\"/data/ewm/first-paint.json\"" not in stream
    assert "json_file(\"/packages/download/first-paint.json\"" not in download
    assert "json_file(\"/publication/current.json\"" not in boot
    assert "/data/ewm/catalog.q42" in boot
    assert "/publication/world.q42" in boot


def test_qualification_stays_honest():
    evidence = json.loads((ROOT / "evidence/ewm/qualification.json").read_text(encoding="utf-8"))
    assert evidence["schema"] == "webizen.ewm-qualification/v1"
    assert evidence["locale"] == "en-AU"
    assert evidence["streaming_default"] is False
    assert evidence["inner_ranges"] is False
    assert evidence["browser_ran"] is True
    assert evidence["art_approval"] is False
    assert evidence["first_paint"]["bytes"] == 16_644_944
    assert evidence["first_paint"]["triangles"] == 137048
    assert evidence["wasm"]["measured_after_ewm_rebuild"] is True
    wasm = (ROOT / "site/pkg/native-world/webizen_world_bg.wasm").read_bytes()
    assert evidence["wasm"]["bytes_today"] == len(wasm)
    assert evidence["wasm"]["sha256_today"] == hashlib.sha256(wasm).hexdigest()
    assert evidence["wasm"]["ci_budget_bytes"] == 24_641_536
    assert evidence["wasm"]["bytes_today"] < evidence["wasm"]["ci_budget_bytes"]
    assert evidence["download"]["opfs_wired"] is True
    assert evidence["download"]["opfs"]["browser_verified"] is True
    assert evidence["download"]["opfs"]["fallback_used"] is False
    assert evidence["download"]["opfs"]["blob_save_kept"] is False
    assert evidence["download"]["opfs"]["identities"] == 24
    assert "Nothing was sent to the downloads folder" in evidence["download"]["opfs"]["status_text"]
    assert evidence["candidate_origin"]["kind"] == "inventory-verifier-finish"
    assert evidence["candidate_origin"]["host_bind"] == "CandidateIdentity::from_parts"
    assert evidence["candidate_origin"]["entries"] == 24
    assert evidence["candidate_origin"]["generation"] == 0
    assert evidence["gis"]["live_tile_change"]["after"] == {"x": 0, "z": 0}
    assert evidence["hosting_range"]["evidence"] == "evidence/ewm/range-hosts.json"
    assert "200 whole object" in evidence["hosting_range"]["cloudflare_worker_static_assets"]
    assert "206" in evidence["hosting_range"]["github_pages"]
    assert "Whole-entry GET" in evidence["hosting_range"]["fallback"]
    assert evidence["dual_boot"]["browser_run"] is True
    assert evidence["dual_boot"]["ewm_1"]["upload_triangles"] == 137048
    assert evidence["dual_boot"]["ewm_1"]["asset_count"] == 24
    assert evidence["dual_boot"]["ewm_1"]["camper_parts"] == 16
    assert evidence["dual_boot"]["flag_off_measured"]["fail_closed"] is True
    assert evidence["dual_boot"]["flag_off_measured"]["asset_count"] == 0
    assert evidence["dual_boot"]["flag_off_measured"]["fetched_scene_json"] is False
    assert evidence["dual_boot"]["scene_graph"] == "q42"
    gaps = " ".join(evidence["honest_gaps"])
    assert "pending-producer" not in gaps
    assert "not browser-verified" not in gaps
    assert "streaming.default stays false" in gaps
    assert "later-track" in gaps
    assert "24 first-paint members" in gaps
    assert evidence["art_approval"] is False


def test_download_button_locks_in_flight():
    source = (ROOT / "native/world-host/src/runtime/ui/download.rs").read_text(encoding="utf-8")
    assert "DOWNLOAD_BUSY" in source
    assert "aria-busy" in source
    assert "pause_ms(400)" in source
    assert "OPFS is not yet wired" not in source
    assert 'button.set_text_content(Some("Keep this place offline"))' in source
    assert "if opfs_dir.is_none()" in source
    assert "Nothing was sent to the downloads folder" in source
    assert "Blob `<a download>` is fallback only" in source
