$ErrorActionPreference = 'Stop'
$env:CARGO_MANIFEST_DIR = 'C:/github/etalon/benches'
$root = 'C:/github/synth/production/webizen-swarm/tooling/native-host'
$candidate = Join-Path $root 'compute-workload-candidate'
New-Item -ItemType Directory -Force -Path $candidate | Out-Null
$deps = 'C:/github/etalon/target-webizen-tests/debug/deps'
$common = @('--edition=2021','-L',"dependency=$deps",'--cfg','etalon_validate_locks_debug')
Get-ChildItem C:/github/etalon/target-webizen-tests/debug/build -Directory | ForEach-Object {
    $out = Join-Path $_.FullName 'out'
    if (Test-Path -LiteralPath $out) { $common += @('-L',"native=$out") }
    $buildOutput = Join-Path $_.FullName 'output'
    if (Test-Path -LiteralPath $buildOutput) {
        foreach ($line in Get-Content -LiteralPath $buildOutput) {
            if ($line -match '^cargo::?rustc-link-search=(.+)$') { $common += @('-L',$Matches[1]) }
        }
    }
}
$fingerprint = Get-Content -Raw C:/github/etalon/target-webizen-tests/debug/.fingerprint/etalon-benchmark-7dadce70023a7b1c/test-bench-etalon-benchmark.json | ConvertFrom-Json
$externs = @()
$pins = @()
foreach ($dependency in $fingerprint.deps) {
    $name = $dependency[1]
    $lib = Get-ChildItem -LiteralPath $deps -Filter "lib$name-*.rlib" | Sort-Object LastWriteTime -Descending | Select-Object -First 1
    if (-not $lib) { throw "Missing original dependency $name" }
    $externs += @('--extern',"$name=$($lib.FullName)")
    $pins += @{name=$name;file=$lib.FullName;sha256=(Get-FileHash -LiteralPath $lib.FullName -Algorithm SHA256).Hash.ToLowerInvariant()}
}
$pins | ConvertTo-Json | Set-Content -Encoding UTF8 (Join-Path $candidate 'original-dependencies.json')
& rustup run 1.96.1 rustc @common @externs --test C:/github/etalon/benches/tests/compute_shader_contract.rs -o (Join-Path $candidate 'shader-contract.exe')
if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }
& (Join-Path $candidate 'shader-contract.exe') 2>&1 | Tee-Object -FilePath (Join-Path $candidate 'shader-contract.log')
if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }
& rustup run 1.96.1 rustc @common @externs --cfg 'feature="tracy"' --crate-name etalon_benchmark_candidate C:/github/etalon/benches/benches/etalon-benchmark/main.rs -o (Join-Path $candidate 'compute-workload.exe')
exit $LASTEXITCODE
