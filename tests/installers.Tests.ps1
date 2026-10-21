$ErrorActionPreference = 'Stop'
. "$PSScriptRoot/../install.ps1"

function Assert-Contract([bool]$Condition, [string]$Message) {
    if (-not $Condition) { throw $Message }
}

$previousOS = $env:OS
$env:OS = 'Windows_NT'
$script:Scenario = 'ready'
$script:Calls = [System.Collections.Generic.List[object]]::new()
function wsl.exe {
    $script:Calls.Add(@($args))
    $global:LASTEXITCODE = 0
    if ($args[0] -eq '--list') {
        if ($script:Scenario -ne 'missing') { "U`0b`0u`0n`0t`0u`0" }
    } elseif ($args[0] -eq '--set-version') {
        if ($script:Scenario -eq 'conversion-failed') { $global:LASTEXITCODE = 1 }
    } elseif ($args -contains 'id') {
        if ($script:Scenario -eq 'root') { '0' } else { '1000' }
    }
}

try {
    Install-MatchPointWsl
    $bash = @($script:Calls | Where-Object { $_ -contains 'bash' })
    Assert-Contract ($bash.Count -eq 1) 'Expected one Linux installation.'
    Assert-Contract ($bash[0][-1] -eq 'main') 'Source ref must be a separate argument.'
    Assert-Contract ($bash[0][-3] -match '--proto') 'Bootstrap must restrict download protocols.'
    foreach ($scenario in @('missing', 'root', 'conversion-failed')) {
        $script:Scenario = $scenario
        $script:Calls.Clear()
        $failed = $false
        try { Install-MatchPointWsl } catch { $failed = $true }
        Assert-Contract $failed "Expected clear failure for $scenario."
        Assert-Contract (@($script:Calls | Where-Object { $_ -contains 'bash' }).Count -eq 0) `
            'Do not install after WSL setup or conversion fails.'
    }
    $failed = $false
    try { . "$PSScriptRoot/../install.ps1" -Ref 'main;touch injected' } catch { $failed = $true }
    Assert-Contract $failed 'Invalid refs must be rejected before entering WSL.'
    Write-Host 'PowerShell installer contracts: 5 passed.'
} finally {
    $env:OS = $previousOS
    Remove-Item Function:wsl.exe
}
