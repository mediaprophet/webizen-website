"""Attach SKL1 walk clips to the field-studio operator organ.

Reads the attested operator-opt .10d, attaches named motor clips
walk_to_panels and walk_to_hitch via QualiaDB container_10d skeletal
attach, writes a working copy, then attests. Motor clips only.

Player clipId (LW-01/LW-02) is q_hash of those names. Missing SKL1 is a
no-op translation. prefers-reduced-motion skips clips and keeps the
translation stages.

Do not call compile_vertebrate_anatomy_10d. Do not copy into
production/native-world/assets (integrator). Operator stays Enhancement;
four-tuple first-paint stays 24. inner_ranges stays false.
"""
from __future__ import annotations

import hashlib
import json
import struct
import subprocess
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
NATIVE = ROOT / "production/native-world"
WORKING = NATIVE / "working/field-studio-operator"
SKL1_DIR = WORKING / "skl1"
ORIGINAL = WORKING / "originals/field-studio-operator-opt.10d"
ATTESTED_SRC = NATIVE / "assets/field-studio-operator-opt.10d"
PROVENANCE = NATIVE / "provenance/field-studio-operator-opt.json"
VIBE = NATIVE / "recipes/field-studio-operator.vibe"
ATTEST_EXE = NATIVE / "provenance/attest.exe"
HELPER_RS = SKL1_DIR / "attach_skl1.rs"
HELPER_EXE = SKL1_DIR / "attach_skl1.exe"
UNATTESTED_OUT = SKL1_DIR / "field-studio-operator-opt.unattested.10d"
ATTESTED_OUT = SKL1_DIR / "field-studio-operator-opt.10d"
RECEIPT = SKL1_DIR / "field-studio-operator-opt.json"
FIRST_PAINT = ROOT / "data/ewm/first-paint.json"
RLIB = Path("C:/github/synth/target-prod/debug/deps/libqualia_core_db.rlib")
DEPS = Path("C:/github/synth/target-prod/debug/deps")
CLIPS = ("walk_to_panels", "walk_to_hitch")
ANATOMY = "compile_vertebrate_anatomy_10d"
FNV_OFFSET = 0xCBF29CE484222325
FNV_PRIME = 0x100000001B3
FNV_MASK60 = 0x0FFFFFFFFFFFFFFF
SKL1_TYPE = 16


def sha256(path: Path) -> str:
    return hashlib.sha256(path.read_bytes()).hexdigest()


def q_hash(name: str) -> int:
    h = FNV_OFFSET
    for byte in name.encode("utf-8"):
        h ^= byte
        h = (h * FNV_PRIME) & 0xFFFFFFFFFFFFFFFF
    return h & FNV_MASK60


def refuse_anatomy(text: str, where: str) -> None:
    if f"{ANATOMY}(" in text:
        raise SystemExit(f"{where} must not call {ANATOMY}")


def load_first_paint() -> dict:
    data = json.loads(FIRST_PAINT.read_text(encoding="utf-8"))
    ids = data["first_paint"]
    if len(ids) != 24:
        raise SystemExit(f"first_paint length {len(ids)} != 24")
    if "field-studio-operator-opt" in ids:
        raise SystemExit("operator-opt must not join first-paint 24")
    extra = {row["id"] for row in data.get("enhancement_not_first_paint", [])}
    if "field-studio-operator-opt" not in extra:
        raise SystemExit("operator-opt is not catalogued as Enhancement")
    if data.get("inner_ranges") is not False:
        raise SystemExit("inner_ranges must stay false")
    return data


