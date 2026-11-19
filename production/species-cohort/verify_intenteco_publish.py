"""Standalone anti-monolith publish verifier for the 13 SE Australian eco-intent species repos.

Reads `data/ewm/species-catalog.ttl` (and verifies `data/ewm/species-catalog.q42`),
locates all 13 sibling repositories under `C:\\github\\eco-intent\\<species-slug>\\`,
verifies every referenced `dist/lods/*.10d` file on disk against its recorded SHA-256
digest and byte size, checks anti-monolith isolation in `webizen-website`, and
prints a concise per-repo GitHub push-readiness table.
"""
from __future__ import annotations

import argparse
import hashlib
import re
import sys
from pathlib import Path

WEBIZEN_ROOT = Path(__file__).resolve().parents[2]
DEFAULT_TTL = WEBIZEN_ROOT / "data" / "ewm" / "species-catalog.ttl"
DEFAULT_Q42 = WEBIZEN_ROOT / "data" / "ewm" / "species-catalog.q42"
DEFAULT_ECO_INTENT_ROOT = WEBIZEN_ROOT.parent / "eco-intent"
if not DEFAULT_ECO_INTENT_ROOT.exists():
    DEFAULT_ECO_INTENT_ROOT = Path(r"C:\github\eco-intent")


def sha256_and_size(path: Path) -> tuple[str, int]:
    """Compute SHA-256 hex digest and byte length of a file."""
    h = hashlib.sha256()
    total = 0
    with path.open("rb") as fh:
        while True:
            chunk = fh.read(1024 * 1024)
            if not chunk:
                break
            h.update(chunk)
            total += len(chunk)
    return h.hexdigest(), total


def parse_species_catalog_ttl(ttl_path: Path) -> tuple[int, list[dict], list[dict]]:
    """Parse species blocks and LOD blocks from `species-catalog.ttl`."""
    text = ttl_path.read_text(encoding="utf-8")
    count_m = re.search(r"ewm:speciesCount\s+(\d+)", text)
    expected_species_count = int(count_m.group(1)) if count_m else 0

    blocks = [b.strip() for b in text.split("\n\n") if b.strip() and not b.strip().startswith("@prefix")]
    species_records: list[dict] = []
    lod_records: list[dict] = []

    for block in blocks:
        if block.startswith("<did:q42:taxon:"):
            slug = _extract_str(block, r"ewm:slug")
            kingdom = _extract_str(block, r"ewm:kingdom")
            canonical_url = _extract_str(block, r"ewm:canonicalUrl")
            canonical_sha = _extract_str(block, r"ewm:canonicalSha256")
            canonical_bytes = _extract_int(block, r"ewm:canonicalBytes")
            lod_iris = re.findall(r"<https://webizen\.au/species/[^/]+/lod/([^>]+)>", block)
            species_records.append(
                {
                    "slug": slug,
                    "kingdom": kingdom,
                    "canonical_url": canonical_url,
                    "canonical_sha256": canonical_sha,
                    "canonical_bytes": canonical_bytes,
                    "declared_tiers": lod_iris,
                }
            )
        elif block.startswith("<https://webizen.au/species/"):
            iri_m = re.match(r"<https://webizen\.au/species/([^/]+)/lod/([^>]+)>", block)
            if not iri_m:
                raise ValueError(f"Malformed LOD IRI in block:\n{block}")
            slug, tier_from_iri = iri_m.group(1), iri_m.group(2)
            lod_records.append(
                {
                    "slug": slug,
                    "tier": _extract_str(block, r"ewm:tier") or tier_from_iri,
                    "url": _extract_str(block, r"ewm:url"),
                    "remote_url": _extract_str(block, r"ewm:remoteUrl"),
                    "sha256": _extract_str(block, r"ewm:sha256"),
                    "bytes": _extract_int(block, r"ewm:bytes"),
                    "triangles": _extract_int(block, r"ewm:triangles"),
                }
            )

    return expected_species_count, species_records, lod_records


def _extract_str(block: str, predicate: str) -> str:
    m = re.search(rf'{predicate}\s+"([^"]*)"', block)
    if not m:
        raise ValueError(f"Missing {predicate} in block:\n{block[:160]}")
    return m.group(1)


def _extract_int(block: str, predicate: str) -> int:
    m = re.search(rf"{predicate}\s+(\d+)", block)
    if not m:
        raise ValueError(f"Missing integer {predicate} in block:\n{block[:160]}")
    return int(m.group(1))


def resolve_intenteco_url(url: str, expected_slug: str, eco_root: Path) -> Path:
    """Resolve `/intenteco/<slug>/...` to a sibling repo file without traversal."""
    prefix = f"/intenteco/{expected_slug}/"
    if not url.startswith(prefix) or ".." in url or "\\" in url:
        raise ValueError(f"Invalid local intenteco URL '{url}' for slug '{expected_slug}'")
    rel = url[len(prefix) :]
    repo_dir = (eco_root / expected_slug).resolve()
    dest = (repo_dir / rel).resolve()
    if not dest.is_relative_to(repo_dir):
        raise ValueError(f"Escaped repo boundary: {dest}")
    return dest


def check_anti_monolith(webizen_root: Path) -> list[str]:
    """Ensure no eco-intent binaries were duplicated into webizen-website."""
    issues: list[str] = []
    for bad_dir in (webizen_root / "intenteco", webizen_root / "site" / "intenteco"):
        if bad_dir.exists():
            issues.append(f"Forbidden monolith directory exists: {bad_dir}")
    return issues


