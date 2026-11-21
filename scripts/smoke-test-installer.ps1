<#
.SYNOPSIS
  Installer smoke test: install the DeskPilot MSI silently, start the app, check it keeps
  running, then uninstall and check nothing is left behind.

.DESCRIPTION
  Meant for a fresh Windows machine (the GitHub Actions runner). It needs administrator
  rights, because the MSI installs per machine. It does not check how the app looks.

.PARAMETER MsiPath
  Path to the MSI. Default: the single MSI under target/release/bundle/msi.

.PARAMETER LogDir
  Folder for the msiexec logs. Created if missing.

.PARAMETER RunSeconds
  How long DeskPilot must stay running after start.
#>
param(
    [string]$MsiPath,
    [string]$LogDir = (Join-Path $PWD 'smoke-logs'),
    [int]$RunSeconds = 20
)

$ErrorActionPreference = 'Stop'
Set-StrictMode -Version Latest

function Step([string]$Message) { Write-Output "==> $Message" }
function Fail([string]$Message) { Write-Error "SMOKE TEST FAILED: $Message"; exit 1 }

# Uninstall entries for DeskPilot in the 64- and 32-bit machine-wide registry views.
function Get-DeskPilotUninstallEntries {
    $roots = @(
        'HKLM:\SOFTWARE\Microsoft\Windows\CurrentVersion\Uninstall\*',
        'HKLM:\SOFTWARE\WOW6432Node\Microsoft\Windows\CurrentVersion\Uninstall\*'
    )
    foreach ($root in $roots) {
        Get-ItemProperty -Path $root -ErrorAction SilentlyContinue |
            Where-Object { $_.PSObject.Properties['DisplayName'] -and $_.DisplayName -like 'deskpilot*' }
    }
}

# msiexec exit codes: 0 = success, 3010 = success but a restart is needed.
function Invoke-Msiexec([string[]]$Arguments, [string]$What) {
    $proc = Start-Process -FilePath 'msiexec.exe' -ArgumentList $Arguments -Wait -PassThru
    if ($proc.ExitCode -notin @(0, 3010)) {
        Fail "$What failed with msiexec exit code $($proc.ExitCode). See the logs in $LogDir."
    }
    Write-Output "$What succeeded (msiexec exit code $($proc.ExitCode))."
}

New-Item -ItemType Directory -Force -Path $LogDir | Out-Null

# 1. Find the MSI
if (-not $MsiPath) {
    $found = @(Get-ChildItem -Path 'target/release/bundle/msi' -Filter '*.msi' -ErrorAction SilentlyContinue)
    if ($found.Count -ne 1) { Fail "Expected exactly one MSI under target/release/bundle/msi, found $($found.Count)." }
    $MsiPath = $found[0].FullName
}
if (-not (Test-Path -LiteralPath $MsiPath -PathType Leaf)) { Fail "MSI not found: $MsiPath" }
$MsiPath = (Resolve-Path -LiteralPath $MsiPath).Path
Step "MSI: $MsiPath ($((Get-Item -LiteralPath $MsiPath).Length) bytes)"

# The test must start from a machine without DeskPilot, or it proves nothing.
if (@(Get-DeskPilotUninstallEntries).Count -gt 0) { Fail 'DeskPilot is already installed on this machine.' }

# 2. Install silently
Step 'Installing'
Invoke-Msiexec @('/i', "`"$MsiPath`"", '/qn', '/norestart', '/l*v', "`"$(Join-Path $LogDir 'install.log')`"") 'Install'

# 3. Find the installed executable
$entries = @(Get-DeskPilotUninstallEntries)
if ($entries.Count -ne 1) { Fail "Expected one DeskPilot uninstall entry after install, found $($entries.Count)." }
$entry = $entries[0]
Step "Installed: $($entry.DisplayName) $($entry.DisplayVersion)"

$candidates = @()
if ($entry.PSObject.Properties['InstallLocation'] -and $entry.InstallLocation) {
    $candidates += Join-Path $entry.InstallLocation 'deskpilot.exe'
}
$candidates += Join-Path $env:ProgramFiles 'deskpilot\deskpilot.exe'
$exe = $candidates | Where-Object { Test-Path -LiteralPath $_ -PathType Leaf } | Select-Object -First 1
if (-not $exe) { Fail "deskpilot.exe not found. Looked in: $($candidates -join ', ')" }
Step "Executable: $exe"

# 4. Start the app and check it keeps running
Step "Starting DeskPilot and watching it for $RunSeconds seconds"
$app = Start-Process -FilePath $exe -PassThru
Start-Sleep -Seconds $RunSeconds
$app.Refresh()
if ($app.HasExited) { Fail "DeskPilot exited during the first $RunSeconds seconds (exit code $($app.ExitCode))." }
Write-Output "DeskPilot is still running (PID $($app.Id))."

# Closing the window only hides DeskPilot to the tray, so stop the process directly.
Get-Process -Name 'deskpilot' -ErrorAction SilentlyContinue | Stop-Process -Force
Start-Sleep -Seconds 2
if (Get-Process -Name 'deskpilot' -ErrorAction SilentlyContinue) { Fail 'DeskPilot could not be stopped before uninstall.' }

# 5. Uninstall and check nothing is left behind
Step 'Uninstalling'
Invoke-Msiexec @('/x', "`"$MsiPath`"", '/qn', '/norestart', '/l*v', "`"$(Join-Path $LogDir 'uninstall.log')`"") 'Uninstall'

if (@(Get-DeskPilotUninstallEntries).Count -gt 0) { Fail 'The uninstall entry is still present after uninstall.' }
if (Test-Path -LiteralPath $exe) { Fail "The executable is still present after uninstall: $exe" }

# DeskPilot only adds a startup entry when the user turns on "Start with Windows".
$run = Get-ItemProperty -Path 'HKCU:\Software\Microsoft\Windows\CurrentVersion\Run' -ErrorAction SilentlyContinue
if ($run -and $run.PSObject.Properties['DeskPilot']) { Fail 'A DeskPilot startup entry exists, but nothing turned it on.' }

Step 'Smoke test passed: install, start, stay running, uninstall, clean.'
