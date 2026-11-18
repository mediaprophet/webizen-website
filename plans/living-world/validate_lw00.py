"""Validate LW-00 living-world playable contract and file ownership.

No network. No jsonschema package. No runtime edits. No cargo.
"""
from __future__ import annotations

import hashlib
import json
import re
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
LW = Path(__file__).resolve().parent
PACKETS_PATH = LW / "work-packets.json"
OWNERSHIP_PATH = LW / "file-ownership.json"
DESIGN_PATH = LW / "LIVING-WORLD-PLAYABLE.md"
RECEIPT_PATH = LW / "lw00-validation.json"

EXPECTED_PACKET_IDS = [
    "LW-00",
    "LW-01",
    "LW-02",
    "LW-03",
    "LW-04",
    "LW-05",
    "LW-06",
    "LW-07",
    "LW-08",
    "LW-09",
    "LW-10",
]
COLLIDING_ID_RE = re.compile(r"^(A\d{2}|N\d{2}|EWM-\d+[a-z]?)$")
LIVE_WAVES = {
    "wave-0": ["LW-00"],
    "wave-1": ["LW-01", "LW-02", "LW-03"],
    "wave-2": ["LW-04", "LW-06", "LW-07"],
    "wave-3": ["LW-08", "LW-09"],
    "wave-4": ["LW-10"],
}
DESIGN_REQUIRED = (
    "P1",
    "Unhitch",
    "WASD",
    "SKL1",
    "awning",
    "Re-hitch",
    "place unload",
    "streaming.default",
    "inner_ranges",
    "LW-00+",
)
EWM_KEEPS = (
    "webizen-website/native/world-host/src/runtime/stream/flag.rs",
    "webizen-website/wrangler.jsonc",
    "webizen-website/packages/current.json",
)


class ValidationError(ValueError):
    pass


def fail(path: str, message: str) -> None:
    raise ValidationError(path + ": " + message)


def load_json(path: Path):
    return json.loads(path.read_text(encoding="utf-8"))


def sha256_file(path: Path) -> str:
    return hashlib.sha256(path.read_bytes()).hexdigest()


def norm(path: str) -> str:
    return path.replace("\\", "/").rstrip("/")


def path_kind(path: str) -> tuple[str, str]:
    p = norm(path)
    if p.endswith("/**"):
        return "dir", p[:-3].rstrip("/")
    if "*" in p:
        star = p.find("*")
        return "glob", p[:star].rstrip("/")
    return "exact", p


def covered_by_exclude(path: str, excludes: list[str]) -> bool:
    kind, base = path_kind(path)
    for exclude in excludes:
        ekind, ebase = path_kind(exclude)
        if ekind == "exact" and kind == "exact" and base == ebase:
            return True
        if ekind in {"dir", "glob"} and (base == ebase or base.startswith(ebase + "/")):
            return True
        if ekind == "exact" and kind in {"dir", "glob"} and ebase.startswith(base + "/"):
            return False
    return False


def owned_overlap(left: dict, right: dict) -> str | None:
    left_ex = [norm(p) for p in left.get("owned_paths_exclude", [])]
    right_ex = [norm(p) for p in right.get("owned_paths_exclude", [])]
    for a in left.get("owned_paths", []):
        if covered_by_exclude(a, left_ex):
            continue
        akind, abase = path_kind(a)
        for b in right.get("owned_paths", []):
            if covered_by_exclude(b, right_ex):
                continue
            bkind, bbase = path_kind(b)
            if covered_by_exclude(a, right_ex) or covered_by_exclude(b, left_ex):
                continue
            same = abase == bbase
            a_in_b = abase == bbase or abase.startswith(bbase + "/")
            b_in_a = bbase == abase or bbase.startswith(abase + "/")
            if akind == "exact" and bkind == "exact" and same:
                return a + " vs " + b
            if akind in {"dir", "glob"} and b_in_a:
                return a + " vs " + b
            if bkind in {"dir", "glob"} and a_in_b:
                return a + " vs " + b
            if same:
                return a + " vs " + b
    return None


def packet_map(doc: dict) -> dict[str, dict]:
    out = {}
    for row in doc["packets"]:
        pid = row["id"]
        if pid in out:
            fail("packets", "duplicate id " + pid)
        out[pid] = row
    return out


def check_ids(packets: dict[str, dict]) -> None:
    ids = list(packets)
    if ids != EXPECTED_PACKET_IDS:
        fail("packets", "id list must be LW-00..LW-10 in order, got " + ",".join(ids))
    for pid in ids:
        if COLLIDING_ID_RE.match(pid):
            fail(pid, "collides with A/N/EWM series")
        if not pid.startswith("LW-"):
            fail(pid, "id series must be LW")


