"""Validate EWM-00 catalog schema, first-paint fixture, grant, and contracts.

No network. No jsonschema package. Native receipts are hashed, not executed.
"""
from __future__ import annotations

import hashlib
import json
import math
import re
import sys
from copy import deepcopy
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
EWM = Path(__file__).resolve().parent
SCHEMA_PATH = ROOT / "schemas" / "ewm" / "catalog.v1.json"
FIXTURE_PATH = ROOT / "data" / "ewm" / "first-paint.json"
INVALID_HEX_PATH = ROOT / "schemas" / "ewm" / "fixtures" / "invalid-single-hex-candidate.json"
GRANT_PATH = EWM / "aaa-path-lock-grant.json"
OWNERSHIP_PATH = EWM / "file-ownership.json"
WASM_PATH = EWM / "wasm-budget.json"
DECISIONS_PATH = EWM / "owner-decisions.json"
IDENTITY_PATH = ROOT / "site" / "pkg" / "native-world" / "runtime-identity.json"
RECEIPT_PATH = EWM / "ewm00-validation.json"

FIRST_PAINT_IDS = [
    "canberra-reference-terrain",
    "habitat-terrain",
    "prototype-eucalyptus-lush-mid",
    "car-passat-b7--gold",
    "car-passat-b7--glass",
    "car-passat-b7--chrome",
    "vintage-windup-camper--chassis",
    "vintage-windup-camper--rubber",
    "vintage-windup-camper--enamel",
    "vintage-windup-camper--chrome",
    "vintage-windup-camper--ochre",
    "vintage-windup-camper--olive",
    "vintage-windup-camper--roof",
    "vintage-windup-camper--canvas",
    "vintage-windup-camper--screen",
    "vintage-windup-camper--canvas-seam",
    "vintage-windup-camper--screen-wire",
    "vintage-windup-camper--step",
    "vintage-windup-camper--steel",
    "vintage-windup-camper--canvas-roof",
    "vintage-windup-camper--red-lens",
    "vintage-windup-camper--amber-lens",
    "solar-array-three--panel",
    "starlink-mini--dish-front",
]
SUM_BYTES = 16644944
AUTHORED_SUBTOTAL = 14697952
DEM_BYTES = 1946992
BUDGET_BYTES = 17825792
WASM_BYTES = 21625218
WASM_BUDGET_BYTES = 24641536
ZERO_SHA384 = "0" * 96
FORBIDDEN_TOKEN_RE = re.compile(
    r"Timeline\.json|\bplaceIds\b|\bplaceId\b|INFERRED_WORK",
    re.IGNORECASE,
)
A06_CARVE_OUTS = [
    "native/world-host/src/runtime/stream/**",
    "native/world-host/src/runtime/gis_join.rs",
    "native/world-host/src/runtime/gis_ingest.rs",
    "native/world-host/src/runtime/ocs_frames.rs",
    "native/world-host/src/runtime/consent.rs",
    "native/world-host/src/runtime/ui/download.rs",
]
INTEGRATOR_PARENTS = [
    "native/world-host/src/runtime.rs",
    "native/world-host/src/runtime/boot.rs",
    "native/world-host/src/lib.rs",
    "native/world-host/Cargo.toml",
]


class ValidationError(ValueError):
    pass


def fail(path: str, message: str) -> None:
    raise ValidationError(path + ": " + message)


def load_json(path: Path):
    return json.loads(path.read_text(encoding="utf-8"))


def sha256_file(path: Path) -> str:
    return hashlib.sha256(path.read_bytes()).hexdigest()


def resolve_ref(schema, document):
    ref = schema["$ref"]
    if not ref.startswith("#/$defs/"):
        fail("$ref", "external schema references are not supported: " + ref)
    name = ref[8:]
    try:
        return document["$defs"][name]
    except KeyError:
        fail("$ref", "unknown schema reference " + ref)


