"""Origin Private File System wiring in download.rs; pack identities stay catalog SHA-256s."""
import json
from pathlib import Path

ROOT = Path(__file__).resolve().parents[3]


def test_download_rs_wires_opfs_via_reflect_not_superblock():
    source = (ROOT / "native/world-host/src/runtime/ui/download.rs").read_text(
        encoding="utf-8"
    )
    assert "js_sys::Reflect" in source
    assert "getDirectory" in source
    assert "navigator" in source
    assert "storage" in source
    assert "ewm" in source
    assert "by-sha256" in source
    assert "write_opfs_identity" in source
    assert "write_opfs_block(" not in source
    assert "pause_ms(400)" in source
    assert "DOWNLOAD_BUSY" in source
    assert "aria-busy" in source
    assert "Origin Private File System is unavailable in this browser" in source
    assert "if opfs_dir.is_none()" in source
    assert "Nothing was sent to the downloads folder" in source
    assert "Keep this place offline" in source


def test_download_pack_identities_are_catalog_sha256_subset():
    catalog = json.loads((ROOT / "data/ewm/first-paint.json").read_text(encoding="utf-8"))
    pack_path = ROOT / "packages/download/first-paint.json"
    assert pack_path.is_file()
    pack = json.loads(pack_path.read_text(encoding="utf-8"))
    catalog_sha = {row["sha256"] for row in catalog["entries"]}
    pack_sha = {row["sha256"] for row in pack["identities"]}
    assert pack_sha
    assert pack_sha <= catalog_sha
    assert pack["streaming_default"] is False


def test_opfs_browser_verified_from_live_run():
    evidence = json.loads(
        (ROOT / "evidence/ewm/opfs-browser.json").read_text(encoding="utf-8")
    )
    assert evidence["schema"] == "webizen.ewm-opfs-browser/v1"
    assert evidence["locale"] == "en-AU"
    assert evidence["browser_verified"] is True
    assert evidence["path"] == "ewm/by-sha256/<sha256>"
    assert evidence["write_opfs_block"] is False
    assert evidence["streaming_default"] is False
    assert evidence["fallback_used"] is False
    assert evidence["blob_save_kept"] is False
    assert "Nothing was sent to the downloads folder" in evidence["status_text"]
    names = evidence["opfs_names"]
    expected = evidence["expected_sha256"]
    assert len(names) == 24
    assert len(expected) == 24
    assert set(names) == set(expected)
    pack = json.loads((ROOT / "packages/download/first-paint.json").read_text(encoding="utf-8"))
    pack_sha = {row["sha256"] for row in pack["identities"]}
    assert set(names) == pack_sha
    spot = evidence["spot_check"]
    assert spot["opfs_bytes"] == spot["catalog_bytes"]
    assert (ROOT / evidence["screenshot"]).is_file()
