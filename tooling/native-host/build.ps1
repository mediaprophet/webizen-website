param(
  [string]$OutputDir = "$PSScriptRoot/../../site/pkg/native-world",
  [string]$Bindgen = 'C:/Users/Hp/AppData/Local/.wasm-pack/wasm-bindgen-06a9b89f25ab66ba/wasm-bindgen.exe'
)
$ErrorActionPreference = 'Stop'
$crate = [IO.Path]::GetFullPath("$PSScriptRoot/../../native/world-host")
$out = [IO.Path]::GetFullPath($OutputDir)
$oldTarget = $env:CARGO_TARGET_DIR
$oldFlags = $env:RUSTFLAGS
$oldToolchain = $env:RUSTUP_TOOLCHAIN
try {
  $env:CARGO_TARGET_DIR = 'C:/github/qualiaDB/target'
  $env:RUSTUP_TOOLCHAIN = '1.99.1'
  $env:RUSTFLAGS = '-C target-feature=+simd128 -C link-arg=-zstack-size=8388608 -C link-arg=--max-memory=1073741824 --cfg getrandom_backend="wasm_js"'
  if (-not (Test-Path -LiteralPath $Bindgen)) { throw 'Matching wasm-bindgen CLI required; pass its path using -Bindgen' }
  $version = & $Bindgen --version
  if ($version -ne 'wasm-bindgen 0.2.129') { throw "Expected wasm-bindgen 0.2.129, found $version" }
  Push-Location $crate
  try {
    & cargo build --target wasm32-unknown-unknown --release --offline --locked
    if ($LASTEXITCODE -ne 0) { throw 'Native world WASM compilation failed' }
    & $Bindgen 'C:/github/qualiaDB/target/wasm32-unknown-unknown/release/webizen_world_host.wasm' --target web --out-dir $out --out-name webizen_world
    if ($LASTEXITCODE -ne 0) { throw 'Native world binding generation failed' }
  } finally { Pop-Location }
  & python "$PSScriptRoot/receipt.py" --output $out --bindgen $Bindgen
  if ($LASTEXITCODE -ne 0) { throw 'Build receipt generation failed' }
} finally {
  $env:CARGO_TARGET_DIR = $oldTarget
  $env:RUSTFLAGS = $oldFlags
  $env:RUSTUP_TOOLCHAIN = $oldToolchain
}
