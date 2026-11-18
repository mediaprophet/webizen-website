"""Compile field-studio camp logic macros to Q42VOL v4 CBOR-LD SuperBlocks.

N3 is the producer RDF (camp-sequence.n3). Turtle is a companion dump for tests
that still inspect TTL. Visitors never read either file. UnifiedVolumeBuilder
writes SuperBlocks of NQuins from triples.json. The woodland host compiles
holographic and deontic NQuins from these facts with QualiaDB
`pack_preset_command_quin` / `compile_norm_quin`, then evaluates them with
`evaluate_predicate_triggers_zero_heap` and `evaluate_deontic_contract`. Spatial
prefixes live in living-world.vibe `camp_rig()`. Changing this volume must not
require a per-scene Rust file.

streaming.default stays false. Four-tuple stays 24. Awning is a split of
the desk organ (Enhancement), not a 25th first-paint identity. hitchRadiusM
defaults to 4 m. Walk stages carry clipId; missing SKL1 is a no-op translation.
"""
from __future__ import annotations

import hashlib
import json
import re
import subprocess
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
OUT_DIR = ROOT / "data/ewm"
HELPER = ROOT / "adapters/qualiadb/q42_web.exe"
BASE = "https://webizen.au/"
REL = BASE + "relation/"
CAMP = BASE + "camp/"
SEQ = CAMP + "field-studio"
SEQ_KIND = CAMP + "Sequence"
STAGE_KIND = CAMP + "Stage"
NORM_KIND = CAMP + "Norm"
GROUP_KIND = CAMP + "Group"
ACTION_KIND = CAMP + "Action"
NUMBER = re.compile(r"^-?(?:0|[1-9]\d*)(?:\.\d+)?(?:[eE][+-]?\d+)?$")

GROUND_Y = 4.4909216892549235

# Authored placements for catalog-only organs (operator). Camper/Passat meshes
# already sit in the woodland frame; the host overlays animation deltas.
# CAD person has feet on y=0, so the origin sits on authored ground.
OPERATOR_Y = GROUND_Y
OPERATOR_XZ = [-19.35, OPERATOR_Y, 26.55]
DESK_AT = [-19.35, GROUND_Y, 27.12]
# Sail-track awning sits desk-local; pack pose retracts the 2.20 m +X roll-out.
AWNING_AT = list(DESK_AT)
HITCH_RADIUS_M = 4.0
GROUP_ORIGIN_ASSET = {
    "operator": "field-studio-operator-opt",
    "desk": "field-studio-operator-desk",
    "awning": "field-studio-operator-awning",
}
GROUP_LABELS = {
    "panels-three": "Three-panel solar array",
    "panels-two": "Two-panel solar array",
    "sides": "Camper sides",
    "roof": "Camper lid",
    "chassis": "Field studio",
    "car": "Car",
    "starlink": "Starlink dish",
    "operator": "Operator",
    "desk": "Desk",
    "awning": "Awning",
}
# Click-to-toolbox: pack/set-up act on the standing camp; unhitch acts on the
# travelling car and the operator who can drive it.
ACTION_ABOUT = {
    "pack_up": [
        "chassis",
        "sides",
        "panels-three",
        "panels-two",
        "roof",
        "awning",
        "desk",
        "starlink",
        "operator",
    ],
    "set_up": [
        "chassis",
        "sides",
        "panels-three",
        "panels-two",
        "roof",
        "awning",
        "desk",
        "starlink",
        "operator",
    ],
    "unhitch": ["car", "operator"],
}

def _place(asset_id: str, translation: list[float]) -> dict:
    return {
        "id": asset_id,
        "translation": list(translation),
        "rotation": [0.0, 0.0, 0.0, 1.0],
        "scale": [1.0, 1.0, 1.0],
    }

