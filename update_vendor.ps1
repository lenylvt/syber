# =============================================================================
# update_vendor.ps1 — Mise à jour / sauvegarde des dépôts Kyber (core + deps)
# =============================================================================
# Usage:
#   .\update_vendor.ps1              Met à jour tous les repos
#   .\update_vendor.ps1 -Core        Met à jour uniquement les core repos
#   .\update_vendor.ps1 -Deps        Met à jour uniquement les deps
#   .\update_vendor.ps1 -Check       Vérifie si des mises à jour sont disponibles
#   .\update_vendor.ps1 -Snapshot    Crée une archive zip de sauvegarde
#   .\update_vendor.ps1 -Status      Affiche l'état de chaque repo
#   .\update_vendor.ps1 -Help        Affiche l'aide
# =============================================================================
[CmdletBinding()]
param(
    [switch]$Core,
    [switch]$Deps,
    [switch]$Check,
    [switch]$Snapshot,
    [switch]$Status,
    [switch]$Help
)

Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'

$SCRIPT_DIR   = Split-Path -Parent $MyInvocation.MyCommand.Path
$VENDOR_DIR   = Join-Path $SCRIPT_DIR 'vendor'
$DEPS_DIR     = Join-Path $VENDOR_DIR 'deps'
$SNAPSHOT_DIR = Join-Path $VENDOR_DIR '.snapshots'
$LOG_FILE     = Join-Path $VENDOR_DIR '.update.log'

$CORE_REPOS = @(
    @{ Name = 'kyutil';  Url = 'https://gitlab.com/kyber/core/kyutil.git'  }
    @{ Name = 'kymux';   Url = 'https://gitlab.com/kyber/core/kymux.git'   }
    @{ Name = 'kymedia'; Url = 'https://gitlab.com/kyber/core/kymedia.git' }
    @{ Name = 'kynput';  Url = 'https://gitlab.com/kyber/core/kynput.git'  }
    @{ Name = 'kysdk';   Url = 'https://gitlab.com/kyber/core/kysdk.git'   }
)

$DEPS_REPOS = @(
    @{ Name = 'keycode';      Url = 'https://gitlab.com/kyber/deps/keycode.git'      }
    @{ Name = 'libudev-sys';  Url = 'https://gitlab.com/kyber/deps/libudev-sys.git'  }
    @{ Name = 'libvlcjni';    Url = 'https://gitlab.com/kyber/deps/libvlcjni.git'    }
    @{ Name = 'rust-sdl2';    Url = 'https://gitlab.com/kyber/deps/rust-sdl2.git'    }
    @{ Name = 'txproto';      Url = 'https://gitlab.com/kyber/deps/txproto.git'      }
    @{ Name = 'vigem-client'; Url = 'https://gitlab.com/kyber/deps/vigem-client.git' }
    @{ Name = 'vlc';          Url = 'https://gitlab.com/kyber/deps/vlc.git'          }
    @{ Name = 'vlc-rs';       Url = 'https://gitlab.com/kyber/deps/vlc-rs.git'       }
    @{ Name = 'winit';        Url = 'https://gitlab.com/kyber/deps/winit.git'        }
)

$LARGE_REPOS = @('vlc', 'libvlcjni')

function Write-Log {
    param([string]$Message)
    $ts = Get-Date -Format 'HH:mm:ss'
    $line = "[$ts] $Message"
    Write-Host $line
    Add-Content -Path $LOG_FILE -Value $line
}
function Write-Ok   { param([string]$m); Write-Host "  " -NoNewline; Write-Host "v" -ForegroundColor Green  -NoNewline; Write-Host " $m"; Add-Content $LOG_FILE "  [OK]   $m" }
function Write-Warn { param([string]$m); Write-Host "  " -NoNewline; Write-Host "!" -ForegroundColor Yellow -NoNewline; Write-Host " $m"; Add-Content $LOG_FILE "  [WARN] $m" }
function Write-Err  { param([string]$m); Write-Host "  " -NoNewline; Write-Host "x" -ForegroundColor Red    -NoNewline; Write-Host " $m"; Add-Content $LOG_FILE "  [ERR]  $m" }
function Write-Info { param([string]$m); Write-Host "  " -NoNewline; Write-Host ">" -ForegroundColor Cyan   -NoNewline; Write-Host " $m"; Add-Content $LOG_FILE "  [INFO] $m" }
function Write-Skip { param([string]$m); Write-Host "  -  $m" -ForegroundColor DarkGray; Add-Content $LOG_FILE "  [SKIP] $m" }
function Write-Header {
    param([string]$m)
    Write-Host ""
    Write-Host "== $m ==" -ForegroundColor Blue
    Add-Content $LOG_FILE ""
    Add-Content $LOG_FILE "== $m =="
}
function Write-Sep { Write-Host "   ------------------------------------------" -ForegroundColor DarkGray }
function IsLarge([string]$name) { return $LARGE_REPOS -contains $name }

