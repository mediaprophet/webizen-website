param([ValidateSet('gl','vulkan')][string]$Case)
$ErrorActionPreference = 'Continue'
Set-Location C:/github/etalon
$env:ETALON_DX12_AGILITY_SDK_PATH = 'C:/github/etalon/target/agility-sdk/build/native/bin/x64'
$env:ETALON_DX12_AGILITY_SDK_VERSION = '619'
$env:ETALON_DX12_AGILITY_SDK_REQUIRE = '1'
$env:PATH = 'C:/VulkanSDK/1.4.363.0/Bin;' + $env:PATH
$env:RUST_BACKTRACE = '1'
$evidenceRoot = 'C:/github/synth/production/webizen-swarm/tooling/native-host'
if ($Case -eq 'gl') {
    $exe = 'C:/github/etalon/target-webizen-tests/debug/deps/etalon_gpu-4405d1164bc21cd6.exe'
    $testName = '[Executed] [Gl/Intel(R) RaptorLake-S Mobile Graphics Controller/5] etalon_gpu::regression::issue_4122::clear_buffer_range_respected'
    $arguments = @('--exact', $testName, '--nocapture')
} else {
    $env:ETALON_BACKEND = 'vulkan'
    $env:ETALON_ADAPTER_NAME = 'Intel(R) RaptorLake-S Mobile Graphics Controller'
    $exe = 'C:/github/etalon/target-webizen-tests/debug/deps/etalon_benchmark-7dadce70023a7b1c.exe'
    $testName = 'Computepass Encoding'
    $arguments = @('--exact', $testName)
}
$beforeHash = (Get-FileHash -LiteralPath $exe -Algorithm SHA256).Hash.ToLowerInvariant()
$started = [DateTime]::UtcNow.ToString('o')
$watch = [Diagnostics.Stopwatch]::StartNew()
& $exe @arguments 2>&1 | Tee-Object -FilePath (Join-Path $evidenceRoot "etalon-focused-original-$Case.log")
$resultCode = $LASTEXITCODE
$watch.Stop()
@{schema='webizen.focused-original-test/v1';case=$Case;revision='785ab9c1b7d729db9afcd9b0f3fd70344b37be1e';binary=$exe;binary_sha256=$beforeHash;test=$testName;started_utc=$started;finished_utc=[DateTime]::UtcNow.ToString('o');seconds=$watch.Elapsed.TotalSeconds;exit_code=$resultCode;rebuilt=$false;other_gpu_work_held=$true} | ConvertTo-Json | Tee-Object -FilePath (Join-Path $evidenceRoot "etalon-focused-original-$Case.json")
exit $resultCode