def format_bytes(n: int) -> str:
    if n >= 1024 * 1024:
        return f"{n / (1024 * 1024):.2f} MiB"
    return f"{n / 1024:.1f} KiB"


def verify(ttl_path: Path, q42_path: Path, eco_root: Path, webizen_root: Path) -> int:
    errors: list[str] = []

    if not ttl_path.is_file():
        print(f"ERROR: Missing catalog TTL: {ttl_path}", file=sys.stderr)
        return 2
    if not q42_path.is_file() or q42_path.stat().st_size == 0:
        errors.append(f"Missing or empty species-catalog.q42: {q42_path}")

    errors.extend(check_anti_monolith(webizen_root))

    expected_count, species_list, lod_list = parse_species_catalog_ttl(ttl_path)
    if expected_count and len(species_list) != expected_count:
        errors.append(f"Catalog speciesCount={expected_count}, parsed {len(species_list)}")

    lods_by_slug: dict[str, list[dict]] = {}
    for lod in lod_list:
        lods_by_slug.setdefault(lod["slug"], []).append(lod)

    rows: list[tuple[str, str, str, str, str, str, str]] = []
    verified_repos = 0
    verified_lods = 0
    total_lods = len(lod_list)

    for sp in species_list:
        slug = sp["slug"]
        repo_dir = eco_root / slug
        repo_lods = lods_by_slug.get(slug, [])
        repo_errors: list[str] = []

        if not repo_dir.is_dir():
            repo_errors.append(f"missing repo dir {repo_dir}")

        if set(sp["declared_tiers"]) != {l["tier"] for l in repo_lods}:
            repo_errors.append("declared ewm:hasLod tiers do not match LOD blocks")

        canon_path = resolve_intenteco_url(sp["canonical_url"], slug, eco_root)
        if not canon_path.is_file():
            repo_errors.append(f"missing canonical {canon_path.name}")
        else:
            c_sha, c_bytes = sha256_and_size(canon_path)
            if c_sha != sp["canonical_sha256"] or c_bytes != sp["canonical_bytes"]:
                repo_errors.append(f"canonical mismatch ({canon_path.name})")

        lod_ok_count = 0
        lod_total_bytes = 0
        max_lod_bytes = 0
        for lod in repo_lods:
            url = lod["url"]
            expected_remote = f"https://raw.githubusercontent.com/intenteco/{slug}/main/dist/lods/"
            if "/dist/lods/" not in url or not url.endswith(".10d"):
                repo_errors.append(f"unexpected LOD path {url}")
                continue
            if not lod["remote_url"].startswith(expected_remote):
                repo_errors.append(f"unexpected remoteUrl {lod['remote_url']}")
            lod_path = resolve_intenteco_url(url, slug, eco_root)
            if not lod_path.is_file():
                repo_errors.append(f"missing {lod_path.name}")
                continue
            actual_sha, actual_bytes = sha256_and_size(lod_path)
            if actual_sha != lod["sha256"]:
                repo_errors.append(f"sha256 mismatch {lod_path.name}")
                continue
            if actual_bytes != lod["bytes"]:
                repo_errors.append(f"bytes mismatch {lod_path.name}: {actual_bytes} != {lod['bytes']}")
                continue
            lod_ok_count += 1
            verified_lods += 1
            lod_total_bytes += actual_bytes
            max_lod_bytes = max(max_lod_bytes, actual_bytes)

        git_state = "git-present" if (repo_dir / ".git").exists() else "git-init-needed"
        if not repo_errors and lod_ok_count == len(repo_lods) and repo_lods:
            verified_repos += 1
            status = "PASS"
        else:
            status = "FAIL"
            errors.extend(f"[{slug}] {e}" for e in repo_errors)

        rows.append(
            (
                slug,
                sp["kingdom"],
                f"{lod_ok_count}/{len(repo_lods)}",
                format_bytes(lod_total_bytes),
                format_bytes(max_lod_bytes),
                git_state,
                status,
            )
        )

    headers = ("Species Repo", "Kingdom", "LODs", "LOD Total", "Max LOD", "Git State", "Status")
    widths = [max(len(h), *(len(r[i]) for r in rows)) for i, h in enumerate(headers)]
    sep = "  ".join("-" * w for w in widths)
    print("  ".join(h.ljust(widths[i]) for i, h in enumerate(headers)))
    print(sep)
    for row in rows:
        print("  ".join(row[i].ljust(widths[i]) for i in range(len(headers))))
    print(sep)
    print(
        f"Summary: {verified_repos}/{len(species_list)} repos verified | "
        f"{verified_lods}/{total_lods} LODs verified | "
        f"q42={'OK' if q42_path.is_file() else 'MISSING'} | "
        f"anti-monolith={'OK' if not check_anti_monolith(webizen_root) else 'FAIL'}"
    )

    if errors:
        for err in errors:
            print(f"ERROR: {err}", file=sys.stderr)
        return 1
    return 0


def main() -> int:
    parser = argparse.ArgumentParser(description="Verify eco-intent sibling repo LODs against species-catalog.ttl.")
    parser.add_argument("--ttl", type=Path, default=DEFAULT_TTL)
    parser.add_argument("--q42", type=Path, default=DEFAULT_Q42)
    parser.add_argument("--eco-root", type=Path, default=DEFAULT_ECO_INTENT_ROOT)
    args = parser.parse_args()
    return verify(args.ttl, args.q42, args.eco_root, WEBIZEN_ROOT)


if __name__ == "__main__":
    raise SystemExit(main())
