$ErrorActionPreference = 'Continue'
Set-Location C:/github/etalon
$env:RUSTUP_TOOLCHAIN = '1.96.1'
$env:CARGO_TARGET_DIR = 'C:/github/etalon/target-webizen-tests'
$env:CARGO_BUILD_JOBS = '2'
$env:NEXTEST_TEST_THREADS = '1'
$env:ETALON_ADAPTER_NAME = 'Intel(R) RaptorLake-S Mobile Graphics Controller'
Remove-Item Env:ETALON_BACKEND -ErrorAction SilentlyContinue
$env:PATH = 'C:/github/etalon/target-webizen-tests/tools/nextest;C:/VulkanSDK/1.4.363.0/Bin;' + $env:PATH
$filter = '(binary(etalon-gpu) & test(clear_buffer_range_respected) & test([Gl/)) | (binary(etalon-benchmark) & test(=Computepass Encoding)) | test(gles::fence::fence_wait::tests::) | binary(compute_shader_contract)'
$root = 'C:/github/synth/production/webizen-swarm/tooling/native-host'
$started = [DateTime]::UtcNow.ToString('o')
& cargo xtask test -E $filter 2>&1 | Tee-Object -FilePath (Join-Path $root 'etalon-combined-focused.log')
$result = $LASTEXITCODE
@{schema='webizen.combined-focused-tests/v1';command='cargo xtask test -E';filter=$filter;started_utc=$started;finished_utc=[DateTime]::UtcNow.ToString('o');exit_code=$result;provider='reviewed GL wait and benchmark workload candidate on785ab9c';expected_focused_cases=15;full_suite=$false;test_workers=1;build_workers=2} | ConvertTo-Json | Tee-Object -FilePath (Join-Path $root 'etalon-combined-focused.json')
exit $result
