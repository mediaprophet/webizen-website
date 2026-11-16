$base=$PSScriptRoot
$deps='C:/github/synth/target-prod/debug/deps'
$serde=(Get-ChildItem "$deps/libserde_json-*.rlib" | Select-Object -First 1).FullName
& rustc --edition 2021 "$base/admit.rs" --extern "qualia_core_db=$deps/libqualia_core_db.rlib" --extern "serde_json=$serde" -L "dependency=$deps" -o "$base/admit.exe"
if($LASTEXITCODE -ne 0){throw 'Current EMF native admission compilation failed'}
& "$base/admit.exe" "$base/assets/light-field.emf" | Set-Content -Encoding utf8 "$base/admission.json"
if($LASTEXITCODE -ne 0){throw 'Current EMF native admission failed'}