def validate_value(value, schema, document, path="$"):
    if "$ref" in schema:
        validate_value(value, resolve_ref(schema, document), document, path)
        extra = {k: v for k, v in schema.items() if k not in ("$ref", "description", "title")}
        if extra:
            schema = extra
        else:
            return
    if "not" in schema:
        try:
            validate_value(value, schema["not"], document, path)
        except ValidationError:
            pass
        else:
            fail(path, "value matched a forbidden form")
    if "const" in schema and value != schema["const"]:
        fail(path, "incorrect constant")
    if "enum" in schema and value not in schema["enum"]:
        fail(path, "unknown enum value")
    kind = schema.get("type")
    correct = {
        "object": isinstance(value, dict),
        "array": isinstance(value, list),
        "string": isinstance(value, str),
        "integer": type(value) is int,
        "number": type(value) in (int, float) and not isinstance(value, bool) and math.isfinite(value),
        "boolean": type(value) is bool,
        "null": value is None,
    }
    if kind and not correct.get(kind, False):
        fail(path, "expected " + kind)
    if kind == "object" or isinstance(value, dict) and ("properties" in schema or "required" in schema or "additionalProperties" in schema or "propertyNames" in schema):
        if not isinstance(value, dict):
            fail(path, "expected object")
        for required in schema.get("required", []):
            if required not in value:
                fail(path, "missing " + required)
        names = schema.get("propertyNames")
        if names:
            for key in value:
                validate_value(key, names, document, path + ".propertyName(" + key + ")")
        additional = schema.get("additionalProperties", True)
        properties = schema.get("properties", {})
        for key, item in value.items():
            if key in properties:
                validate_value(item, properties[key], document, path + "." + key)
            elif additional is False:
                fail(path, "unknown property " + key)
            elif isinstance(additional, dict):
                validate_value(item, additional, document, path + "." + key)
    if kind == "array" or isinstance(value, list) and ("items" in schema or "minItems" in schema or "maxItems" in schema or "uniqueItems" in schema):
        if not isinstance(value, list):
            fail(path, "expected array")
        if len(value) < schema.get("minItems", 0):
            fail(path, "array shorter than minItems")
        if len(value) > schema.get("maxItems", math.inf):
            fail(path, "array longer than maxItems")
        if schema.get("uniqueItems") and len(value) != len(set(json.dumps(x, sort_keys=True) for x in value)):
            fail(path, "array items are not unique")
        item_schema = schema.get("items")
        if isinstance(item_schema, dict):
            for i, item in enumerate(value):
                validate_value(item, item_schema, document, path + "[" + str(i) + "]")
    if kind == "string" or isinstance(value, str) and ("pattern" in schema or "minLength" in schema or "maxLength" in schema):
        if not isinstance(value, str):
            fail(path, "expected string")
        if len(value) < schema.get("minLength", 0) or len(value) > schema.get("maxLength", math.inf):
            fail(path, "text length outside bounds")
        if "pattern" in schema and re.search(schema["pattern"], value) is None:
            fail(path, "text does not match pattern")
    if kind in ("integer", "number") and isinstance(value, (int, float)) and not isinstance(value, bool):
        if value < schema.get("minimum", -math.inf) or value > schema.get("maximum", math.inf):
            fail(path, "number outside bounds")


def must_fail(document, schema, why: str) -> str:
    try:
        validate_value(document, schema, schema)
    except ValidationError as error:
        return str(error)
    fail("$", "expected rejection (" + why + ")")
    return ""


def walk_strings(value, path="$"):
    if isinstance(value, dict):
        for key, item in value.items():
            if FORBIDDEN_TOKEN_RE.search(str(key)):
                fail(path, "forbidden key " + str(key))
            walk_strings(item, path + "." + str(key))
    elif isinstance(value, list):
        for i, item in enumerate(value):
            walk_strings(item, path + "[" + str(i) + "]")
    elif isinstance(value, str):
        hit = FORBIDDEN_TOKEN_RE.search(value)
        if hit:
            fail(path, "forbidden token " + hit.group(0))


def schema_forbids_single_hex(schema) -> None:
    names = schema.get("propertyNames", {}).get("not", {}).get("enum", [])
    for token in ("candidate_sha384", "placeIds", "INFERRED_WORK"):
        if token not in names:
            fail("schema.propertyNames", "must reject " + token)
    allowed = schema.get("properties", {})
    if "candidate_sha384" in allowed:
        fail("schema.properties", "candidate_sha384 must not be an allowed field")
    candidate = schema["$defs"]["CandidateIdentity"]
    required = set(candidate["required"])
    if required != {"descriptor_sha384", "context_sha384", "generation", "entries"}:
        fail("schema.candidate", "four-tuple required fields mismatch")
    if "candidate_sha384" in candidate.get("properties", {}):
        fail("schema.candidate", "single-hex field must not be part of CandidateIdentity")