function Test-Dependencies {
    $missing = @()
    foreach ($cmd in @('git')) {
        if (-not (Get-Command $cmd -ErrorAction SilentlyContinue)) { $missing += $cmd }
    }
    if ($missing.Count -gt 0) {
        Write-Err "Dependances manquantes : $($missing -join ', ')"
        exit 1
    }
}

function Test-Network {
    try {
        $r = Invoke-WebRequest -Uri 'https://gitlab.com' -TimeoutSec 6 -UseBasicParsing -ErrorAction Stop
        return $r.StatusCode -lt 500
    } catch { return $false }
}

function Invoke-Clone {
    param([string]$Name, [string]$Url, [string]$Dest)
    $flags = @('--depth=1')
    if (IsLarge $Name) {
        $flags += '--filter=blob:none'
        Write-Info "$Name : repo volumineux -- clone partiel active"
    }
    Write-Log "Clonage de $Name..."
    & git @(@('clone') + $flags + @($Url, $Dest)) 2>&1 | ForEach-Object { Write-Host $_; Add-Content $LOG_FILE "$_" }
    if ($LASTEXITCODE -ne 0) { Write-Err "Echec du clonage de $Name"; return $false }
    $commit = & git -C $Dest log --oneline -1 2>$null
    if (-not $commit) { $commit = '?' }
    Write-Ok "$Name clone  [$commit]"
    return $true
}

function Update-Repo {
    param([string]$Name, [string]$Url, [string]$Dest)
    if (-not (Test-Path (Join-Path $Dest '.git'))) {
        return Invoke-Clone -Name $Name -Url $Url -Dest $Dest
    }
    $before = & git -C $Dest rev-parse --short HEAD 2>$null
    if (-not $before) { $before = '?' }
    & git -C $Dest fetch origin --depth=1 --quiet 2>&1 | ForEach-Object { Add-Content $LOG_FILE "$_" }
    if ($LASTEXITCODE -ne 0) { Write-Err "$Name : fetch echoue"; return $false }
    $remoteInfo = & git -C $Dest remote show origin 2>$null
    $branch = ($remoteInfo | Select-String 'HEAD branch') -replace '.*HEAD branch:\s*', ''
    $branch = $branch.Trim()
    if (-not $branch) { $branch = 'main' }
    & git -C $Dest reset --hard "origin/$branch" --quiet 2>&1 | ForEach-Object { Add-Content $LOG_FILE "$_" }
    $after = & git -C $Dest rev-parse --short HEAD 2>$null
    if (-not $after) { $after = '?' }
    if ($before -ne $after) {
        $msg = & git -C $Dest log -1 --format="%s" 2>$null
        if ($msg -and $msg.Length -gt 60) { $msg = $msg.Substring(0, 60) }
        Write-Host "  " -NoNewline; Write-Host "v" -ForegroundColor Green -NoNewline
        Write-Host " $Name : " -NoNewline; Write-Host "$before --> $after" -ForegroundColor Yellow -NoNewline
        Write-Host "  $msg"
        Add-Content $LOG_FILE "  [OK] $Name : $before --> $after  $msg"
    } else {
        Write-Skip "$Name : a jour  ($after)"
    }
    $gitmodules = Join-Path $Dest '.gitmodules'
    if ((Test-Path $gitmodules) -and (Select-String -Path $gitmodules -Pattern '\[submodule' -Quiet)) {
        Write-Info "$Name : submodules..."
        & git -C $Dest submodule update --init --recursive --depth=1 --quiet 2>&1 | ForEach-Object { Add-Content $LOG_FILE "$_" }
        if ($LASTEXITCODE -ne 0) { Write-Warn "$Name : submodules partiellement mis a jour" }
    }
    return $true
}

