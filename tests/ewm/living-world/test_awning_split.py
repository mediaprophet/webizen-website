"""LW-03: sail-track awning is its own organ; desk no longer carries the sail."""
import json
from pathlib import Path

ROOT = Path(__file__).resolve().parents[3]
VIBE = ROOT / "site/world/living-world.vibe"
RECIPE = ROOT / "production/native-world/recipes/field-studio-operator.vibe"
WORKING = ROOT / "production/native-world/working/field-studio-operator"
RUNTIME = ROOT / "native/world-host/src/runtime.rs"
HOST_SRC = ROOT / "native/world-host/src"
FIRST_PAINT = ROOT / "data/ewm/first-paint.json"
COMBINED_DESK_BYTES = 357544


def _camp_rig(text: str) -> str:
    start = text.index("pure fn camp_rig()")
    end = text.index("pure fn chapter_settings", start)
    return text[start:end]


def test_camp_rig_lists_awning_and_dual_travel_flags():
    vibe = VIBE.read_text(encoding="utf-8")
    rig = _camp_rig(vibe)
    assert 'id: "awning"' in rig
    assert "field-studio-operator-awning" in rig
    assert "hitch_radius_m" in rig
    assert "travels_hitched: true" in rig
    assert "travels_unhitched: true" in rig
    assert "travels_unhitched: false" in rig
    assert 'id: "unhitch"' in rig
    assert 'label: "Unhitch"' in rig


def test_recipe_splits_awning_and_skips_vertebrate_compile():
    recipe = RECIPE.read_text(encoding="utf-8")
    assert "compile_vertebrate_anatomy_10d" in recipe
    assert recipe.count("compile_vertebrate_anatomy_10d") == 1
    assert "Do not call compile_vertebrate_anatomy_10d" in recipe
    assert 'key: "field-studio-operator-awning.10d"' in recipe
    assert "awning.container_10d" in recipe
    desk = recipe[
        recipe.index('title: "Folding timber table, camp chair"') : recipe.index(
            'title: "Door-side sail-track awning"'
        )
    ]
    awning = recipe[
        recipe.index('title: "Door-side sail-track awning"') : recipe.index("let base_p")
    ]
    assert "keder" not in desk
    assert "canvas" not in desk
    assert "pole_0" not in desk
    assert "keder" in awning
    assert "canvas" in awning
    assert "pole_0" in awning
    canvas_loft = recipe[recipe.index("let canvas =") : recipe.index("let desk =")]
    assert "hypermedia.manifold.loft" in canvas_loft


def test_desk_is_not_the_only_awning_carrier():
    orig_desk = WORKING / "originals" / "field-studio-operator-desk.10d"
    orig_awn = WORKING / "originals" / "field-studio-operator-awning.10d"
    att_desk = WORKING / "attested" / "field-studio-operator-desk.10d"
    att_awn = WORKING / "attested" / "field-studio-operator-awning.10d"
    assert orig_desk.is_file()
    assert orig_awn.is_file()
    assert att_desk.is_file()
    assert att_awn.is_file()
    assert orig_desk.stat().st_size < COMBINED_DESK_BYTES
    assert orig_awn.stat().st_size > 1024
    assert att_awn.stat().st_size > orig_awn.stat().st_size
    assert att_desk.stat().st_size > orig_desk.stat().st_size
    receipt = WORKING / "lw03-produce.json"
    assert receipt.is_file()
    text = receipt.read_text(encoding="utf-8")
    assert "awning" in text
    assert "attested_sha256" in text
    assert "Timeline" not in text
    assert "INFERRED_WORK" not in text


def test_no_camp_rs_and_first_paint_stays_24():
    runtime = RUNTIME.read_text(encoding="utf-8")
    assert "mod camp;" not in runtime
    assert "camp.rs" not in runtime
    assert not (HOST_SRC / "camp.rs").exists()
    assert not (HOST_SRC / "runtime" / "camp.rs").exists()
    paint = json.loads(FIRST_PAINT.read_text(encoding="utf-8"))
    assert len(paint["first_paint"]) == 24
    assert paint["streaming"]["default"] is False
