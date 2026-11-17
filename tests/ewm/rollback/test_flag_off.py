"""Flag off keeps whole-file boot; ewm=1 is independent of gpu-timing."""
from pathlib import Path

ROOT = Path(__file__).resolve().parents[3]


def test_boot_still_has_whole_file_path():
    boot = (ROOT / "native/world-host/src/runtime/boot.rs").read_text(encoding="utf-8")
    assert "ewm_streaming" in boot
    assert "bytes(&a.url, 64 * 1024 * 1024)" in boot
    assert "gpu-timing=1" in boot
    assert "ewm_query_enabled" in boot


def test_flag_helper_documents_rollback():
    flag = (ROOT / "native/world-host/src/runtime/stream/flag.rs").read_text(encoding="utf-8")
    assert "streaming.default false" in flag
    md = (ROOT / "tooling/ewm_flag.md").read_text(encoding="utf-8")
    assert "streaming.default" in md
    assert "?ewm=1" in md


def test_receipt_refuses_replacement_character_not_ascii():
    receipt = (ROOT / "tooling/native-host/receipt.py").read_text(encoding="utf-8")
    assert "isascii" not in receipt
    assert "\\ufffd" in receipt or "\ufffd" in receipt
    assert "utf-8" in receipt