function Update-Group {
    param([string]$Label, [string]$BaseDir, [array]$Repos)
    $failed = 0
    Write-Header $Label
    foreach ($r in $Repos) {
        Write-Host ""
        $ok = Update-Repo -Name $r.Name -Url $r.Url -Dest (Join-Path $BaseDir $r.Name)
        if (-not $ok) { $failed++ }
    }
    return $failed
}

function Show-StatusOne {
    param([string]$Name, [string]$Dest)
    if (-not (Test-Path (Join-Path $Dest '.git'))) {
        Write-Host "  x $Name  " -ForegroundColor Red -NoNewline; Write-Host "NON CLONE" -ForegroundColor Red
        return
    }
    $commit  = & git -C $Dest rev-parse --short HEAD 2>$null;   if (-not $commit)  { $commit  = '?' }
    $branch  = & git -C $Dest branch --show-current 2>$null;    if (-not $branch)  { $branch  = '?' }
    $date    = (& git -C $Dest log -1 --format="%ci" 2>$null) -replace ' .*', ''
    $msg     = & git -C $Dest log -1 --format="%s" 2>$null;     if ($msg -and $msg.Length -gt 58) { $msg = $msg.Substring(0,58) }
    $files   = (Get-ChildItem -Recurse -File -Path $Dest -ErrorAction SilentlyContinue | Where-Object { $_.FullName -notmatch '\\.git\\' } | Measure-Object).Count
    $rsCount = (Get-ChildItem -Recurse -File -Filter '*.rs' -Path $Dest -ErrorAction SilentlyContinue | Where-Object { $_.FullName -notmatch '\\.git\\' } | Measure-Object).Count
    $extra   = if ($rsCount -gt 0) { "  ${rsCount}.rs" } else { "" }
    Write-Host "  v " -ForegroundColor Green -NoNewline
    Write-Host "$Name" -ForegroundColor White -NoNewline
    Write-Host "  [$branch@$commit | $date]$extra  " -NoNewline
    Write-Host "$files files" -ForegroundColor DarkGray
    Write-Host "     | $msg" -ForegroundColor DarkGray
}

function Show-Status {
    Write-Header "Etat du vendor Kyber"
    Write-Host ""; Write-Host "CORE  gitlab.com/kyber/core" -ForegroundColor White; Write-Sep
    foreach ($r in $CORE_REPOS) { Show-StatusOne -Name $r.Name -Dest (Join-Path $VENDOR_DIR $r.Name) }
    Write-Host ""; Write-Host "DEPS  gitlab.com/kyber/deps" -ForegroundColor White; Write-Sep
    foreach ($r in $DEPS_REPOS) { Show-StatusOne -Name $r.Name -Dest (Join-Path $DEPS_DIR $r.Name) }
    Write-Host ""; Write-Sep
    $sum = (Get-ChildItem -Recurse -File $VENDOR_DIR -ErrorAction SilentlyContinue | Measure-Object -Property Length -Sum).Sum
    $total = if ($sum) { [math]::Round($sum / 1MB, 1) } else { 0 }
    Write-Host "  Taille totale vendor/ : " -NoNewline; Write-Host "${total} MB" -ForegroundColor White
    Write-Host ""
    $snapCount = 0
    if (Test-Path $SNAPSHOT_DIR) { $snapCount = (Get-ChildItem -Path $SNAPSHOT_DIR -Filter '*.zip' -ErrorAction SilentlyContinue | Measure-Object).Count }
    if ($snapCount -gt 0) {
        Write-Host "  Snapshots ($snapCount) :" -ForegroundColor Cyan
        Get-ChildItem -Path $SNAPSHOT_DIR -Filter '*.zip' | Sort-Object LastWriteTime -Descending | Select-Object -First 5 | ForEach-Object {
            Write-Host "    $([math]::Round($_.Length/1MB,1)) MB  $($_.Name)"
        }
    } else {
        Write-Host "  Aucun snapshot -- lancez : .\update_vendor.ps1 -Snapshot" -ForegroundColor Yellow
    }
}