def clip_ids_from_10d(path: Path) -> list[str]:
    blob = path.read_bytes()
    if blob[:4] != b"10d\0":
        raise SystemExit(f"{path} is not a .10d")
    table_off, count = struct.unpack_from("<II", blob, 56)
    payload = None
    for i in range(count):
        rec = table_off + i * 24
        st, _tier, _res, off, length, _stride, _n, _crc = struct.unpack_from(
            "<BBHIIIII", blob, rec
        )
        if st == SKL1_TYPE:
            payload = blob[off : off + length]
            break
    if payload is None:
        raise SystemExit(f"{path} has no SKL1 section")
    if payload[:4] != b"SKL1":
        raise SystemExit(f"{path} skeletal section is not SKL1")
    joints, vertices, clips, _tracks, _keys = struct.unpack_from("<IIIII", payload, 8)
    if clips < 2:
        raise SystemExit(f"{path} SKL1 clip count {clips} < 2")
    names = []
    want = {q_hash(name): name for name in CLIPS}
    base = 32 + joints * 144
    for i in range(clips):
        cid, duration, _first, _ntracks, looping, reserved = struct.unpack_from(
            "<QdIIII", payload, base + i * 32
        )
        if reserved != 0:
            raise SystemExit("SKL1 clip reserved must be 0")
        name = want.get(cid)
        if name:
            names.append(name)
        if looping not in (0, 1) or duration <= 0:
            raise SystemExit("invalid SKL1 clip header")
    if vertices == 0:
        raise SystemExit("SKL1 vertex count is 0")
    missing = [name for name in CLIPS if name not in names]
    if missing:
        raise SystemExit(f"{path} missing clips {missing}")
    return names


def compile_helper() -> None:
    if not HELPER_RS.is_file():
        raise SystemExit(f"missing {HELPER_RS}")
    refuse_anatomy(HELPER_RS.read_text(encoding="utf-8"), str(HELPER_RS))
    if not RLIB.is_file():
        raise SystemExit(f"missing QualiaDB rlib {RLIB} (do not rebuild synth target)")
    stale = (not HELPER_EXE.is_file()) or (
        HELPER_EXE.stat().st_mtime < HELPER_RS.stat().st_mtime
    )
    if not stale:
        return
    cmd = [
        "rustc",
        "--edition",
        "2021",
        str(HELPER_RS),
        f"--extern=qualia_core_db={RLIB}",
        f"-L",
        f"dependency={DEPS}",
        "-o",
        str(HELPER_EXE),
    ]
    result = subprocess.run(cmd, capture_output=True, text=True)
    if result.returncode != 0:
        raise SystemExit(
            "native SKL1 helper rustc failed (existing rlib, no cargo/synth rebuild):\n"
            + result.stderr
        )


def run_helper(args: list[str]) -> dict:
    result = subprocess.run(
        [str(HELPER_EXE), *args], capture_output=True, text=True, check=False
    )
    if result.returncode != 0:
        raise SystemExit(result.stderr.strip() or result.stdout.strip() or "helper failed")
    return json.loads(result.stdout)


def attest(unattested: Path, attested: Path) -> dict:
    result = subprocess.run(
        [
            str(ATTEST_EXE),
            str(unattested),
            str(VIBE),
            str(attested),
            "text/x-vibe-script",
        ],
        capture_output=True,
        text=True,
        check=False,
    )
    if result.returncode != 0:
        raise SystemExit(result.stderr.strip() or result.stdout.strip() or "attest failed")
    report = json.loads(result.stdout)
    report.update(
        {
            "id": "field-studio-operator-opt",
            "original_path": str(unattested.relative_to(ROOT)).replace("\\", "/"),
            "attested_path": str(attested.relative_to(ROOT)).replace("\\", "/"),
            "original_sha256": sha256(unattested),
            "source_sha256": sha256(VIBE),
            "attested_sha256": sha256(attested),
            "native_helper_sha256": sha256(ATTEST_EXE),
            "source_media_type": "text/x-vibe-script",
            "source_path": "production/native-world/recipes/field-studio-operator.vibe",
            "rights": (
                "User-commissioned artwork for the Webizen website project; local "
                "preview and inclusion in this project authorised by the project owner. "
                "No general reuse licence or third-party endorsement asserted."
            ),
            "credential_asserted": False,
            "compile_vertebrate_anatomy_10d": False,
            "role": "Enhancement",
            "first_paint": False,
            "clip_names": list(CLIPS),
            "reduced_motion_skips_clips": True,
        }
    )
    return report


