# Build the scoped helper against the same native codec library as the installed producer.
# This links existing QualiaDB APIs; it does not reimplement their format or bypass policy.
$base = $PSScriptRoot
& rustc --edition 2021 "$base/main.rs" --extern qualia_core_db=C:/github/synth/target-prod/debug/deps/libqualia_core_db.rlib -L dependency=C:/github/synth/target-prod/debug/deps -o "$base/attest.exe"
if ($LASTEXITCODE -ne 0) { throw 'Native provenance helper compilation failed' }
