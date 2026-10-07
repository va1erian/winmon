# Builds the release binary and copies it to dist\winmon-setup.exe.
#
# winmon is its own installer: started under a name containing "setup" (and
# no arguments), it opens the setup window instead of the dashboard. So the
# distributable is the same exe under another name; nothing else is needed.
#
#   powershell -ExecutionPolicy Bypass -File scripts\package.ps1 [-Features nvml]

param([string]$Features = "")

$ErrorActionPreference = "Stop"
$root = Split-Path -Parent $PSScriptRoot
Push-Location $root
try {
    $cargoArgs = @("build", "--release")
    if ($Features) { $cargoArgs += @("--features", $Features) }
    & cargo @cargoArgs
    if ($LASTEXITCODE -ne 0) { throw "cargo build failed" }

    New-Item -ItemType Directory -Force dist | Out-Null
    $out = Join-Path $root "dist\winmon-setup.exe"
    Copy-Item target\release\winmon.exe $out -Force
    $size = [math]::Round((Get-Item $out).Length / 1MB, 2)
    Write-Host "dist\winmon-setup.exe ($size MB)"
} finally {
    Pop-Location
}