AUTHORED_PLACEMENTS = {
    "field-studio-operator-opt": _place("field-studio-operator-opt", OPERATOR_XZ),
    "field-studio-operator-shirt": _place("field-studio-operator-shirt", OPERATOR_XZ),
    "field-studio-operator-shorts": _place("field-studio-operator-shorts", OPERATOR_XZ),
    "field-studio-operator-hair": _place("field-studio-operator-hair", OPERATOR_XZ),
    "field-studio-operator-desk": _place("field-studio-operator-desk", DESK_AT),
    "field-studio-operator-laptop": _place("field-studio-operator-laptop", DESK_AT),
    "field-studio-operator-laptop-lid": _place("field-studio-operator-laptop-lid", DESK_AT),
    "field-studio-operator-awning": _place("field-studio-operator-awning", AWNING_AT),
}


def sha256(path: Path) -> str:
    return hashlib.sha256(path.read_bytes()).hexdigest()


def lit_int(value) -> str:
    return str(int(value))


def lit_float(value) -> str:
    n = float(value)
    if n != n or n in (float("inf"), float("-inf")):
        raise SystemExit("Non-finite camp number")
    return json.dumps(n, ensure_ascii=False)


def ttl_term(term: str) -> str:
    if term.startswith("http://") or term.startswith("https://"):
        return "<" + term + ">"
    if term in ("true", "false", "null") or NUMBER.match(term):
        return term
    return json.dumps(term, ensure_ascii=False)


def n3_term(term: str) -> str:
    prefixes = (
        (CAMP + "action/", "action:"),
        (CAMP + "group/", "group:"),
        (CAMP + "stage/", "stage:"),
        (CAMP + "norm/", "norm:"),
        (REL, "rel:"),
        (CAMP, "camp:"),
        (BASE, "webizen:"),
    )
    if term.startswith("http://") or term.startswith("https://"):
        for iri, prefix in prefixes:
            if term.startswith(iri):
                rest = term[len(iri) :]
                if rest and all(ch.isalnum() or ch in "-_" for ch in rest):
                    return prefix + rest
        return "<" + term + ">"
    if term in ("true", "false", "null") or NUMBER.match(term):
        return term
    return json.dumps(term, ensure_ascii=False)


def n3_dump(triples: list[list[str]]) -> str:
    lines = [
        "@prefix rel: <https://webizen.au/relation/> .",
        "@prefix camp: <https://webizen.au/camp/> .",
        "@prefix action: <https://webizen.au/camp/action/> .",
        "@prefix group: <https://webizen.au/camp/group/> .",
        "@prefix stage: <https://webizen.au/camp/stage/> .",
        "@prefix norm: <https://webizen.au/camp/norm/> .",
        "@prefix webizen: <https://webizen.au/> .",
        "",
    ]
    for row in triples:
        lines.append(" ".join(n3_term(part) for part in row) + " .")
    return "\n".join(lines) + "\n"


def authored_placements() -> dict:
    return {k: dict(v) for k, v in AUTHORED_PLACEMENTS.items()}