def already_done() -> bool:
    if not ATTESTED_OUT.is_file() or not RECEIPT.is_file() or not HELPER_EXE.is_file():
        return False
    try:
        names = clip_ids_from_10d(ATTESTED_OUT)
        inspect = run_helper(["inspect", str(ATTESTED_OUT)])
    except (SystemExit, json.JSONDecodeError, OSError):
        return False
    return names == list(CLIPS) and inspect.get("passed") is True


def attach() -> dict:
    SKL1_DIR.mkdir(parents=True, exist_ok=True)
    refuse_anatomy(HELPER_RS.read_text(encoding="utf-8"), str(HELPER_RS))
    catalog = load_first_paint()
    pin = json.loads(PROVENANCE.read_text(encoding="utf-8"))
    if sha256(ORIGINAL) != pin["original_sha256"]:
        raise SystemExit("original operator-opt sha256 drifted from provenance")
    if not ATTESTED_SRC.is_file() or sha256(ATTESTED_SRC) != pin["attested_sha256"]:
        raise SystemExit("attested operator-opt is missing or drifted")
    compile_helper()
    if already_done() and "--force" not in sys.argv:
        receipt = json.loads(RECEIPT.read_text(encoding="utf-8"))
        receipt["idempotent"] = True
        return receipt
    attached = run_helper([str(ORIGINAL), str(UNATTESTED_OUT)])
    if attached.get("compile_vertebrate_anatomy_10d") is not False:
        raise SystemExit("helper must not claim anatomy compile")
    names = clip_ids_from_10d(UNATTESTED_OUT)
    if names != list(CLIPS):
        raise SystemExit(f"unattested clips {names}")
    report = attest(UNATTESTED_OUT, ATTESTED_OUT)
    inspect = run_helper(["inspect", str(ATTESTED_OUT)])
    report.update(
        {
            "skl1_vertices": inspect["vertices"],
            "skl1_joints": inspect["joints"],
            "skl1_clips": inspect["clips"],
            "clip_records": inspect["clip_records"],
            "unattested_bytes": UNATTESTED_OUT.stat().st_size,
            "attested_bytes": ATTESTED_OUT.stat().st_size,
            "assets_copy": "integrator copies attested operator-opt .10d into production/native-world/assets after SKL1 readback",
            "first_paint_len": len(catalog["first_paint"]),
            "inner_ranges": False,
            "streaming_default": False,
            "passed": True,
        }
    )
    RECEIPT.write_text(json.dumps(report, indent=2) + "\n", encoding="utf-8")
    return report


def main() -> int:
    if "--help" in sys.argv:
        print("python tooling/ewm_operator_skl1.py [--force] [--verify]")
        return 0
    if "--verify" in sys.argv and ATTESTED_OUT.is_file():
        compile_helper()
        names = clip_ids_from_10d(ATTESTED_OUT)
        inspect = run_helper(["inspect", str(ATTESTED_OUT)])
        load_first_paint()
        print(
            json.dumps(
                {
                    "passed": True,
                    "clip_names": names,
                    "inspect": inspect,
                    "attested_sha256": sha256(ATTESTED_OUT),
                    "compile_vertebrate_anatomy_10d": False,
                    "first_paint": False,
                },
                indent=2,
            )
        )
        return 0
    report = attach()
    print(json.dumps({k: report[k] for k in (
        "passed",
        "attested_path",
        "attested_sha256",
        "clip_names",
        "skl1_vertices",
        "skl1_clips",
        "first_paint_len",
        "compile_vertebrate_anatomy_10d",
        "reduced_motion_skips_clips",
    ) if k in report}, indent=2))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
