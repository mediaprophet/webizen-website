"""Play mode: observer-to-agent stance, click-to-toolbox, N3 aboutGroup."""
from pathlib import Path

ROOT = Path(__file__).resolve().parents[3]
N3 = ROOT / "data/ewm/camp-sequence.n3"
TTL = ROOT / "data/ewm/camp-sequence.ttl"
CAMP = ROOT / "data/ewm/camp.q42"
MACROS = ROOT / "native/world-host/src/runtime/macros.rs"
PLAY = ROOT / "native/world-host/src/runtime/play.rs"
RUNTIME = ROOT / "native/world-host/src/runtime.rs"
KNOWLEDGE = ROOT / "native/world-host/src/runtime/knowledge.rs"
CSS = ROOT / "site/world/native.css"
LOADER = ROOT / "site/world/loader.css"
PRODUCER = ROOT / "tooling/ewm_camp_q42.py"


def test_play_mode_host_exists_without_camp_rs():
    play = PLAY.read_text(encoding="utf-8")
    runtime = RUNTIME.read_text(encoding="utf-8")
    macros = MACROS.read_text(encoding="utf-8")
    assert PLAY.is_file()
    assert "mod play;" in runtime
    assert "mod camp;" not in runtime
    assert "camp.rs" not in runtime
    assert (ROOT / "native/world-host/src/runtime/camp.rs").exists() is False
    car = 'if id == "car"'
    assert car not in play
    assert car not in macros
    assert "play_mode" in runtime
    assert "native-play-toggle" in play
    assert "native-play-toolbox" in play
    assert "permitted_actions_about" in macros
    assert "aboutGroup" in macros
    assert "steer_keydown_mode" in macros
    assert "focus_world" in macros
    assert "pub(super) fn pick_at" in play
    assert "fn nearest_asset" in play
    assert "fn screen_from_world" in play
    input_rs = (ROOT / "native/world-host/src/runtime/input.rs").read_text(encoding="utf-8")
    assert "play::pick_at" in input_rs
    assert "pending_pick_age" in runtime


def test_play_mode_n3_about_group():
    n3 = N3.read_text(encoding="utf-8")
    ttl = TTL.read_text(encoding="utf-8")
    producer = PRODUCER.read_text(encoding="utf-8")
    assert "rel:aboutGroup" in n3
    assert "action:pack_up" in n3
    assert "action:unhitch" in n3
    assert "group:chassis" in n3
    assert "group:car" in n3
    assert "group:sides" in n3
    assert "group:panels-three" in n3
    assert "group:panels-two" in n3
    assert "aboutGroup" in ttl
    assert "ACTION_ABOUT" in producer
    assert CAMP.is_file()
    assert CAMP.read_bytes()[:3] == b"Q42"


def test_play_mode_sparql_and_css():
    knowledge = KNOWLEDGE.read_text(encoding="utf-8")
    css = CSS.read_text(encoding="utf-8")
    loader = LOADER.read_text(encoding="utf-8")
    assert "actions_about_group_query" in knowledge
    assert "aboutGroup" in knowledge
    assert "#native-play-toggle" in css
    assert "#native-play-toolbox" in css
    assert "native-play-toggle" in loader
    assert "html.cl-loading #native-play-toggle" in loader
