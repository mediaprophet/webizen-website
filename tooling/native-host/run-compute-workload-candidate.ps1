$ErrorActionPreference = 'Continue'
Set-Location C:/github/etalon
$env:ETALON_DX12_AGILITY_SDK_PATH = 'C:/github/etalon/target/agility-sdk/build/native/bin/x64'
$env:ETALON_DX12_AGILITY_SDK_VERSION = '619'
$env:ETALON_DX12_AGILITY_SDK_REQUIRE = '1'
$env:ETALON_BACKEND = 'vulkan'
$env:ETALON_ADAPTER_NAME = 'Intel(R) RaptorLake-S Mobile Graphics Controller'
$env:PATH = 'C:/VulkanSDK/1.4.363.0/Bin;' + $env:PATH
$env:RUST_BACKTRACE = '1'
$candidate = 'C:/github/synth/production/webizen-swarm/tooling/native-host/compute-workload-candidate'
$exe = Join-Path $candidate 'compute-workload.exe'
$sources = @('benches/benches/etalon-benchmark/computepass.rs','benches/benches/etalon-benchmark/computepass.wgsl','benches/benches/etalon-benchmark/computepass-bindless.wgsl') | ForEach-Object { @{path=$_;sha256=(Get-FileHash -LiteralPath (Join-Path C:/github/etalon $_) -Algorithm SHA256).Hash.ToLowerInvariant()} }
$started = [DateTime]::UtcNow.ToString('o')
$watch = [Diagnostics.Stopwatch]::StartNew()
& $exe --exact 'Computepass Encoding' 2>&1 | Tee-Object -FilePath (Join-Path $candidate 'intel-vulkan.log')
$resultCode = $LASTEXITCODE
$watch.Stop()
@{schema='webizen.corrected-workload-test/v1';provider_revision='785ab9c1b7d729db9afcd9b0f3fd70344b37be1e';provider_libraries_rebuilt=$false;candidate_workload_sources=$sources;binary_sha256=(Get-FileHash -LiteralPath $exe -Algorithm SHA256).Hash.ToLowerInvariant();test='Computepass Encoding';adapter=$env:ETALON_ADAPTER_NAME;backend=$env:ETALON_BACKEND;started_utc=$started;finished_utc=[DateTime]::UtcNow.ToString('o');seconds=$watch.Elapsed.TotalSeconds;exit_code=$resultCode;other_gpu_work_held=$true} | ConvertTo-Json -Depth 5 | Tee-Object -FilePath (Join-Path $candidate 'intel-vulkan.json')
exit $resultCode
