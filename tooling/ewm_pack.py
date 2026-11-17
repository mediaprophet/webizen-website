"""Pack EWM first-paint members as byte-identical copies and QBDL whole-entry shards."""
from __future__ import annotations

import hashlib
import json
import shutil
import struct
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
CATALOG = ROOT / "data" / "ewm" / "first-paint.json"
DEM_SRC = (
    ROOT
    / "data"
    / "publications"
    / "aaa"
    / "terrain-model-v2"
    / "reference-terrain-model.10d"
)
PACKAGES = ROOT / "packages"
QBDL = PACKAGES / "qbdl"
BY_SHA = PACKAGES / "by-sha256"
DOWNLOAD = PACKAGES / "download"
WOODLAND_QBDL = ROOT / "production" / "lush-woodland" / "qbdl"

# CRC-32C Castagnoli
_CRC32C_TABLE = None


def crc32c(data: bytes) -> int:
    global _CRC32C_TABLE
    if _CRC32C_TABLE is None:
        table = []
        poly = 0x82F63B78
        for i in range(256):
            crc = i
            for _ in range(8):
                crc = (crc >> 1) ^ poly if crc & 1 else crc >> 1
            table.append(crc)
        _CRC32C_TABLE = table
    crc = 0xFFFFFFFF
    for b in data:
        crc = _CRC32C_TABLE[(crc ^ b) & 0xFF] ^ (crc >> 8)
    return crc ^ 0xFFFFFFFF


def cbor_uint(n: int) -> bytes:
    if n < 24:
        return bytes([n])
    if n < 256:
        return bytes([0x18, n])
    if n < 65536:
        return bytes([0x19]) + n.to_bytes(2, "big")
    if n < 2**32:
        return bytes([0x1A]) + n.to_bytes(4, "big")
    return bytes([0x1B]) + n.to_bytes(8, "big")


def cbor_text(s: str) -> bytes:
    b = s.encode("utf-8")
    if len(b) < 24:
        return bytes([0x60 + len(b)]) + b
    if len(b) < 256:
        return bytes([0x78, len(b)]) + b
    return bytes([0x79]) + len(b).to_bytes(2, "big") + b


def cbor_bytes(b: bytes) -> bytes:
    if len(b) < 24:
        return bytes([0x40 + len(b)]) + b
    if len(b) < 256:
        return bytes([0x58, len(b)]) + b
    return bytes([0x59]) + len(b).to_bytes(2, "big") + b


def cbor_map(items: list[tuple[str, bytes]]) -> bytes:
    head = bytes([0xA0 + len(items)]) if len(items) < 24 else bytes([0xB8, len(items)])
    return head + b"".join(cbor_text(k) + v for k, v in items)


def cbor_array(items: list[bytes]) -> bytes:
    if len(items) < 24:
        head = bytes([0x80 + len(items)])
    else:
        head = bytes([0x98, len(items)])
    return head + b"".join(items)


def align_up(n: int, align: int) -> int:
    return (n + align - 1) // align * align


def write_qbdl(entry_key: str, kind: str, payload: bytes, dest: Path) -> None:
    align = 64
    out = bytearray(64)
    padded = align_up(len(out), align)
    out.extend(b"\x00" * (padded - len(out)))
    offset = len(out)
    out.extend(payload)
    sha = hashlib.sha256(payload).digest()
    index = cbor_array(
        [
            cbor_map(
                [
                    ("key", cbor_text(entry_key)),
                    ("kind", cbor_text(kind)),
                    ("offset", cbor_uint(offset)),
                    ("length", cbor_uint(len(payload))),
                    ("sha256", cbor_bytes(sha)),
                ]
            )
        ]
    )
    padded = align_up(len(out), align)
    out.extend(b"\x00" * (padded - len(out)))
    index_offset = len(out)
    out.extend(index)
    total = len(out)
    out[0:4] = b"QBDL"
    struct.pack_into("<H", out, 4, 1)
    struct.pack_into("<H", out, 6, 0)
    struct.pack_into("<I", out, 8, 1)
    struct.pack_into("<Q", out, 12, index_offset)
    struct.pack_into("<Q", out, 20, len(index))
    struct.pack_into("<Q", out, 28, total)
    struct.pack_into("<I", out, 36, 0)
    crc = crc32c(bytes(out))
    struct.pack_into("<I", out, 36, crc)
    dest.parent.mkdir(parents=True, exist_ok=True)
    dest.write_bytes(out)
    if dest.stat().st_size >= 24 * 1024 * 1024:
        raise SystemExit(f"{dest} exceeds 24 MiB shard cap")


