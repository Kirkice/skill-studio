$ErrorActionPreference = 'Stop'
$exe = Join-Path $PSScriptRoot 'target/release/skill-studio-gpui.exe'
$p = Start-Process -FilePath $exe -PassThru
try {
    Start-Sleep -Seconds 5
    $p.Refresh()
    if ($p.HasExited) { throw "Application exited during startup: $($p.ExitCode)" }
    if ($p.MainWindowHandle -eq 0) { throw 'Application did not create a main window' }
    Write-Output "Startup smoke passed: window=$($p.MainWindowHandle)"
} finally {
    if (-not $p.HasExited) { Stop-Process -Id $p.Id }
}