def semantic_fixture(fixture) -> None:
    ids = fixture["first_paint"]
    if ids != FIRST_PAINT_IDS:
        fail("first_paint", "id list does not match the design table")
    if len(ids) != 24:
        fail("first_paint", "expected 24 ids")
    by_id = {row["scene_id"]: row for row in fixture["entries"]}
    if not set(ids).issubset(set(by_id)):
        fail("entries", "first_paint must be a subset of catalog entries")
    extra = set(by_id) - set(ids)
    listed = {row["id"] for row in fixture.get("enhancement_not_first_paint", [])}
    if extra != listed:
        fail(
            "enhancement_not_first_paint",
            "must name every non-first-paint catalog entry",
        )
    for row in fixture["entries"]:
        if row["scene_id"] in ids:
            if row["role"] != "Base":
                fail(row["scene_id"], "first-paint role must be Base")
        else:
            if row["role"] != "Enhancement":
                fail(row["scene_id"], "enhancement role must be Enhancement")
            if row["obligation"] != "Optional":
                fail(row["scene_id"], "enhancement obligation must be Optional")
            if row["shard"] != "enhancement":
                fail(row["scene_id"], "enhancement shard must be enhancement")
    total = sum(by_id[i]["bytes"] for i in ids)
    if total != SUM_BYTES:
        fail("entries", "byte sum " + str(total) + " != " + str(SUM_BYTES))
    if by_id["canberra-reference-terrain"]["bytes"] != DEM_BYTES:
        fail("canberra-reference-terrain", "DEM bytes mismatch")
    authored = total - DEM_BYTES
    if authored != AUTHORED_SUBTOTAL:
        fail("entries", "authored subtotal " + str(authored) + " != " + str(AUTHORED_SUBTOTAL))
    declared = fixture["declared_subset"]
    if declared["bytes"] != SUM_BYTES or declared["ids"] != 24 or declared["v1_budget_bytes"] != BUDGET_BYTES:
        fail("declared_subset", "first-paint budget contract mismatch")
    if declared["authored_subtotal_bytes"] != AUTHORED_SUBTOTAL:
        fail("declared_subset", "authored subtotal mismatch")
    if fixture["inner_ranges"] is not False:
        fail("inner_ranges", "v1 must be false")
    if fixture["streaming"]["default"] is not False:
        fail("streaming.default", "production default must be false")
    if fixture["streaming"]["query_token"] != "ewm=1":
        fail("streaming.query_token", "expected ewm=1")
    if fixture["streaming"]["query_independent_of"] != "gpu-timing=1":
        fail("streaming.query_independent_of", "ewm=1 must be independent of gpu-timing=1")
    if fixture["streaming"]["canvas_attribute"] != "data-ewm-streaming":
        fail("streaming.canvas_attribute", "expected data-ewm-streaming")
    cand = fixture["candidate"]
    for field in ("descriptor_sha384", "context_sha384"):
        if cand[field] == ZERO_SHA384:
            fail("candidate." + field, "zero digest is rejected by from_parts")
        if len(cand[field]) != 96:
            fail("candidate." + field, "SHA-384 must be 96 hex")
    if cand["entries"] != 24:
        fail("candidate.entries", "expected 24 (InventoryVerifier::finish of the first-paint members)")
    dem = by_id["canberra-reference-terrain"]
    if dem["epistemic_class"] != "measured-gis":
        fail("canberra-reference-terrain", "must be measured-gis")
    if not dem["url"].startswith("/packages/"):
        fail("canberra-reference-terrain.url", "first-paint DEM must sit under /packages/")
    if by_id["habitat-terrain"]["epistemic_class"] != "authored-artwork":
        fail("habitat-terrain", "must be authored-artwork")
    place = fixture["places"][0]
    if place["bbox_wsen"] != [149.06, -35.3, 149.08, -35.28]:
        fail("places[0].bbox_wsen", "must be the Canberra public DEM window")
    if place["not_personal_campsite"] is not True:
        fail("places[0]", "not_personal_campsite must be true")
    layer = place["colocated"][0]
    if layer["realm_class"] != "noosphere" or layer["label"] != "infosphere":
        fail("colocated", "infosphere must be Noosphere, not a RealmClass named cyber")
    if "cyber" == layer["realm_class"]:
        fail("colocated.realm_class", "do not invent RealmClass cyber")
    tris = sum(by_id[i]["triangles"] for i in ids)
    verts = sum(by_id[i]["vertices"] for i in ids)
    if tris != declared["triangles"] or verts != declared["vertices"]:
        fail("declared_subset", "GPU subset counts must match entry sums")