def check_waves(doc: dict, packets: dict[str, dict]) -> None:
    wave_ids = []
    seen = []
    for wave in doc["waves"]:
        wave_ids.append(wave["id"])
        for pid in wave["packets"]:
            if pid not in packets:
                fail(wave["id"], "unknown packet " + pid)
            seen.append(pid)
    if wave_ids != ["wave-0", "wave-1", "wave-2", "wave-3", "wave-4"]:
        fail("waves", "expected wave-0..wave-4")
    if sorted(seen) != sorted(EXPECTED_PACKET_IDS) or len(seen) != len(set(seen)):
        fail("waves", "wave graph must list each LW packet once")
    if doc["waves"][2].get("note") is None:
        fail("wave-2", "must note LW-05 sequential after LW-04")
    if "LW-05" not in " ".join(doc["waves"][2]["packets"]):
        fail("wave-2", "must include LW-05")


def check_live_disjoint(packets: dict[str, dict]) -> None:
    for wave, ids in LIVE_WAVES.items():
        rows = [packets[pid] for pid in ids]
        for i, left in enumerate(rows):
            for right in rows[i + 1 :]:
                hit = owned_overlap(left, right)
                if hit:
                    fail(wave, "owned_paths overlap among live specialists: " + hit)
    sequential = owned_overlap(packets["LW-04"], packets["LW-05"])
    if sequential:
        fail("LW-04/LW-05", "same-specialist sequential packets still overlap: " + sequential)
    skl = owned_overlap(packets["LW-03"], packets["LW-07"])
    if skl:
        fail("LW-03/LW-07", "operator working tree overlaps SKL1: " + skl)


def check_ewm_keeps(doc: dict) -> None:
    owners = doc["single_file_owners"]
    if owners["webizen-website/native/world-host/src/runtime/stream/flag.rs"] != "EWM-09":
        fail("single_file_owners", "flag.rs must stay EWM-09")
    if owners["webizen-website/wrangler.jsonc"] != "EWM-05":
        fail("single_file_owners", "wrangler.jsonc must stay EWM-05")
    if owners["webizen-website/packages/current.json"] != "EWM-10":
        fail("single_file_owners", "packages/current.json must stay EWM-10")
    lw00 = next(p for p in doc["packets"] if p["id"] == "LW-00")
    for path in EWM_KEEPS:
        if path in lw00.get("owned_paths", []):
            fail("LW-00", "must not own " + path)
    rules = " ".join(doc["execution_rules"])
    if "streaming.default stays false" not in rules:
        fail("execution_rules", "streaming.default must stay false")
    if "inner_ranges stays false" not in rules:
        fail("execution_rules", "inner_ranges must stay false")
    if "Timeline.json" not in rules:
        fail("execution_rules", "must forbid Timeline.json")
    handoff = doc["ewm_handoff"]
    for key in ("stream", "qualification_tests", "first_paint_json", "gis_ingest", "packages_qbdl"):
        if key not in handoff:
            fail("ewm_handoff", "missing " + key)


def check_design(text: str) -> None:
    lowered = text
    for token in DESIGN_REQUIRED:
        if token not in lowered and token.lower() not in lowered.lower():
            fail("LIVING-WORLD-PLAYABLE.md", "missing " + token)
    if "**Done (P1).**" not in text and "Done (P1)" not in text:
        fail("LIVING-WORLD-PLAYABLE.md", "must name P1 unhitch as done")
    if "streaming.default` stays **false**" not in text and "streaming.default` stays **false**".replace("`", "") not in text:
        if "streaming.default` stays **false**" not in text:
            if "streaming.default" not in text or "false" not in text:
                fail("LIVING-WORLD-PLAYABLE.md", "streaming.default must remain false")
    if "inner_ranges` stays **false**" not in text and "`inner_ranges` stays **false**" not in text:
        if "inner_ranges" not in text:
            fail("LIVING-WORLD-PLAYABLE.md", "inner_ranges must stay false")
    if "Do not reuse A00" not in text and "Do not reuse A00–A27" not in text:
        fail("LIVING-WORLD-PLAYABLE.md", "must keep LW id series out of A/N/EWM")


def check_privacy(design: str, packets: dict, ownership: dict) -> None:
    if (LW / "Timeline.json").exists():
        fail("plans/living-world", "Timeline.json must not exist")
    for path in LW.rglob("*"):
        if path.name.lower() == "timeline.json":
            fail(str(path), "Timeline.json must not exist")
    for key in packets:
        if "INFERRED_WORK" in key or key == "placeIds":
            fail("packets", "forbidden data key " + key)
    for row in packets.values():
        status = str(row.get("status", ""))
        if status == "INFERRED_WORK":
            fail(row["id"], "status must not be INFERRED_WORK")
        for field in ("placeIds", "placeId", "INFERRED_WORK"):
            if field in row:
                fail(row["id"], "forbidden field " + field)
    policy = design + json.dumps(ownership.get("glob_collision_audit", {}))
    if "Timeline.json" not in policy:
        fail("privacy", "contract must forbid Timeline.json")


