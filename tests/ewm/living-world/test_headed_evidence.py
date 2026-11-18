"""LW-09: headed evidence PNGs, wasm sha under 23.5 MiB, no art claim."""
import hashlib
import json
from pathlib import Path

ROOT = Path(__file__).resolve().parents[3]
EVIDENCE = ROOT / "evidence/ewm/living-world"
LEDGER = EVIDENCE / "headed.json"
WASM = ROOT / "site/pkg/native-world/webizen_world_bg.wasm"
IDENTITY = ROOT / "site/pkg/native-world/runtime-identity.json"
BUDGET_BYTES = 24641536
BUDGET_MIB = 23.5
SHOTS = ("hitch-river.png", "unhitch-river.png", "place-unload.png")


def test_headed_screenshots_exist():
    for name in SHOTS:
        path = EVIDENCE / name
        assert path.is_file(), path
        assert path.stat().st_size > 1024, path


def test_headed_ledger_records_wasm_and_refuses_art_acceptance():
    assert LEDGER.is_file(), LEDGER
    ledger = json.loads(LEDGER.read_text(encoding="utf-8"))
    assert ledger["schema"] == "webizen.living-world-evidence/v1"
    assert ledger["locale"] == "en-AU"
    assert ledger["art_approval"] is False
    assert ledger["streaming_default"] is False
    assert ledger["inner_ranges"] is False
    assert ledger["browser_ran"] is True
    wasm = ledger["wasm"]
    assert wasm["ci_budget_mib"] == BUDGET_MIB
    assert wasm["ci_budget_bytes"] == BUDGET_BYTES
    assert wasm["bytes"] <= BUDGET_BYTES
    assert wasm["under_budget"] is True
    raw = WASM.read_bytes()
    digest = hashlib.sha256(raw).hexdigest()
    ident = json.loads(IDENTITY.read_text(encoding="utf-8"))
    assert wasm["sha256"] == digest
    assert wasm["bytes"] == len(raw)
    assert ident["wasm"]["sha256"] == digest
    assert ident["wasm"]["bytes"] == len(raw)
    shots = ledger["screenshots"]
    assert (ROOT / shots["hitch_river"]).is_file()
    assert (ROOT / shots["unhitch_river"]).is_file()
    assert (ROOT / shots["place_unload"]).is_file()
    blob = json.dumps(ledger)
    assert "Timeline" not in blob
    assert "INFERRED_WORK" not in blob
    assert "art complete" not in blob.lower()
    assert "art accepted" not in blob.lower()


def test_does_not_flip_streaming_default():
    catalog = json.loads((ROOT / "data/ewm/first-paint.json").read_text(encoding="utf-8"))
    pointer = json.loads((ROOT / "packages/current.json").read_text(encoding="utf-8"))
    index = json.loads((ROOT / "data/ewm/place-index.json").read_text(encoding="utf-8"))
    flag = (ROOT / "native/world-host/src/runtime/stream/flag.rs").read_text(
        encoding="utf-8"
    )
    ledger = json.loads(LEDGER.read_text(encoding="utf-8"))
    assert catalog["streaming"]["default"] is False
    assert catalog["inner_ranges"] is False
    assert len(catalog["first_paint"]) == 24
    assert pointer["streaming"]["default"] is False
    assert index["streaming"]["default"] is False
    assert index["inner_ranges"] is False
    assert "Leave streaming.default false" in flag
    assert ledger["streaming_default"] is False
    assert ledger["inner_ranges"] is False
    assert "streaming.default = true" not in flag