def semantic_grant(grant) -> None:
    if grant["merged_by"] != "A00":
        fail("grant.merged_by", "A00 merges the grant")
    if "plans/aaa-overhaul/work-packages.json" not in grant["merged_into"]:
        fail("grant.merged_into", "must target AAA work-packages.json")
    if grant["catalog_home"] != "webizen-website/schemas/ewm/**":
        fail("grant.catalog_home", "catalog is not under native/contracts")
    if grant["catalog_not"] != "native/contracts/**":
        fail("grant.catalog_not", "catalog must not be native/contracts")
    if grant["a00_a27_identifiers_untouched"] is not True:
        fail("grant", "A00-A27 identifiers must stay untouched")
    if grant["runtime_behaviour_change"] is not False:
        fail("grant", "EWM-00 must not change runtime behaviour")
    missing = [p for p in A06_CARVE_OUTS if p not in grant["a06_carve_outs"]]
    if missing:
        fail("grant.a06_carve_outs", "missing " + ", ".join(missing))
    if "ocs_frames.rs" not in " ".join(grant["a06_carve_outs"]):
        fail("grant.a06_carve_outs", "must include ocs_frames.rs")
    parent_paths = [row["path"] for row in grant["integrator_exclusive_for_programme_duration"]]
    missing_parents = [p for p in INTEGRATOR_PARENTS if p not in parent_paths]
    if missing_parents:
        fail("grant.integrator_exclusive", "missing " + ", ".join(missing_parents))
    for row in grant["integrator_exclusive_for_programme_duration"]:
        if row.get("a06") != "read-only":
            fail(row["path"], "A06 must be read-only on integrator parents")
    ewm = " ".join(grant["data_ewm_vs_a02"]["ewm"])
    if "data/ewm/frames/**" not in ewm or "first-paint.json" not in ewm:
        fail("grant.data_ewm_vs_a02", "data/ewm including frames must be listed")
    a02 = grant["data_ewm_vs_a02"]["a02_untouched"]
    for path in ("data/catalogue/**", "data/publications/**"):
        if path not in a02:
            fail("grant.data_ewm_vs_a02", "A02 path missing " + path)


def semantic_wasm(budget, identity) -> None:
    wasm = identity["wasm"]
    if budget["bytes_today"] != WASM_BYTES or wasm["bytes"] != WASM_BYTES:
        fail("wasm-budget", "today bytes must be 21,625,218")
    if budget["ci_budget_bytes"] != WASM_BUDGET_BYTES or budget["ci_budget_mib"] != 23.5:
        fail("wasm-budget", "CI budget must be 23.5 MiB / 24,641,536 bytes")
    if budget["sha256"] != wasm["sha256"]:
        fail("wasm-budget", "SHA-256 must match runtime-identity.json")
    if budget["mib_today"] != 20.63:
        fail("wasm-budget", "today MiB must be 20.63")


def assert_no_runtime_edits() -> None:
    forbidden_writes = [
        ROOT / "native" / "world-host" / "src" / "runtime" / "stream" / "flag.rs",
        ROOT / "plans" / "aaa-overhaul" / "work-packages.json",
    ]
    # flag.rs must not be created by EWM-00
    flag = forbidden_writes[0]
    if flag.exists():
        fail(str(flag), "EWM-00 must not create flag.rs")


