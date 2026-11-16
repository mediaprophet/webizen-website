$ErrorActionPreference = 'Continue'
$env:RUSTUP_TOOLCHAIN = '1.96.1'
$env:CARGO_TARGET_DIR = 'C:/github/etalon/target-webizen-tests'
$env:CARGO_BUILD_JOBS = '2'
$env:NEXTEST_TEST_THREADS = '2'
$env:GIT_OPTIONAL_LOCKS = '0'
$env:PATH = 'C:/github/etalon/target-webizen-tests/tools/nextest;C:/VulkanSDK/1.4.363.0/Bin;' + $env:PATH
Set-Location C:/github/etalon
$runStamp = Get-Date -Format 'yyyyMMdd-HHmmss'
$evidenceRoot = 'C:/github/synth/production/webizen-swarm/tooling/native-host'
$runLog = Join-Path $evidenceRoot "etalon-full-resume-$runStamp.log"
$revision = (& git rev-parse HEAD).Trim()
$started = [DateTime]::UtcNow.ToString('o')
& cargo xtask test 2>&1 | Tee-Object -FilePath $runLog
$resultCode = $LASTEXITCODE
@{command='cargo xtask test'; revision=$revision; started_utc=$started; finished_utc=[DateTime]::UtcNow.ToString('o'); toolchain=$env:RUSTUP_TOOLCHAIN; cargo_build_jobs=2; nextest_test_threads=2; target_dir=$env:CARGO_TARGET_DIR; exit_code=$resultCode; log=$runLog} | ConvertTo-Json | Set-Content -Encoding UTF8 (Join-Path $evidenceRoot "etalon-full-resume-$runStamp.json")
exit $resultCode
