$base=$PSScriptRoot
$deps='C:/github/synth/target-prod/debug/deps'
$serde=(Get-ChildItem "$deps/libserde_json-*.rlib" | Select-Object -First 1).FullName
& rustc --edition 2021 "$base/main.rs" --extern "qualia_core_db=$deps/libqualia_core_db.rlib" --extern "serde_json=$serde" -L "dependency=$deps" -o "$base/material-namespace.exe"
if($LASTEXITCODE -ne 0){throw 'Material namespace compilation failed'}
& rustc --edition 2021 --test "$base/main.rs" --extern "qualia_core_db=$deps/libqualia_core_db.rlib" --extern "serde_json=$serde" -L "dependency=$deps" -o "$base/material-namespace-tests.exe"
if($LASTEXITCODE -ne 0){throw 'Material namespace test compilation failed'}
& "$base/material-namespace-tests.exe"
if($LASTEXITCODE -ne 0){throw 'Material namespace tests failed'}
