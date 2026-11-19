"""Generate data/ewm/species-catalog.ttl and production/species-cohort/species-catalog.json.

Reads each species' `dist/lods/lod-manifest.json`, `ontology/taxon.ttl`, and sidecar
digests (`taxon.q42`, `-field.emf`, `-substrate.m64` / `-call.qac`) directly from
`C:\\github\\eco-intent\\<slug>\\` without copying any `.10d` or `.hmc` binaries.
"""
from __future__ import annotations

import hashlib
import json
import re
from pathlib import Path

ECO_INTENT_ROOT = Path(r"C:\github\eco-intent")
WEBIZEN_ROOT = Path(r"C:\github\webizen-website")
OUT_TTL = WEBIZEN_ROOT / "data" / "ewm" / "species-catalog.ttl"
OUT_JSON = WEBIZEN_ROOT / "production" / "species-cohort" / "species-catalog.json"

SPECIES_META = [
    ("eucalyptus-camaldulensis", "flora", "Riverine floodplains & riparian corridors (Murray-Darling / SE Australia)"),
    ("eucalyptus-dalrympleana", "flora", "Montane & subalpine wet sclerophyll forests (SE Australian highlands)"),
    ("eucalyptus-regnans", "flora", "Cool temperate tall wet sclerophyll mountain gullies (Victoria & Tasmania)"),
    ("eucalyptus-melliodora", "flora", "Grassy box-gum woodlands on tablelands, foothills & western slopes"),
    ("banksia-integrifolia", "flora", "Coastal dunes, headlands & littoral sclerophyll woodlands"),
    ("acacia-pycnantha", "flora", "Open sclerophyll forests, dry ridges & mallee-woodland margins"),
    ("dicksonia-antarctica", "flora", "Cool temperate fern gullies, rainforest understorey & riparian slopes"),
    ("wallabia-bicolor", "fauna", "Dense coastal & montane sclerophyll understorey, heath & damp gullies"),
    ("phascolarctos-cinereus", "fauna", "Arboreal eucalypt open forest & riparian woodland canopy"),
    ("vombatus-ursinus", "fauna", "Burrowing temperate sclerophyll forests, coastal heath & subalpine slopes"),
    ("macropus-giganteus", "fauna", "Grassy open woodlands, riverine plains & subalpine grasslands"),
    ("ornithorhynchus-anatinus", "fauna", "Freshwater lotic creeks, river pools & riparian earth banks"),
    ("dacelo-novaeguineae", "fauna", "Open eucalypt woodlands, forest edges & riparian corridor perches"),
    ("poa-labillardierei", "flora", "Riparian floodplains, moist grassy valleys & damp sclerophyll understorey (SE Australia)"),
    ("themeda-triandra", "flora", "Grassy box-gum woodlands, basalt plains & open sclerophyll understorey (SE Australia)"),
    ("lomandra-longifolia", "flora", "Riparian banks, damp gullies, coastal dunes & dry-to-wet sclerophyll understorey (SE Australia)"),
    ("bursaria-spinosa", "flora", "Dry & damp sclerophyll woodland understorey, grassy box-gum plains & rocky slopes (SE Australia)"),
]


def sha256_file(path: Path) -> tuple[str, int]:
    raw = path.read_bytes()
    return hashlib.sha256(raw).hexdigest(), len(raw)


def parse_taxon_names(slug: str) -> tuple[str, str, str]:
    ttl = (ECO_INTENT_ROOT / slug / "ontology" / "taxon.ttl").read_text(encoding="utf-8")
    sci_m = re.search(r'dwc:scientificName\s+"([^"]+)"', ttl)
    vern_m = re.search(r'dwc:vernacularName\s+"([^"]+)"@en', ttl)
    lbl_m = re.search(r'rdfs:label\s+"([^"]+)"@en', ttl)
    king_m = re.search(r'dwc:kingdom\s+"([^"]+)"', ttl)
    sci = sci_m.group(1) if sci_m else slug
    common = (vern_m or lbl_m).group(1) if (vern_m or lbl_m) else slug
    dwc_kingdom = king_m.group(1) if king_m else "Unknown"
    return sci, common, dwc_kingdom


def ttl_str(val: str) -> str:
    return json.dumps(val, ensure_ascii=False)