def main() -> int:
    schema = load_json(SCHEMA_PATH)
    fixture = load_json(FIXTURE_PATH)
    invalid_hex = load_json(INVALID_HEX_PATH)
    grant = load_json(GRANT_PATH)
    ownership = load_json(OWNERSHIP_PATH)
    wasm_budget = load_json(WASM_PATH)
    decisions = load_json(DECISIONS_PATH)
    identity = load_json(IDENTITY_PATH)

    validate_value(fixture, schema, schema)
    semantic_fixture(fixture)
    semantic_grant(grant)
    semantic_wasm(wasm_budget, identity)
    assert_no_runtime_edits()

    if ownership["single_file_owners"]["webizen-website/native/world-host/src/runtime/stream/flag.rs"] != "EWM-09":
        fail("file-ownership", "flag.rs owner must be EWM-09")
    for key in ("Q1_bearer", "Q2_locality", "Q3_gis_phases", "Q4_portal_unload", "Q5_q42", "Q6_same_origin", "Q7_cosmic_infosphere", "Q8_timeline"):
        if key not in decisions:
            fail("owner-decisions", "missing " + key)

    walk_strings(fixture)
    schema_forbids_single_hex(schema)
    if "cyber" in schema["$defs"]["RealmClass"]["enum"]:
        fail("schema.RealmClass", "do not invent RealmClass cyber")

    hex_error = must_fail(invalid_hex, schema, "single-hex candidate_sha384")
    extra = deepcopy(fixture)
    extra["candidate_sha384"] = extra["candidate"]["descriptor_sha384"]
    extra_error = must_fail(extra, schema, "candidate_sha384 additional property")
    as_string = deepcopy(fixture)
    as_string["candidate"] = extra["candidate"]["descriptor_sha384"]
    string_error = must_fail(as_string, schema, "candidate as a single hex string")
    missing = deepcopy(fixture)
    del missing["candidate"]
    missing["candidate_sha384"] = extra["candidate"]["descriptor_sha384"]
    missing_error = must_fail(missing, schema, "candidate_sha384 instead of four-tuple")

    files = {
        "schemas/ewm/catalog.v1.json": sha256_file(SCHEMA_PATH),
        "data/ewm/first-paint.json": sha256_file(FIXTURE_PATH),
        "plans/ewm/aaa-path-lock-grant.json": sha256_file(GRANT_PATH),
        "plans/ewm/file-ownership.json": sha256_file(OWNERSHIP_PATH),
        "plans/ewm/wasm-budget.json": sha256_file(WASM_PATH),
        "plans/ewm/owner-decisions.json": sha256_file(DECISIONS_PATH),
        "schemas/ewm/fixtures/invalid-single-hex-candidate.json": sha256_file(INVALID_HEX_PATH),
        "plans/ewm/EXPLORABLE-WORLD-MODEL.md": sha256_file(EWM / "EXPLORABLE-WORLD-MODEL.md"),
        "plans/ewm/work-packets.json": sha256_file(EWM / "work-packets.json"),
    }
    receipt = {
        "schema": "webizen.ewm-00-validation/v1",
        "status": "EWM-00 planning contracts validated; no runtime change; AAA grant remains draft until A00 merges it",
        "first_paint_ids": FIRST_PAINT_IDS,
        "first_paint_bytes": SUM_BYTES,
        "authored_subtotal_bytes": AUTHORED_SUBTOTAL,
        "measured_gis_bytes": DEM_BYTES,
        "v1_budget_bytes": BUDGET_BYTES,
        "inner_ranges": False,
        "streaming_default": False,
        "query_token": "ewm=1",
        "canvas_attribute": "data-ewm-streaming",
        "flag_rs_owner": "EWM-09",
        "catalog_home": "schemas/ewm/**",
        "catalog_not": "native/contracts/**",
        "wasm_bytes_today": WASM_BYTES,
        "wasm_ci_budget_bytes": WASM_BUDGET_BYTES,
        "a00_grant_merged": False,
        "runtime_behaviour_change": False,
        "a00_a27_identifiers_untouched": True,
        "rejections": {
            "invalid-single-hex-candidate": hex_error,
            "candidate_sha384_additional_property": extra_error,
            "candidate_hex_string": string_error,
            "candidate_sha384_instead_of_four_tuple": missing_error,
        },
        "file_sha256": files,
    }
    RECEIPT_PATH.write_text(json.dumps(receipt, indent=2) + "\n", encoding="utf-8")
    print(json.dumps({"status": receipt["status"], "first_paint_bytes": SUM_BYTES, "receipt": str(RECEIPT_PATH)}, indent=2))
    return 0


if __name__ == "__main__":
    try:
        sys.exit(main())
    except ValidationError as error:
        print(error, file=sys.stderr)
        sys.exit(1)