function Check-GroupUpdates {
    param([string]$Label, [string]$BaseDir, [array]$Repos)
    $any = $false
    Write-Host $Label -ForegroundColor White; Write-Sep
    foreach ($r in $Repos) {
        $dest = Join-Path $BaseDir $r.Name
        if (-not (Test-Path (Join-Path $dest '.git'))) { Write-Warn "$($r.Name) : non clone"; continue }
        $branch = & git -C $dest branch --show-current 2>$null
        if (-not $branch) { $branch = 'main' }
        & git -C $dest fetch origin --depth=1 --quiet 2>$null
        if ($LASTEXITCODE -ne 0) { Write-Warn "$($r.Name) : impossible de verifier (reseau ?)"; continue }
        $localH  = & git -C $dest rev-parse HEAD 2>$null
        $remoteH = & git -C $dest rev-parse "origin/$branch" 2>$null
        if ($remoteH -and $localH -ne $remoteH) {
            $ls = $localH.Substring(0,[math]::Min(8,$localH.Length))
            $rs = $remoteH.Substring(0,[math]::Min(8,$remoteH.Length))
            Write-Host "  ! " -ForegroundColor Yellow -NoNewline
            Write-Host "$($r.Name) : MAJ disponible  $ls --> $rs" -ForegroundColor Yellow
            $any = $true
        } else {
            Write-Ok "$($r.Name) : a jour  ($($localH.Substring(0,[math]::Min(8,$localH.Length))))"
        }
    }
    return $any
}

function Invoke-CheckUpdates {
    Write-Header "Verification des mises a jour"
    $any = $false
    $any = (Check-GroupUpdates -Label 'CORE' -BaseDir $VENDOR_DIR -Repos $CORE_REPOS) -or $any
    Write-Host ""
    $any = (Check-GroupUpdates -Label 'DEPS' -BaseDir $DEPS_DIR -Repos $DEPS_REPOS) -or $any
    Write-Host ""
    if (-not $any) { Write-Ok "Tous les repos sont a jour." }
    else           { Write-Warn "Des mises a jour sont disponibles."; Write-Host "  Lancez : .\update_vendor.ps1" }
}

