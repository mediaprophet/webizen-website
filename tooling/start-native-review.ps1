param(
    [ValidateRange(1024,65535)][int]$Port = 8802,
    [string]$SynthServer = 'C:\github\synth\target-prod\debug\webizen-studio.exe'
)
$ErrorActionPreference = 'Stop'
$reviewRoot = [IO.Path]::GetFullPath((Join-Path $PSScriptRoot '..'))
$reviewLogs = Join-Path $reviewRoot 'evidence/aaa-server'
New-Item -ItemType Directory -Path $reviewLogs -Force | Out-Null
$reviewUrl = "http://127.0.0.1:$Port/aaa-proof.html"
$reviewListener = [System.Net.NetworkInformation.IPGlobalProperties]::GetIPGlobalProperties().GetActiveTcpListeners() | Where-Object Port -eq $Port
if ($reviewListener) { try {
    $response = Invoke-WebRequest -Uri $reviewUrl -TimeoutSec 2
    if ($response.StatusCode -eq 200 -and $response.Content.Contains('data-scene="/production/aaa-proof/scene.json"')) {
        $capabilities = Invoke-RestMethod -Uri "http://127.0.0.1:$Port/api/capabilities" -TimeoutSec 15
        if (-not $capabilities.morphisms) { throw 'Listener does not expose the native Synth capabilities.' }
        Write-Output "Native Synth review is already available: $reviewUrl"
        exit 0
    }
    throw "Port $Port already serves another application. Choose a different -Port."
} catch [System.Net.Http.HttpRequestException] {
    # No listener; launch Synth with this repository attached.
} catch [System.Threading.Tasks.TaskCanceledException] {
    throw "Port $Port did not respond; choose a different -Port."
} }
if (-not (Test-Path -LiteralPath $SynthServer -PathType Leaf)) {
    throw "Build Synth's webizen-studio binary first. Expected: $SynthServer"
}
$reviewProcess = Start-Process -FilePath $SynthServer -ArgumentList @('--website-root',('"{0}"' -f $reviewRoot),'--port',"$Port") -WorkingDirectory $reviewRoot -WindowStyle Hidden -PassThru -RedirectStandardOutput (Join-Path $reviewLogs "stdout-$Port.log") -RedirectStandardError (Join-Path $reviewLogs "stderr-$Port.log")
for ($attempt=0; $attempt -lt 20; $attempt++) {
    if ($reviewProcess.HasExited) { throw "Native server exited. Inspect $reviewLogs." }
    try {
        $response = Invoke-WebRequest -Uri $reviewUrl -TimeoutSec 1
        if ($response.StatusCode -eq 200) {
            Write-Output "Native review ready: $reviewUrl (process $($reviewProcess.Id))"
            exit 0
        }
    } catch { Start-Sleep -Milliseconds 250 }
}
throw "Native review did not start. Inspect $reviewLogs."
