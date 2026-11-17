"""EWM qualification: first-paint catalog and pack identities."""
import hashlib
import json
from pathlib import Path

ROOT = Path(__file__).resolve().parents[3]


def test_first_paint_ids_and_bytes():
    catalog = json.loads((ROOT / "data/ewm/first-paint.json").read_text(encoding="utf-8"))
    assert catalog["declared_subset"]["bytes"] == 14_632_064
    assert len(catalog["first_paint"]) == 10
    assert catalog["streaming"]["default"] is False
    total = 0
    for row in catalog["entries"]:
        if row["scene_id"] not in catalog["first_paint"]:
            continue
        src = ROOT / row["url"].lstrip("/")
        assert src.is_file(), src
        raw = src.read_bytes()
        assert hashlib.sha256(raw).hexdigest() == row["sha256"]
        total += len(raw)
    assert total == 14_632_064


def test_declared_subset_matches_entry_geometry():
    catalog = json.loads((ROOT / "data/ewm/first-paint.json").read_text(encoding="utf-8"))
    ids = set(catalog["first_paint"])
    verts = tris = 0
    for row in catalog["entries"]:
        if row["scene_id"] in ids:
            verts += row["vertices"]
            tris += row["triangles"]
    assert catalog["declared_subset"]["vertices"] == verts
    assert catalog["declared_subset"]["triangles"] == tris


def test_host_source_is_utf8_not_ascii_gated():
    receipt = (ROOT / "tooling/native-host/receipt.py").read_text(encoding="utf-8")
    assert "isascii" not in receipt
    assert "utf-8" in receipt


def test_no_timeline_leak():
    fixtures = [
        ROOT / "data/ewm/first-paint.json",
        ROOT / "packages/current.json",
        ROOT / "packages/download/first-paint.json",
        ROOT / "packages/qbdl/receipt.json",
        ROOT / "data/ewm/gis/places/canberra-public-reference.json",
        ROOT / "data/ewm/frames/canberra-public-reference.json",
        ROOT / "data/ewm/consent/ewm_static.json",
    ]
    for path in fixtures:
        text = path.read_text(encoding="utf-8")
        assert "Timeline.json" not in text, path
        assert "INFERRED_WORK" not in text, path
        assert "placeId" not in text, path
    for folder in (
        ROOT / "data/ewm",
        ROOT / "packages",
        ROOT / "production/lush-woodland/qbdl",
    ):
        for path in folder.rglob("*"):
            assert path.name != "Timeline.json", path


def test_qbdl_magic():
    receipt = json.loads((ROOT / "packages/qbdl/receipt.json").read_text(encoding="utf-8"))
    assert receipt["magic"] == "QBDL"
    woodland = ROOT / "production/lush-woodland/qbdl"
    for row in receipt["entries"]:
        hmc = ROOT / row["hmc"].lstrip("/")
        assert hmc.read_bytes()[:4] == b"QBDL"
        assert hmc.stat().st_size < 24 * 1024 * 1024
        rel = row["hmc"].lstrip("/").split("packages/qbdl/", 1)[1]
        copy = woodland / rel
        assert copy.is_file(), copy
        assert copy.read_bytes() == hmc.read_bytes()


def test_catalog_edges_bind_inside_first_paint():
    catalog = json.loads((ROOT / "data/ewm/first-paint.json").read_text(encoding="utf-8"))
    by_id = {row["entry_id"]: row["scene_id"] for row in catalog["entries"]}
    live = set(catalog["first_paint"])
    assert catalog["edges"]
    for edge in catalog["edges"]:
        assert by_id[edge["from_entry_id"]] in live
        assert by_id[edge["to_entry_id"]] in live
        assert edge["reason"] == "Geometry"


def test_candidate_is_pending_producer_four_tuple():
    catalog = json.loads((ROOT / "data/ewm/first-paint.json").read_text(encoding="utf-8"))
    cand = catalog["candidate"]
    assert len(cand["descriptor_sha384"]) == 96
    assert len(cand["context_sha384"]) == 96
    assert cand["descriptor_sha384"] != "0" * 96
    assert catalog["candidate_origin"]["kind"] == "pending-producer"
    assert catalog["candidate_origin"]["browser_inventory_verifier"] is False
    assert catalog["candidate_origin"]["host_bind"] == "CandidateIdentity::from_parts"
