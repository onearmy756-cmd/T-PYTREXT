# =============================================================
# MTECH OS — AGENT INSTALLER (Windows PowerShell)
# Mbilinyi Tech · Umiliki ni wako, leseni ni yako, faida ni yako
#
# MATUMIZI (kwenye PowerShell ya admin kwenye kifaa):
#   Set-ExecutionPolicy -Scope Process Bypass -Force
#   .\agent-mtech.ps1 -Server "http://192.168.1.10:8080" -Device "lab_user_101"
#
# Script inafanya:
#   1. Kujisajili kwenye server (inapata TOKEN — inahifadhiwa ndani ya kifaa)
#   2. Heartbeat kila sekunde 20 (afya ya kifaa)
#   3. Poll kazi ZILIZOIDHINISHWA (HITL) — mteja haianzi kitu bila idhini ya admin
#   4. Kutekeleza kazi (usalama: ukaguzi wa kifaa; bundle/install: winget)
#   5. Kuripoti maendeleo (progress 0-100)
# =============================================================
param(
    [Parameter(Mandatory=$true)][string]$Server,
    [Parameter(Mandatory=$false)][string]$Device = $env:COMPUTERNAME,
    [Parameter(Mandatory=$false)][int]$Interval = 20
)

$ErrorActionPreference = "Stop"
$StateDir  = Join-Path $env:ProgramData "MTECH-OS"
$StateFile = Join-Path $StateDir "agent.json"
New-Item -ItemType Directory -Force -Path $StateDir | Out-Null

function Read-State {
    if (Test-Path $StateFile) { return Get-Content $StateFile -Raw | ConvertFrom-Json }
    return $null
}
function Write-State($s) { $s | ConvertTo-Json | Set-Content -Path $StateFile -Encoding UTF8 }

function Invoke-Api($Method, $Path, $Body) {
    $json = if ($null -ne $Body) { $Body | ConvertTo-Json } else { $null }
    Invoke-RestMethod -Uri "$Server$Path" -Method $Method -ContentType "application/json" -Body $json -TimeoutSec 15
}

# ---------- 1. REGISTER (au re-register kama token ipo) ----------
$state = Read-State
if (-not $state -or -not $state.agent_id -or -not $state.token) {
    Write-Host "[MTECH OS] Inajisajili kwenye server: $Server (kifaa: $Device)"
    $reg = Invoke-Api "POST" "/api/fleet/register" @{ device = $Device; os_type = "win11"; health = "nzuri" }
    if (-not $reg.ok) { throw "Usajili umeshindikana: $($reg.error)" }
    $state = [PSCustomObject]@{ agent_id = $reg.agent_id; token = $reg.token; device = $Device }
    Write-State $state
    Write-Host "[MTECH OS] Imesajiliwa — agent_id: $($reg.agent_id)"
} else {
    Write-Host "[MTECH OS] Token ipo — inaendelea (agent_id: $($state.agent_id))"
}

function Get-Health {
    $cpu = (Get-CimInstance Win32_Processor | Measure-Object -Property LoadPercentage -Average).Average
    $mem = [math]::Round((Get-CimInstance Win32_OperatingSystem).FreePhysicalMemory/1MB, 1)
    $disk = (Get-PSDrive C).Free/1GB
    "CPU $cpu% · RAM $mem GB bure · Disk $([math]::Round($disk,0)) GB bure"
}

function Invoke-SecurityCheck {
    # Ukaguzi wa usalama wa ndani (bila zana za nje)
    $fw = (Get-NetFirewallProfile | Where-Object { -not $_.Enabled }).Count
    $av = (Get-CimInstance -Namespace root/SecurityCenter2 -ClassName AntiVirusProduct | Measure-Object).Count
    $findings = @()
    if ($fw -gt 0) { $findings += @{ kind = "problem"; detail = "Firewall imezimwa kwenye profile $fw"; severity = 40 } }
    if ($av -eq 0) { $findings += @{ kind = "problem"; detail = "Hakuna antivirus inayojulikana"; severity = 30 } }
    return $findings
}

function Invoke-Job($job) {
    $stage = $job.stage
    $jid   = $job.id
    try {
        if ($stage -like "secops:*") {
            Invoke-Api "POST" "/api/fleet/report" @{ agent_id = $state.agent_id; token = $state.token; progress = 30; message = "Ukaguzi wa usalama unaendelea" } | Out-Null
            $f = Invoke-SecurityCheck
            # Tuma matokeo kwenye secops (ripoti rasmi ya usalama)
            $body = @{ account = "mteja1"; target = $Device; mode = "security"; findings = $f; sources = "mfumo"; note = "agent" }
            Invoke-Api "POST" "/api/secops/result" $body | Out-Null
            Invoke-Api "POST" "/api/fleet/report" @{ agent_id = $state.agent_id; token = $state.token; progress = 100; message = "Ukaguzi umekamilika" } | Out-Null
        }
        elseif ($stage -like "bundle:*" -or $stage -like "deploy*") {
            Invoke-Api "POST" "/api/fleet/report" @{ agent_id = $state.agent_id; token = $state.token; progress = 20; message = "Usakinishaji unaendelea" } | Out-Null
            # Apps HALISI kutoka server (winget) — kila amri inatekelezwa kwa mfululizo.
            $apps = @($job.apps)
            if ($apps.Count -eq 0) {
                # server haikutuma apps (bundle isiyojulikana) — usakinishaji wa msingi salama
                $apps = @("winget install --id Microsoft.PowerToys --silent --accept-package-agreements --accept-source-agreements")
            }
            $total = $apps.Count
            for ($i = 0; $i -lt $total; $i++) {
                $c = $apps[$i]
                $pct = 20 + [int](($i + 1) * 70 / $total)
                Invoke-Api "POST" "/api/fleet/report" @{ agent_id = $state.agent_id; token = $state.token; progress = $pct; message = "App $($i+1)/$total" } | Out-Null
                cmd.exe /c $c 2>$null
            }
            Invoke-Api "POST" "/api/fleet/report" @{ agent_id = $state.agent_id; token = $state.token; progress = 100; message = "Usakinishaji umekamilika" } | Out-Null
        }
        else {
            Invoke-Api "POST" "/api/fleet/report" @{ agent_id = $state.agent_id; token = $state.token; progress = 100; message = "Kazi haijatambulika — imeghairi" } | Out-Null
        }
    } catch {
        Invoke-Api "POST" "/api/fleet/report" @{ agent_id = $state.agent_id; token = $state.token; progress = 100; message = "Kosa: $($_.Exception.Message)" } | Out-Null
    }
}

# ---------- 2-5. LOOP KUU ----------
Write-Host "[MTECH OS] Agent inaendesha — Ctrl+C kuzima"
while ($true) {
    try {
        # Heartbeat
        $hb = Invoke-Api "POST" "/api/fleet/heartbeat" @{ agent_id = $state.agent_id; token = $state.token; health = (Get-Health) }
        # Poll kazi
        $p = Invoke-Api "POST" "/api/fleet/poll" @{ agent_id = $state.agent_id; token = $state.token }
        if ($p.ok -and $null -ne $p.job) {
            Write-Host "[MTECH OS] Kazi mpya: $($p.job.id) ($($p.job.stage))"
            Invoke-Job $p.job
        }
    } catch {
        Write-Host "[MTECH OS] Jaribio linalofuata litarejea: $($_.Exception.Message)"
    }
    Start-Sleep -Seconds $Interval
}
