"""Native InventoryVerifier four-tuple copied into catalog and receipt."""
import json
from pathlib import Path

ROOT = Path(__file__).resolve().parents[3]
ZERO = "0" * 96
SIDECAR = ROOT / "packages/qbdl/inventory-identity.json"
WOODLAND_SIDECAR = ROOT / "production/lush-woodland/qbdl/inventory-identity.json"
CATALOG = ROOT / "data/ewm/first-paint.json"
RECEIPT = ROOT / "packages/qbdl/receipt.json"


def _nonzero_sha384(text):
    assert isinstance(text, str)
    assert len(text) == 96
    bytes.fromhex(text)
    assert text != ZERO


def test_sidecar_inventory_verifier_finish():
    sidecar = json.loads(SIDECAR.read_text(encoding="utf-8"))
    assert sidecar["schema"] == "webizen.ewm-inventory-identity/v1"
    assert sidecar["status"] == "inventory-verifier-finish"
    cand = sidecar["candidate"]
    _nonzero_sha384(cand["descriptor_sha384"])
    _nonzero_sha384(cand["context_sha384"])
    assert isinstance(cand["generation"], int)
    assert cand["generation"] >= 0
    assert cand["entries"] == 24
    assert sidecar["streaming_default"] is False
    assert sidecar["browser_inventory_verifier"] is False
    assert sidecar["host_bind"] == "CandidateIdentity::from_parts"
    assert sidecar["inner_ranges"] is False


def test_catalog_copies_sidecar_candidate():
    sidecar = json.loads(SIDECAR.read_text(encoding="utf-8"))
    catalog = json.loads(CATALOG.read_text(encoding="utf-8"))
    assert catalog["candidate"] == sidecar["candidate"]
    origin = catalog["candidate_origin"]
    assert origin["kind"] == "inventory-verifier-finish"
    assert origin["host_bind"] == "CandidateIdentity::from_parts"
    assert origin["browser_inventory_verifier"] is False
    assert catalog["streaming"]["default"] is False
    assert catalog["inner_ranges"] is False
    assert len(catalog["first_paint"]) == 24
    assert catalog["windowed_inventory"]["candidate_entry_count"] == 24
    assert catalog["windowed_inventory"]["first_paint_window"] == 24


def test_receipt_matches_sidecar_when_present():
    sidecar = json.loads(SIDECAR.read_text(encoding="utf-8"))
    receipt = json.loads(RECEIPT.read_text(encoding="utf-8"))
    assert receipt["candidate"] == sidecar["candidate"]
    assert receipt["candidate_origin"]["kind"] == "inventory-verifier-finish"
    assert receipt["candidate_origin"]["host_bind"] == "CandidateIdentity::from_parts"
    assert receipt["candidate_origin"]["browser_inventory_verifier"] is False


def test_woodland_sidecar_copy():
    assert WOODLAND_SIDECAR.read_bytes() == SIDECAR.read_bytes()