def main() -> int:
    catalog_entries = []
    lines = [
        "@prefix rdf:  <http://www.w3.org/1999/02/22-rdf-syntax-ns#> .",
        "@prefix rdfs: <http://www.w3.org/2000/01/rdf-schema#> .",
        "@prefix owl:  <http://www.w3.org/2002/07/owl#> .",
        "@prefix xsd:  <http://www.w3.org/2001/XMLSchema#> .",
        "@prefix dwc:  <http://rs.tdwg.org/dwc/terms/> .",
        "@prefix dcat: <http://www.w3.org/ns/dcat#> .",
        "@prefix ewm:  <https://webizen.au/relation/> .",
        "",
        "<https://webizen.au/catalog/se-australia-species>",
        "    a <https://webizen.au/catalog/SpeciesCatalog> ;",
        '    ewm:schema "webizen.species-catalog/v1" ;',
        f"    ewm:speciesCount {len(SPECIES_META)} ;",
    ]
    species_iris = [f"<did:q42:taxon:{slug}>" for slug, _, _ in SPECIES_META]
    lines.append("    ewm:hasSpecies " + " , ".join(species_iris) + " .")
    lines.append("")

    for slug, kingdom, eco_zone in SPECIES_META:
        dist_dir = ECO_INTENT_ROOT / slug / "dist"
        manifest = json.loads((dist_dir / "lods" / "lod-manifest.json").read_text(encoding="utf-8"))
        sci_name, common_name, dwc_kingdom = parse_taxon_names(slug)
        did_slug = f"did:q42:taxon:{slug}"
        did_under = f"did:q42:taxon:{slug.replace('-', '_')}"

        src = manifest["source_10d"]
        bmin = src["bounds_min"]
        bmax = src["bounds_max"]

        taxon_sha, taxon_bytes = sha256_file(dist_dir / "taxon.q42")
        emf_sha, emf_bytes = sha256_file(dist_dir / f"{slug}-field.emf")
        sidecars = {
            "taxon_q42": {
                "local_url": f"/intenteco/{slug}/dist/taxon.q42",
                "remote_url": f"https://raw.githubusercontent.com/intenteco/{slug}/main/dist/taxon.q42",
                "sha256": taxon_sha,
                "bytes": taxon_bytes,
            },
            "field_emf": {
                "local_url": f"/intenteco/{slug}/dist/{slug}-field.emf",
                "remote_url": f"https://raw.githubusercontent.com/intenteco/{slug}/main/dist/{slug}-field.emf",
                "sha256": emf_sha,
                "bytes": emf_bytes,
            },
        }
        if kingdom == "flora":
            m64_sha, m64_bytes = sha256_file(dist_dir / f"{slug}-substrate.m64")
            sidecars["substrate_m64"] = {
                "local_url": f"/intenteco/{slug}/dist/{slug}-substrate.m64",
                "remote_url": f"https://raw.githubusercontent.com/intenteco/{slug}/main/dist/{slug}-substrate.m64",
                "sha256": m64_sha,
                "bytes": m64_bytes,
            }
        else:
            qac_sha, qac_bytes = sha256_file(dist_dir / f"{slug}-call.qac")
            sidecars["call_qac"] = {
                "local_url": f"/intenteco/{slug}/dist/{slug}-call.qac",
                "remote_url": f"https://raw.githubusercontent.com/intenteco/{slug}/main/dist/{slug}-call.qac",
                "sha256": qac_sha,
                "bytes": qac_bytes,
            }

        lod_iris = [f"<https://webizen.au/species/{slug}/lod/{tier}>" for tier in manifest["lods"]]

        lines.extend([
            f"<{did_slug}>",
            "    a dwc:Taxon , dcat:Dataset ;",
            f"    owl:sameAs <{did_under}> ;",
            f"    ewm:slug {ttl_str(slug)} ;",
            f"    dwc:scientificName {ttl_str(sci_name)} ;",
            f"    dwc:vernacularName {ttl_str(common_name)}@en ;",
            f"    rdfs:label {ttl_str(common_name)}@en ;",
            f"    ewm:kingdom {ttl_str(kingdom)} ;",
            f"    dwc:kingdom {ttl_str(dwc_kingdom)} ;",
            f"    ewm:ecologicalZone {ttl_str(eco_zone)} ;",
            f"    ewm:aabbMinX {bmin[0]:.6f} ;",
            f"    ewm:aabbMinY {bmin[1]:.6f} ;",
            f"    ewm:aabbMinZ {bmin[2]:.6f} ;",
            f"    ewm:aabbMaxX {bmax[0]:.6f} ;",
            f"    ewm:aabbMaxY {bmax[1]:.6f} ;",
            f"    ewm:aabbMaxZ {bmax[2]:.6f} ;",
            f'    ewm:canonicalUrl "/intenteco/{slug}/dist/{slug}.10d" ;',
            f'    ewm:canonicalRemoteUrl "https://raw.githubusercontent.com/intenteco/{slug}/main/dist/{slug}.10d" ;',
            f"    ewm:canonicalSha256 {ttl_str(src['sha256'])} ;",
            f"    ewm:canonicalBytes {src['bytes']} ;",
            f"    ewm:canonicalTriangles {src['triangles']} ;",
            f"    ewm:canonicalVertices {src['vertices']} ;",
            f"    ewm:sidecarTaxonQ42 {ttl_str(sidecars['taxon_q42']['local_url'])} ;",
            f"    ewm:sidecarTaxonQ42Sha256 {ttl_str(taxon_sha)} ;",
            f"    ewm:sidecarFieldEmf {ttl_str(sidecars['field_emf']['local_url'])} ;",
            f"    ewm:sidecarFieldEmfSha256 {ttl_str(emf_sha)} ;",
        ])
        if "substrate_m64" in sidecars:
            lines.extend([
                f"    ewm:sidecarSubstrateM64 {ttl_str(sidecars['substrate_m64']['local_url'])} ;",
                f"    ewm:sidecarSubstrateM64Sha256 {ttl_str(sidecars['substrate_m64']['sha256'])} ;",
            ])
        else:
            lines.extend([
                f"    ewm:sidecarCallQac {ttl_str(sidecars['call_qac']['local_url'])} ;",
                f"    ewm:sidecarCallQacSha256 {ttl_str(sidecars['call_qac']['sha256'])} ;",
            ])
        lines.append("    ewm:hasLod " + " , ".join(lod_iris) + " .")
        lines.append("")

        for tier, lod in manifest["lods"].items():
            lmin, lmax = lod["bounds_min"], lod["bounds_max"]
            lines.extend([
                f"<https://webizen.au/species/{slug}/lod/{tier}>",
                f"    ewm:forSpecies <{did_slug}> ;",
                f"    ewm:tier {ttl_str(tier)} ;",
                f"    ewm:distanceBand {ttl_str(lod['distance_band_m'])} ;",
                f"    ewm:url {ttl_str(lod['local_url'])} ;",
                f"    ewm:remoteUrl {ttl_str(lod['remote_url'])} ;",
                f"    ewm:sha256 {ttl_str(lod['sha256'])} ;",
                f"    ewm:bytes {lod['bytes']} ;",
                f"    ewm:triangles {lod['triangles']} ;",
                f"    ewm:vertices {lod['vertices']} ;",
                f"    ewm:aabbMinX {lmin[0]:.6f} ;",
                f"    ewm:aabbMinY {lmin[1]:.6f} ;",
                f"    ewm:aabbMinZ {lmin[2]:.6f} ;",
                f"    ewm:aabbMaxX {lmax[0]:.6f} ;",
                f"    ewm:aabbMaxY {lmax[1]:.6f} ;",
                f"    ewm:aabbMaxZ {lmax[2]:.6f} .",
                "",
            ])

        catalog_entries.append({
            "slug": slug,
            "did": did_slug,
            "did_taxon_underscore": did_under,
            "scientific_name": sci_name,
            "common_name": common_name,
            "kingdom": kingdom,
            "dwc_kingdom": dwc_kingdom,
            "ecological_zone": eco_zone,
            "aabb": {"min": bmin, "max": bmax},
            "canonical": {
                "local_url": f"/intenteco/{slug}/dist/{slug}.10d",
                "remote_url": f"https://raw.githubusercontent.com/intenteco/{slug}/main/dist/{slug}.10d",
                "sha256": src["sha256"],
                "bytes": src["bytes"],
                "triangles": src["triangles"],
                "vertices": src["vertices"],
            },
            "lods": manifest["lods"],
            "sidecars": sidecars,
        })

    OUT_TTL.parent.mkdir(parents=True, exist_ok=True)
    OUT_TTL.write_text("\n".join(lines), encoding="utf-8")
    OUT_JSON.parent.mkdir(parents=True, exist_ok=True)
    OUT_JSON.write_text(
        json.dumps(
            {
                "schema": "webizen.species-catalog/v1",
                "species_count": len(catalog_entries),
                "species": catalog_entries,
            },
            indent=2,
            ensure_ascii=False,
        )
        + "\n",
        encoding="utf-8",
    )
    print(f"Wrote {OUT_TTL} ({OUT_TTL.stat().st_size} bytes) and {OUT_JSON} ({OUT_JSON.stat().st_size} bytes)")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