function New-Snapshot {
    Write-Header "Creation d'un snapshot de sauvegarde"
    New-Item -ItemType Directory -Path $SNAPSHOT_DIR -Force | Out-Null
    $ts      = Get-Date -Format 'yyyyMMdd_HHmmss'
    $archive = Join-Path $SNAPSHOT_DIR "kyber_vendor_${ts}.zip"
    Write-Log "Archive    : $archive"
    Write-Log "Compression en cours (peut prendre ~1 min pour vlc)..."
    $exclude = @('*\contrib\work\*','*\target\*','*\.snapshots\*','*\.update.log')
    $files   = Get-ChildItem -Path $VENDOR_DIR -Recurse -File -ErrorAction SilentlyContinue | Where-Object {
        $p = $_.FullName
        -not ($exclude | Where-Object { $p -like $_ }) -and $p -notmatch '\\.git\\objects\\pack\\.*\\.pack$'
    }
    Add-Type -AssemblyName System.IO.Compression.FileSystem
    $zip = [System.IO.Compression.ZipFile]::Open($archive, 'Create')
    try {
        foreach ($file in $files) {
            $rel = $file.FullName.Substring($SCRIPT_DIR.Length).TrimStart('\','/')
            [System.IO.Compression.ZipFileExtensions]::CreateEntryFromFile($zip, $file.FullName, $rel) | Out-Null
        }
    } finally { $zip.Dispose() }
    $size = [math]::Round((Get-Item $archive).Length / 1MB, 1)
    Write-Host "  v " -ForegroundColor Green -NoNewline
    Write-Host "Snapshot cree : $(Split-Path $archive -Leaf)  (${size} MB)"
    $snaps = Get-ChildItem -Path $SNAPSHOT_DIR -Filter '*.zip' | Sort-Object LastWriteTime -Descending
    if ($snaps.Count -gt 5) {
        Write-Warn "Rotation : suppression des anciens snapshots (garde 5 max)..."
        $snaps | Select-Object -Skip 5 | Remove-Item -Force
    }
    Write-Host ""; Write-Host "Snapshots disponibles :" -ForegroundColor Cyan
    Get-ChildItem -Path $SNAPSHOT_DIR -Filter '*.zip' | Sort-Object LastWriteTime -Descending | ForEach-Object {
        Write-Host "  $([math]::Round($_.Length/1MB,1)) MB  $($_.Name)"
    }
}

function Show-Help {
    Write-Host ""
    Write-Host "update_vendor.ps1" -ForegroundColor White -NoNewline
    Write-Host " -- Gestion des dependances Kyber pour Syber"
    Write-Host ""
    Write-Host "USAGE" -ForegroundColor White
    Write-Host "  .\update_vendor.ps1              Met a jour core + deps"
    Write-Host "  .\update_vendor.ps1 -Core        Met a jour uniquement les core repos"
    Write-Host "  .\update_vendor.ps1 -Deps        Met a jour uniquement les deps"
    Write-Host "  .\update_vendor.ps1 -Check       Verifie si des MAJ sont disponibles"
    Write-Host "  .\update_vendor.ps1 -Snapshot    Cree une archive zip complete"
    Write-Host "  .\update_vendor.ps1 -Status      Affiche l'etat de chaque repo"
    Write-Host "  .\update_vendor.ps1 -Help        Affiche cette aide"
    Write-Host ""
    Write-Host "CORE REPOS" -ForegroundColor White
    foreach ($r in $CORE_REPOS) { Write-Host "  * $($r.Name)" }
    Write-Host ""
    Write-Host "DEPS REPOS" -ForegroundColor White
    foreach ($r in $DEPS_REPOS) {
        $note = if (IsLarge $r.Name) { "  [volumineux -- clone partiel]" } else { "" }
        Write-Host "  * $($r.Name)$note"
    }
    Write-Host ""
    Write-Host "CHEMINS" -ForegroundColor White
    Write-Host "  Core      : $VENDOR_DIR"
    Write-Host "  Deps      : $DEPS_DIR"
    Write-Host "  Snapshots : $SNAPSHOT_DIR"
    Write-Host "  Log       : $LOG_FILE"
}

# ─── Main ────────────────────────────────────────────────────────────────────

New-Item -ItemType Directory -Path $VENDOR_DIR -Force | Out-Null
New-Item -ItemType Directory -Path $DEPS_DIR   -Force | Out-Null

Add-Content $LOG_FILE ""
Add-Content $LOG_FILE ("=" * 50)
Add-Content $LOG_FILE "$(Get-Date -Format 'yyyy-MM-dd HH:mm:ss')  update_vendor.ps1"
Add-Content $LOG_FILE ("=" * 50)

Test-Dependencies

if ($Help)     { Show-Help;   exit 0 }
if ($Status)   { Show-Status; exit 0 }
if ($Check)    {
    if (-not (Test-Network)) { Write-Err "Reseau indisponible"; exit 1 }
    Invoke-CheckUpdates; exit 0
}
if ($Snapshot) { New-Snapshot; exit 0 }

if (-not (Test-Network)) { Write-Err "Reseau indisponible"; exit 1 }

$totalFailed = 0

if ($Core) { $totalFailed += Update-Group -Label "CORE  (gitlab.com/kyber/core)" -BaseDir $VENDOR_DIR -Repos $CORE_REPOS; exit ($totalFailed -gt 0 ? 1 : 0) }
if ($Deps) { $totalFailed += Update-Group -Label "DEPS  (gitlab.com/kyber/deps)" -BaseDir $DEPS_DIR   -Repos $DEPS_REPOS; exit ($totalFailed -gt 0 ? 1 : 0) }

$totalFailed += Update-Group -Label "CORE  (gitlab.com/kyber/core)" -BaseDir $VENDOR_DIR -Repos $CORE_REPOS
$totalFailed += Update-Group -Label "DEPS  (gitlab.com/kyber/deps)" -BaseDir $DEPS_DIR   -Repos $DEPS_REPOS

Write-Host ""
Show-Status

if ($totalFailed -gt 0) {
    Write-Err "$totalFailed repo(s) ont echoue -- voir $LOG_FILE"
    exit 1
} else {
    Write-Host ""
    Write-Ok "Tous les repos Kyber sont a jour."
    Write-Host "  Conseil : " -NoNewline
    Write-Host "securisez une copie avec .\update_vendor.ps1 -Snapshot" -ForegroundColor Yellow
}