def main() -> int:
    triples: list[list[str]] = []

    def fact(s: str, p: str, o: str) -> None:
        triples.append([s, REL + p, o])

    fact(SEQ, "kind", SEQ_KIND)
    fact(SEQ, "schema", "webizen.logic-sequence/v1")
    fact(SEQ, "id", "field-studio")
    fact(SEQ, "label", "Camp")
    fact(SEQ, "presetFamily", "physical_dynamics")
    fact(SEQ, "preset", "spring_settle")
    fact(SEQ, "temporalAlgebra", "Allen")
    fact(SEQ, "rigFunction", "camp_rig")
    fact(SEQ, "gateGroup", "car")
    fact(SEQ, "travelAction", "urn:webizen:camp:travel")
    fact(SEQ, "forbidMessage", "That camp action is not permitted until the hitch is complete.")
    fact(SEQ, "travelMessage", "The field studio is travelling to this place.")
    fact(SEQ, "travelMessageHitched", "The field studio is travelling to this place.")
    fact(
        SEQ,
        "travelMessageUnhitched",
        "The car is travelling to this place. The field studio stays put.",
    )
    fact(SEQ, "hitchRadiusM", lit_float(HITCH_RADIUS_M))
    fact(SEQ, "originX", lit_float(-21.0))
    fact(SEQ, "originY", lit_float(GROUND_Y))
    fact(SEQ, "originZ", lit_float(28.0))

    actions = [
        ("pack_up", "Pack up", "event:pack_up", "Packing the field studio…"),
        ("set_up", "Set up here", "event:set_up", "Setting up camp…"),
        ("unhitch", "Unhitch", "event:unhitch", "Unhitching the car…"),
    ]
    for index, (action_id, label, event, message) in enumerate(actions):
        uri = CAMP + "action/" + action_id
        fact(SEQ, "hasAction", uri)
        fact(uri, "kind", ACTION_KIND)
        fact(uri, "index", lit_int(index))
        fact(uri, "id", action_id)
        fact(uri, "label", label)
        fact(uri, "triggerEvent", event)
        fact(uri, "actionUri", "urn:webizen:camp:" + action_id)
        fact(uri, "statusMessage", message)
        for group_id in ACTION_ABOUT.get(action_id, ()):
            fact(uri, "aboutGroup", CAMP + "group/" + group_id)

    # travelsHitched: whole set moves after hitch. travelsUnhitched: car+operator
    # can leave the standing camp. Desk and awning stay with the chassis.
    # Fold scale is the packed suitcase / slide-in size at fold pose 1.
    # Stow delta is the translation into the tub after fold. Bunk tents
    # (canvas-roof) slide in with the sides, then tuck into the tub. The
    # hard lid drops after sides and stowed panels. Base stays put.
    groups = [
        {
            "id": "panels-three",
            "prefixes": "solar-array-three",
            "dx": 0.2,
            "dy": 0.38,
            "dz": -4.55,
            "sx": 0.36,
            "sy": 1.0,
            "sz": 1.0,
            "hitched": True,
            "unhitched": False,
        },
        {
            "id": "panels-two",
            "prefixes": "solar-array-two",
            "dx": -3.4,
            "dy": 0.38,
            "dz": -4.3,
            "sx": 0.52,
            "sy": 1.0,
            "sz": 1.0,
            "hitched": True,
            "unhitched": False,
        },
        {
            "id": "sides",
            "prefixes": "vintage-windup-camper--canvas,vintage-windup-camper--screen,vintage-windup-camper--canvas-seam,vintage-windup-camper--screen-wire,vintage-windup-camper--canvas-roof",
            "dx": 0.0,
            "dy": -1.18,
            "dz": 0.0,
            "sx": 1.0,
            "sy": 1.0,
            "sz": 0.48,
            "hitched": True,
            "unhitched": False,
        },
        {
            "id": "roof",
            "prefixes": "vintage-windup-camper--roof",
            "dx": 0.0,
            "dy": -1.24,
            "dz": 0.0,
            "sx": 1.0,
            "sy": 1.0,
            "sz": 1.0,
            "hitched": True,
            "unhitched": False,
        },
        {
            "id": "chassis",
            "prefixes": "vintage-windup-camper--",
            "dx": 0.0,
            "dy": 0.0,
            "dz": 0.0,
            "sx": 1.0,
            "sy": 1.0,
            "sz": 1.0,
            "hitched": True,
            "unhitched": False,
        },
        {
            "id": "car",
            "prefixes": "car-passat-b7",
            "dx": -2.35,
            "dy": 0.0,
            "dz": 1.15,
            "sx": 1.0,
            "sy": 1.0,
            "sz": 1.0,
            "hitched": True,
            "unhitched": True,
        },
        {
            "id": "starlink",
            "prefixes": "starlink-mini",
            "dx": -0.7,
            "dy": -2.4,
            "dz": 0.65,
            "sx": 1.0,
            "sy": 1.0,
            "sz": 1.0,
            "hitched": True,
            "unhitched": False,
        },
        {
            "id": "operator",
            "prefixes": "field-studio-operator-opt,field-studio-operator-shirt,field-studio-operator-shorts,field-studio-operator-hair",
            "dx": 0.0,
            "dy": 0.0,
            "dz": 0.0,
            "sx": 1.0,
            "sy": 1.0,
            "sz": 1.0,
            "hitched": True,
            "unhitched": True,
        },
        {
            "id": "desk",
            "prefixes": "field-studio-operator-desk,field-studio-operator-laptop",
            "dx": 0.0,
            "dy": 0.0,
            "dz": 0.0,
            "sx": 1.0,
            "sy": 1.0,
            "sz": 1.0,
            "hitched": True,
            "unhitched": False,
        },
        {
            "id": "awning",
            "prefixes": "field-studio-operator-awning",
            "dx": -2.2,
            "dy": 0.0,
            "dz": 0.0,
            "sx": 1.0,
            "sy": 1.0,
            "sz": 1.0,
            "hitched": True,
            "unhitched": False,
        },
    ]
    for index, group in enumerate(groups):
        group_id = group["id"]
        uri = CAMP + "group/" + group_id
        fact(SEQ, "hasGroup", uri)
        fact(uri, "kind", GROUP_KIND)
        fact(uri, "index", lit_int(index))
        fact(uri, "id", group_id)
        fact(uri, "label", GROUP_LABELS.get(group_id, group_id))
        fact(uri, "prefixes", group["prefixes"])
        fact(uri, "deltaX", lit_float(group["dx"]))
        fact(uri, "deltaY", lit_float(group["dy"]))
        fact(uri, "deltaZ", lit_float(group["dz"]))
        fact(uri, "scaleX", lit_float(group["sx"]))
        fact(uri, "scaleY", lit_float(group["sy"]))
        fact(uri, "scaleZ", lit_float(group["sz"]))
        fact(uri, "travels", "true" if group["hitched"] else "false")
        fact(uri, "travelsHitched", "true" if group["hitched"] else "false")
        fact(uri, "travelsUnhitched", "true" if group["unhitched"] else "false")
        place = AUTHORED_PLACEMENTS.get(GROUP_ORIGIN_ASSET.get(group_id, ""))
        if place:
            fact(uri, "originX", lit_float(place["translation"][0]))
            fact(uri, "originY", lit_float(place["translation"][1]))
            fact(uri, "originZ", lit_float(place["translation"][2]))

    # Pack-up Meets: walk → awning in → bunks/sides slide in → canvas tucks
    # into the tub → fold 3+2 arrays → stow arrays inside → Starlink gone →
    # lid down onto the base → walk → hitch. Set-up is the reverse without
    # walks. Host budgets: 16/32/16/8.
    def stage_row(sid, action, trigger, group, follow, index, duration, goal, clip="", **extra):
        row = {
            "id": sid,
            "action": action,
            "trigger": trigger,
            "group": group,
            "follow": follow,
            "index": index,
            "duration": duration,
            "goal": goal,
            "clip": clip,
        }
        row.update(extra)
        return row

    stages = [
        stage_row("walk-to-panels", "pack_up", "event:pack_up", "operator", "event:stow_awning", 0, 1400, "0", "walk_to_panels"),
        stage_row("stow-awning", "pack_up", "event:stow_awning", "awning", "event:slide_sides", 1, 1600, "1"),
        stage_row("slide-sides", "pack_up", "event:slide_sides", "sides", "event:tuck_sides", 2, 1400, "1", channel="fold"),
        stage_row("tuck-sides", "pack_up", "event:tuck_sides", "sides", "event:fold_panels", 3, 1200, "1"),
        stage_row("fold-panels-three", "pack_up", "event:fold_panels", "panels-three", "event:stow_panels", 4, 1400, "1", channel="fold"),
        stage_row("fold-panels-two", "pack_up", "event:fold_panels", "panels-two", "event:stow_panels", 4, 1400, "1", channel="fold"),
        stage_row("stow-panels-three", "pack_up", "event:stow_panels", "panels-three", "event:stow_starlink", 5, 1600, "1"),
        stage_row("stow-panels-two", "pack_up", "event:stow_panels", "panels-two", "event:stow_starlink", 5, 1600, "1"),
        stage_row("stow-starlink", "pack_up", "event:stow_starlink", "starlink", "event:roof_ready", 6, 1200, "1"),
        stage_row("lower-roof", "pack_up", "event:roof_ready", "roof", "event:walk_to_hitch", 7, 1600, "1"),
        stage_row("walk-to-hitch", "pack_up", "event:walk_to_hitch", "operator", "event:hitch_ready", 8, 1400, "0", "walk_to_hitch"),
        stage_row("hitch", "pack_up", "event:hitch_ready", "car", "", 9, 1800, "1", hitch_radius=True),
        stage_row("open-hitch", "set_up", "event:set_up", "car", "event:raise_roof", 0, 1800, "0"),
        stage_row("raise-roof", "set_up", "event:raise_roof", "roof", "event:raise_sides", 1, 1600, "0"),
        stage_row("raise-sides", "set_up", "event:raise_sides", "sides", "event:deploy_starlink", 2, 1200, "0"),
        stage_row("deploy-starlink", "set_up", "event:deploy_starlink", "starlink", "event:deploy_panels", 3, 1200, "0"),
        stage_row("deploy-panels-three", "set_up", "event:deploy_panels", "panels-three", "event:unfold_panels", 4, 1600, "0"),
        stage_row("deploy-panels-two", "set_up", "event:deploy_panels", "panels-two", "event:unfold_panels", 4, 1600, "0"),
        stage_row("unfold-panels-three", "set_up", "event:unfold_panels", "panels-three", "event:slide_sides_out", 5, 1400, "0", channel="fold"),
        stage_row("unfold-panels-two", "set_up", "event:unfold_panels", "panels-two", "event:slide_sides_out", 5, 1400, "0", channel="fold"),
        stage_row("slide-sides-out", "set_up", "event:slide_sides_out", "sides", "event:deploy_awning", 6, 1400, "0", channel="fold"),
        stage_row("deploy-awning", "set_up", "event:deploy_awning", "awning", "", 7, 1600, "0"),
        stage_row("unhitch", "unhitch", "event:unhitch", "car", "", 0, 1800, "0"),
    ]
    for stage in stages:
        uri = CAMP + "stage/" + stage["id"]
        fact(SEQ, "hasStage", uri)
        fact(uri, "kind", STAGE_KIND)
        fact(uri, "id", stage["id"])
        fact(uri, "action", stage["action"])
        fact(uri, "triggerEvent", stage["trigger"])
        fact(uri, "targetGroup", stage["group"])
        fact(uri, "followUpEvent", stage["follow"])
        fact(uri, "index", lit_int(stage["index"]))
        fact(uri, "durationMs", lit_int(stage["duration"]))
        fact(uri, "temporalOp", "Meets")
        fact(uri, "temporalOpCode", lit_int(2))
        fact(uri, "opcodePlayPreset", lit_int(0x70))
        fact(uri, "blendInMs", lit_int(80))
        fact(uri, "amplitude", lit_int(255))
        fact(uri, "goal", stage["goal"])
        fact(uri, "channel", stage.get("channel", "stow"))
        if stage["clip"]:
            fact(uri, "clipId", stage["clip"])
        if stage.get("hitch_radius"):
            fact(uri, "hitchRadiusM", lit_float(HITCH_RADIUS_M))

    # Deontic macros. Host compiles OP_PERMIT 0x11 / OP_FORBID 0x12 NQuins.
    # Hitch/unhitch travel permits are q42:unless defeaters of forbid-travel.
    # permit-hitch-when-near is q42:unless of forbid-hitch-when-far; action is
    # urn:webizen:camp:hitch so P1 pack_up deontic stays unchanged. Radius is
    # hitchRadiusM on the sequence and hitch stage.
    hitch_action = "urn:webizen:camp:hitch"
    norms = [
        ("permit-pack", "0x11", "urn:webizen:camp:pack_up", False, False),
        ("permit-setup", "0x11", "urn:webizen:camp:set_up", False, False),
        ("permit-unhitch", "0x11", "urn:webizen:camp:unhitch", False, False),
        ("forbid-travel", "0x12", "urn:webizen:camp:travel", False, False),
        ("permit-travel-when-hitched", "0x11", "urn:webizen:camp:travel", True, False),
        ("permit-travel-when-unhitched", "0x11", "urn:webizen:camp:travel", False, True),
        ("forbid-hitch-when-far", "0x12", hitch_action, False, False),
        ("permit-hitch-when-near", "0x11", hitch_action, True, False),
    ]
    for norm_id, opcode, action, hitch, unhitch in norms:
        uri = CAMP + "norm/" + norm_id
        fact(SEQ, "hasNorm", uri)
        fact(uri, "kind", NORM_KIND)
        fact(uri, "id", norm_id)
        fact(uri, "opcode", opcode)
        fact(uri, "action", action)
        fact(uri, "party", "urn:webizen:camp:operator")
        fact(uri, "contract", "urn:webizen:camp:field-studio-contract")
        fact(uri, "isDefeater", "true" if hitch or unhitch else "false")
        fact(uri, "requiresHitch", "true" if hitch else "false")
        fact(uri, "requiresUnhitch", "true" if unhitch else "false")
        if norm_id in ("permit-hitch-when-near", "forbid-hitch-when-far"):
            fact(uri, "hitchRadiusM", lit_float(HITCH_RADIUS_M))
    fact(
        CAMP + "norm/forbid-hitch-when-far",
        "unless",
        CAMP + "norm/permit-hitch-when-near",
    )

    triples = sorted({tuple(row) for row in triples})
    triples = [list(row) for row in triples]
    triple_path = OUT_DIR / "camp-triples.json"
    triple_path.write_text(
        json.dumps(triples, ensure_ascii=False, indent=2) + "\n", encoding="utf-8"
    )
    ttl = "\n".join(" ".join(ttl_term(part) for part in row) + " ." for row in triples) + "\n"
    (OUT_DIR / "camp-sequence.ttl").write_text(ttl, encoding="utf-8")
    (OUT_DIR / "camp-sequence.n3").write_text(n3_dump(triples), encoding="utf-8")

    digest = hashlib.sha256(
        json.dumps(triples, ensure_ascii=False, separators=(",", ":")).encode("utf-8")
    ).hexdigest()
    named = OUT_DIR / f"camp-{digest[:16]}.q42"
    served = OUT_DIR / "camp.q42"
    if not named.exists():
        if not HELPER.exists():
            raise SystemExit(f"Missing Q42 helper {HELPER}")
        run = subprocess.run(
            [str(HELPER), "build", str(triple_path), str(named)],
            capture_output=True,
            text=True,
            encoding="utf-8",
            timeout=90,
        )
        if run.returncode:
            raise SystemExit(run.stderr.strip() or run.stdout.strip() or "q42_web build failed")
        print(run.stdout.strip())
    served.write_bytes(named.read_bytes())
    inspect = subprocess.run(
        [str(HELPER), "inspect", str(served)],
        capture_output=True,
        text=True,
        encoding="utf-8",
        timeout=90,
    )
    if inspect.returncode:
        raise SystemExit(inspect.stderr.strip() or inspect.stdout.strip() or "q42_web inspect failed")
    loaded = json.loads(inspect.stdout)
    if loaded["total_quins"] != len(triples):
        raise SystemExit("Q42 quin count differs from authored camp triples")
    print(
        json.dumps(
            {
                "q42": str(served.relative_to(ROOT)).replace("\\", "/"),
                "sha256": sha256(served),
                "bytes": served.stat().st_size,
                "quins": loaded["total_quins"],
                "triples": len(triples),
                "n3": "data/ewm/camp-sequence.n3",
                "ttl": "data/ewm/camp-sequence.ttl",
                "encoding": "q42vol-v4-cbor-ld",
                "engine": "holographic_scene + deontic",
                "streaming_default": False,
                "hitchRadiusM": HITCH_RADIUS_M,
                "groups": [row["id"] for row in groups],
                "stages": [row["id"] for row in stages],
                "norms": [row[0] for row in norms],
                "actions": [row[0] for row in actions],
                "clipIds": [row["clip"] for row in stages if row["clip"]],
                "counts": {
                    "groups": len(groups),
                    "stages": len(stages),
                    "norms": len(norms),
                    "actions": len(actions),
                },
                "max_host": {
                    "groups": 16,
                    "stages": 32,
                    "norms": 16,
                    "actions": 8,
                },
            },
            indent=2,
        )
    )
    return 0


if __name__ == "__main__":
    sys.exit(main())