def resolve_url(url: str) -> Path:
    if not url.startswith("/"):
        raise SystemExit(f"bad url {url}")
    return ROOT / url.lstrip("/")


def main() -> int:
    catalog = json.loads(CATALOG.read_text(encoding="utf-8"))
    BY_SHA.mkdir(parents=True, exist_ok=True)
    (QBDL / "base").mkdir(parents=True, exist_ok=True)
    (QBDL / "enhancement").mkdir(parents=True, exist_ok=True)
    WOODLAND_QBDL.mkdir(parents=True, exist_ok=True)
    DOWNLOAD.mkdir(parents=True, exist_ok=True)

    if not DEM_SRC.is_file():
        raise SystemExit(f"missing DEM {DEM_SRC}")
    dem_bytes = DEM_SRC.read_bytes()
    dem_sha = hashlib.sha256(dem_bytes).hexdigest()
    want = None
    for row in catalog["entries"]:
        if row["scene_id"] == "canberra-reference-terrain":
            want = row["sha256"]
            break
    if want and dem_sha != want:
        raise SystemExit(f"DEM sha {dem_sha} != catalog {want}")
    dest_dem = BY_SHA / dem_sha
    if not dest_dem.exists() or dest_dem.read_bytes() != dem_bytes:
        shutil.copyfile(DEM_SRC, dest_dem)

    copied = []
    for row in catalog["entries"]:
        url = row["url"]
        src = resolve_url(url)
        if not src.is_file():
            raise SystemExit(f"missing first-paint source {src}")
        raw = src.read_bytes()
        sha = hashlib.sha256(raw).hexdigest()
        if sha != row["sha256"]:
            raise SystemExit(f"sha mismatch {row['scene_id']}: {sha}")
        if len(raw) != row["bytes"]:
            raise SystemExit(f"size mismatch {row['scene_id']}")
        shard_kind = row.get("shard", "base")
        shard_dir = QBDL / shard_kind
        shard_dir.mkdir(parents=True, exist_ok=True)
        hmc_name = f"{row['scene_id']}.hmc"
        write_qbdl(row["scene_id"] + ".10d", "10d", raw, shard_dir / hmc_name)
        woodland_dir = WOODLAND_QBDL / shard_kind
        woodland_dir.mkdir(parents=True, exist_ok=True)
        shutil.copyfile(shard_dir / hmc_name, woodland_dir / hmc_name)
        copied.append(
            {
                "scene_id": row["scene_id"],
                "sha256": sha,
                "bytes": len(raw),
                "url": url,
                "hmc": f"/packages/qbdl/{shard_kind}/{row['scene_id']}.hmc",
            }
        )

    receipt = {
        "schema": "webizen.ewm-pack-receipt/v1",
        "inner_ranges": False,
        "magic": "QBDL",
        "first_paint": catalog["first_paint"],
        "entries": copied,
        "source_publication": str(DEM_SRC.relative_to(ROOT).as_posix()),
    }
    (QBDL / "receipt.json").write_text(json.dumps(receipt, indent=2) + "\n", encoding="utf-8")
    shutil.copyfile(QBDL / "receipt.json", WOODLAND_QBDL / "receipt.json")
    download = {
        "schema": "webizen.ewm-download-pack/v1",
        "identities": copied,
        "same_as_stream": True,
        "streaming_default": False,
    }
    (DOWNLOAD / "first-paint.json").write_text(
        json.dumps(download, indent=2) + "\n", encoding="utf-8"
    )
    print(f"packed {len(copied)} first-paint members; DEM {dem_sha[:12]}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
