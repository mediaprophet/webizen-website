# Native material identity adapter

`from namespace import namespace_asset`

`namespace_asset(record, input_path, output_path, *, output_url=None) -> dict`

Put `production/aaa-proof/materials` on the Python import path. Pass the original asset record and its actual file, and a new derivative path beneath `production/aaa-proof/assets`. The wrapper returns the updated record and writes a native verification receipt beside the derivative. Existing derivatives are reused only when input, output and helper identities match. The source is never overwritten.

Build with `materials/build.ps1`. This links the installed producer's actual QualiaDB native container/material codecs. The helper normalises each material ID to1, canonically encodes a native material section with one dummy triangle range, hashes it using `CborLdManifest::compute_sha256_digest`, and sets the newID to the first64digestbits OR the highbit. Submesh ranges are remapped. Identical materials share an ID; unequal canonical records colliding on an ID fail closed. The wrapper checks collisions across assets using exact canonical bytes in the receipt registry.

Every non-material section remains byte-exact, including source provenance, geometry, UV, normals and tangents. Header policy flags remain unchanged. Native decode verifies unchanged geometry/counts/attributes and admitted governance. Texture digests, transforms and all non-ID material properties remain unchanged. The renderer and its duplicate-ID refusal are not modified.