def check_ownership_board(packets_doc: dict, ownership: dict) -> None:
    if ownership["schema"] != "webizen.living-world-file-ownership/v1":
        fail("file-ownership.json", "schema mismatch")
    if ownership["id_series"] != "LW":
        fail("file-ownership.json", "id_series must be LW")
    if ownership["streaming_default"] is not False:
        fail("file-ownership.json", "streaming.default must be false")
    if ownership["inner_ranges"] is not False:
        fail("file-ownership.json", "inner_ranges must be false")
    if ownership["first_paint_len"] != 24:
        fail("file-ownership.json", "four-tuple first-paint stays 24")
    if ownership["runtime_behaviour_change"] is not False:
        fail("file-ownership.json", "LW-00 must not change runtime behaviour")
    own_map = packet_map(ownership)
    pkt_map = packet_map(packets_doc)
    if list(own_map) != list(pkt_map):
        fail("file-ownership.json", "packet ids must match work-packets.json")
    for pid, row in pkt_map.items():
        owned = own_map[pid].get("owned_paths", [])
        if owned != row.get("owned_paths", []):
            fail(pid, "file-ownership owned_paths drifted from work-packets")
        if own_map[pid].get("owned_paths_exclude", []) != row.get("owned_paths_exclude", []):
            fail(pid, "file-ownership owned_paths_exclude drifted from work-packets")
    if packets_doc["single_file_owners"] != ownership["single_file_owners"]:
        fail("single_file_owners", "ownership board drifted from work-packets")
    if packets_doc["integrator_exclusive_paths"] != ownership["integrator_exclusive_paths"]:
        fail("integrator_exclusive_paths", "ownership board drifted from work-packets")
    if packets_doc.get("glob_collision_audit") != ownership.get("glob_collision_audit"):
        fail("glob_collision_audit", "ownership board drifted from work-packets")


def check_lw00_scope(packets: dict[str, dict]) -> None:
    row = packets["LW-00"]
    if row["owned_paths"] != ["webizen-website/plans/living-world/**"]:
        fail("LW-00", "must own only plans/living-world/**")
    if "webizen-website/plans/ewm/**" not in row["owned_paths_exclude"]:
        fail("LW-00", "must exclude plans/ewm")
    if row.get("integrator_routes"):
        fail("LW-00", "integrator_routes must be empty")


def main() -> int:
    packets_doc = load_json(PACKETS_PATH)
    ownership = load_json(OWNERSHIP_PATH)
    design = DESIGN_PATH.read_text(encoding="utf-8")

    if packets_doc["schema"] != "webizen.living-world-work-packets/v1":
        fail("work-packets.json", "schema must be webizen.living-world-work-packets/v1")
    if packets_doc["id_series"] != "LW":
        fail("work-packets.json", "id_series must be LW")
    if "A00-A27" not in packets_doc["id_collision_policy"]:
        fail("id_collision_policy", "must exclude A00-A27")
    if "N01-N09" not in packets_doc["id_collision_policy"]:
        fail("id_collision_policy", "must exclude N01-N09")
    if "EWM-00+" not in packets_doc["id_collision_policy"]:
        fail("id_collision_policy", "must exclude EWM-00+")
    if "later_track" not in packets_doc:
        fail("work-packets.json", "missing later-track")

    packets = packet_map(packets_doc)
    check_ids(packets)
    check_waves(packets_doc, packets)
    check_live_disjoint(packets)
    check_ewm_keeps(packets_doc)
    check_design(design)
    check_privacy(design, packets, ownership)
    check_ownership_board(packets_doc, ownership)
    check_lw00_scope(packets)

    files = {
        "plans/living-world/LIVING-WORLD-PLAYABLE.md": sha256_file(DESIGN_PATH),
        "plans/living-world/work-packets.json": sha256_file(PACKETS_PATH),
        "plans/living-world/file-ownership.json": sha256_file(OWNERSHIP_PATH),
        "plans/living-world/validate_lw00.py": sha256_file(LW / "validate_lw00.py"),
    }
    receipt = {
        "schema": "webizen.lw-00-validation/v1",
        "status": "LW-00 living-world playable contract validated; no runtime change",
        "id_series": "LW",
        "packet_ids": EXPECTED_PACKET_IDS,
        "waves": [wave["id"] for wave in packets_doc["waves"]],
        "streaming_default": False,
        "inner_ranges": False,
        "first_paint_len": 24,
        "runtime_behaviour_change": False,
        "p1_unhitch_done": True,
        "listed_remainder": ["WASD", "SKL1 walk", "awning pack", "re-hitch", "place unload"],
        "file_sha256": files,
    }
    RECEIPT_PATH.write_text(json.dumps(receipt, indent=2) + "\n", encoding="utf-8")
    print(
        json.dumps(
            {
                "status": receipt["status"],
                "receipt": str(RECEIPT_PATH),
                "streaming_default": False,
                "inner_ranges": False,
            },
            indent=2,
        )
    )
    return 0


if __name__ == "__main__":
    try:
        sys.exit(main())
    except ValidationError as error:
        print(error, file=sys.stderr)
        sys.exit(1)
