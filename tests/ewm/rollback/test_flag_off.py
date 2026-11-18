"""Flag off fails closed; woodland no longer whole-file loads scene.json."""
from pathlib import Path

ROOT = Path(__file__).resolve().parents[3]


def test_boot_fails_closed_without_ewm_on_q42_scene():
    boot = (ROOT / "native/world-host/src/runtime/boot.rs").read_text(encoding="utf-8")
    assert "ewm_streaming" in boot
    assert "ewm_query_enabled" in boot
    assert "gpu-timing=1" in boot
    assert "scene.q42" in boot
    assert "The woodland streams when you add ?ewm=1" in boot
    assert "/production/lush-woodland/scene.json" not in boot
    assert "/publication/current.json" not in boot
    assert "/data/ewm/catalog.q42" in boot
    assert "bytes(&a.url, 64 * 1024 * 1024)" in boot


def test_homepage_points_at_scene_q42():
    html = (ROOT / "site/index.html").read_text(encoding="utf-8")
    assert 'data-scene="/production/lush-woodland/scene.q42"' in html
    assert "scene.json" not in html
    assert "scene-mobile.json" not in html


def test_flag_helper_documents_rollback():
    flag = (ROOT / "native/world-host/src/runtime/stream/flag.rs").read_text(encoding="utf-8")
    assert "streaming.default false" in flag
    assert "reading view" in flag
    md = (ROOT / "tooling/ewm_flag.md").read_text(encoding="utf-8")
    assert "streaming.default" in md
    assert "?ewm=1" in md
    assert "fails closed" in md
    assert "scene.json" in md


def test_receipt_refuses_replacement_character_not_ascii():
    receipt = (ROOT / "tooling/native-host/receipt.py").read_text(encoding="utf-8")
    assert "isascii" not in receipt
    assert "\\ufffd" in receipt or "\ufffd" in receipt
    assert "utf-8" in receipt
