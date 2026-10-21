# MatchPoint Windows bootstrap. The application runs inside Ubuntu on WSL2.
[CmdletBinding()]
param(
    [ValidatePattern('^[A-Za-z0-9][A-Za-z0-9._-]{0,63}$')]
    [string]$Distribution = 'Ubuntu',
    [ValidatePattern('^[A-Za-z0-9][A-Za-z0-9._/-]{0,199}$')]
    [string]$Ref = 'main'
)

$ErrorActionPreference = 'Stop'

function Invoke-WslChecked {
    param([string[]]$WslArguments)
    & wsl.exe @WslArguments
    if ($LASTEXITCODE -ne 0) { throw "WSL command failed (exit $LASTEXITCODE)." }
}

function Install-MatchPointWsl {
    if ($env:OS -ne 'Windows_NT') { throw 'Run this bootstrap in Windows PowerShell.' }
    if (-not (Get-Command wsl.exe -ErrorAction SilentlyContinue)) {
        throw 'WSL is unavailable. Install WSL from Microsoft, then rerun this script.'
    }
    $installed = @(& wsl.exe --list --quiet 2>$null) -replace "`0", ''
    if ($LASTEXITCODE -ne 0 -or $Distribution -notin @($installed | ForEach-Object { $_.Trim() })) {
        Write-Host "Installing $Distribution with WSL2. Windows may request elevation or a restart."
        & wsl.exe --install --distribution $Distribution --no-launch
        if ($LASTEXITCODE -ne 0) {
            throw 'Complete WSL installation/restart, then rerun this script.'
        }
        throw "Open $Distribution once to create your Linux user, exit, then rerun this script."
    }
    Invoke-WslChecked -WslArguments @('--set-version', $Distribution, '2')
    $userId = & wsl.exe --distribution $Distribution --exec id -u
    if ($LASTEXITCODE -ne 0 -or "$userId".Trim() -eq '0') {
        throw "Open $Distribution and finish creating your normal Linux user, then rerun."
    }
    # Ref is validated above and is passed as a separate bash argument.
    $bootstrap = @'
set -eu
if ! command -v curl >/dev/null 2>&1; then
    sudo apt-get update
    sudo apt-get install -y curl ca-certificates
fi
installer=$(mktemp)
trap 'rm -f "$installer"' EXIT
curl --proto '=https' --tlsv1.2 -fsSL --connect-timeout 15 --max-time 180 \
    https://raw.githubusercontent.com/osvaldoandrade/matchpoint-install/main/install.sh -o "$installer"
bash "$installer" --ref "$1"
'@
    $bootstrap = $bootstrap -replace "`r", ''
    Invoke-WslChecked -WslArguments @('--distribution', $Distribution, '--exec', 'bash', '-c', $bootstrap, 'matchpoint', $Ref)
    Write-Host "Installed. Open $Distribution and run: matchpoint tui"
    Write-Host 'Files on Windows are available under /mnt/c; keep application state in the Linux home directory.'
}

if ($MyInvocation.InvocationName -ne '.') { Install-MatchPointWsl }
