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

# ─── Chemins ─────────────────────────────────────────────────────────────────

$SCRIPT_DIR   = Split-Path -Parent $MyInvocation.MyCommand.Path
$VENDOR_DIR   = Join-Path $SCRIPT_DIR 'vendor'
$DEPS_DIR     = Join-Path $VENDOR_DIR 'deps'
$SNAPSHOT_DIR = Join-Path $VENDOR_DIR '.snapshots'
$LOG_FILE     = Join-Path $VENDOR_DIR '.update.log'

# ─── Repos ───────────────────────────────────────────────────────────────────

$CORE_REPOS = @(
    [ordered]@{ Name = 'kyutil';  Url = 'https://gitlab.com/kyber/core/kyutil.git'  }
    [ordered]@{ Name = 'kymux';   Url = 'https://gitlab.com/kyber/core/kymux.git'   }
    [ordered]@{ Name = 'kymedia'; Url = 'https://gitlab.com/kyber/core/kymedia.git' }
    [ordered]@{ Name = 'kynput';  Url = 'https://gitlab.com/kyber/core/kynput.git'  }
    [ordered]@{ Name = 'kysdk';   Url = 'https://gitlab.com/kyber/core/kysdk.git'   }
)

$DEPS_REPOS = @(
    [ordered]@{ Name = 'keycode';      Url = 'https://gitlab.com/kyber/deps/keycode.git'      }
    [ordered]@{ Name = 'libudev-sys';  Url = 'https://gitlab.com/kyber/deps/libudev-sys.git'  }
    [ordered]@{ Name = 'libvlcjni';    Url = 'https://gitlab.com/kyber/deps/libvlcjni.git'    }
    [ordered]@{ Name = 'rust-sdl2';    Url = 'https://gitlab.com/kyber/deps/rust-sdl2.git'    }
    [ordered]@{ Name = 'txproto';      Url = 'https://gitlab.com/kyber/deps/txproto.git'      }
    [ordered]@{ Name = 'vigem-client'; Url = 'https://gitlab.com/kyber/deps/vigem-client.git' }
    [ordered]@{ Name = 'vlc';          Url = 'https://gitlab.com/kyber/deps/vlc.git'          }
    [ordered]@{ Name = 'vlc-rs';       Url = 'https://gitlab.com/kyber/deps/vlc-rs.git'       }
    [ordered]@{ Name = 'winit';        Url = 'https://gitlab.com/kyber/deps/winit.git'        }
)

# Repos nécessitant un clone partiel (blob:none) en raison de leur taille
$LARGE_REPOS = [System.Collections.Generic.HashSet[string]]@('vlc', 'libvlcjni')

# ─── Logging ─────────────────────────────────────────────────────────────────

function Write-Log {
    param([string]$Message)
    $ts   = Get-Date -Format 'HH:mm:ss'
    $line = "[$ts] $Message"
    Write-Host $line
    Add-Content -LiteralPath $LOG_FILE -Value $line
}

function Write-Ok {
    param([string]$m)
    Write-Host '  ' -NoNewline
    Write-Host 'v' -ForegroundColor Green  -NoNewline
    Write-Host " $m"
    Add-Content -LiteralPath $LOG_FILE -Value "  [OK]   $m"
}

function Write-Warn {
    param([string]$m)
    Write-Host '  ' -NoNewline
    Write-Host '!' -ForegroundColor Yellow -NoNewline
    Write-Host " $m"
    Add-Content -LiteralPath $LOG_FILE -Value "  [WARN] $m"
}

function Write-Err {
    param([string]$m)
    Write-Host '  ' -NoNewline
    Write-Host 'x' -ForegroundColor Red    -NoNewline
    Write-Host " $m"
    Add-Content -LiteralPath $LOG_FILE -Value "  [ERR]  $m"
}

function Write-Info {
    param([string]$m)
    Write-Host '  ' -NoNewline
    Write-Host '>' -ForegroundColor Cyan   -NoNewline
    Write-Host " $m"
    Add-Content -LiteralPath $LOG_FILE -Value "  [INFO] $m"
}

function Write-Skip {
    param([string]$m)
    Write-Host "  -  $m" -ForegroundColor DarkGray
    Add-Content -LiteralPath $LOG_FILE -Value "  [SKIP] $m"
}

function Write-Header {
    param([string]$m)
    Write-Host ''
    Write-Host "== $m ==" -ForegroundColor Blue
    Add-Content -LiteralPath $LOG_FILE -Value ''
    Add-Content -LiteralPath $LOG_FILE -Value "== $m =="
}

function Write-Sep {
    Write-Host '   ------------------------------------------' -ForegroundColor DarkGray
}

# ─── Helpers ─────────────────────────────────────────────────────────────────

function Test-IsLarge {
    param([string]$Name)
    return $LARGE_REPOS.Contains($Name)
}

function Get-DefaultBranch {
    <#
    .SYNOPSIS
        Retourne la branche par défaut du remote origin d'un dépôt git.
        Essaie d'abord via symbolic-ref (rapide, offline), puis via remote show.
    #>
    param([string]$RepoPath)

    # Tentative rapide : lire la ref symbolique du remote (fonctionnel si déjà fetchée)
    $symRef = & git -C $RepoPath rev-parse --abbrev-ref 'origin/HEAD' 2>$null
    if ($LASTEXITCODE -eq 0 -and $symRef -match '^origin/(.+)$') {
        return $Matches[1].Trim()
    }

    # Fallback : interroger le remote (nécessite réseau)
    $remoteLines = & git -C $RepoPath remote show origin 2>$null
    if ($LASTEXITCODE -eq 0) {
        $match = $remoteLines | Select-String 'HEAD branch:\s*(.+)$'
        if ($match) {
            return $match.Matches[0].Groups[1].Value.Trim()
        }
    }

    # Défaut conservatif
    return 'main'
}

function Get-ShortHash {
    param([string]$RepoPath, [int]$Length = 8)
    $hash = & git -C $RepoPath rev-parse HEAD 2>$null
    if ($LASTEXITCODE -ne 0 -or -not $hash) { return '?' }
    return $hash.Substring(0, [math]::Min($Length, $hash.Length))
}

function Test-Dependencies {
    $missing = @()
    foreach ($cmd in @('git')) {
        if (-not (Get-Command $cmd -ErrorAction SilentlyContinue)) {
            $missing += $cmd
        }
    }
    if ($missing.Count -gt 0) {
        Write-Err "Dependances manquantes : $($missing -join ', ')"
        exit 1
    }
}

function Test-Network {
    try {
        $response = Invoke-WebRequest `
            -Uri            'https://gitlab.com' `
            -TimeoutSec     8 `
            -UseBasicParsing `
            -ErrorAction    Stop
        return $response.StatusCode -lt 500
    }
    catch {
        return $false
    }
}

# ─── Git operations ──────────────────────────────────────────────────────────

function Invoke-GitClone {
    <#
    .SYNOPSIS
        Clone un dépôt git avec shallow clone.
        Retourne $true en cas de succès, $false sinon.
    .OUTPUTS
        [bool]
    #>
    param(
        [string]$Name,
        [string]$Url,
        [string]$Dest
    )

    $flags = [System.Collections.Generic.List[string]]@('--depth=1')

    if (Test-IsLarge $Name) {
        $flags.Add('--filter=blob:none')
        Write-Info "$Name : repo volumineux — clone partiel activé"
    }

    Write-Log "Clonage de $Name..."

    $gitArgs = @('clone') + $flags + @($Url, $Dest)
    & git @gitArgs 2>&1 | ForEach-Object {
        Write-Host $_
        Add-Content -LiteralPath $LOG_FILE -Value "$_"
    }

    if ($LASTEXITCODE -ne 0) {
        Write-Err "Echec du clonage de $Name (code $LASTEXITCODE)"
        return $false
    }

    $commit = & git -C $Dest log --oneline -1 2>$null
    if (-not $commit) { $commit = '?' }
    Write-Ok "$Name cloné  [$commit]"
    return $true
}

function Update-Repo {
    <#
    .SYNOPSIS
        Met à jour un dépôt git (clone si absent, fetch+reset sinon).
        Retourne $true en cas de succès, $false sinon.
    .OUTPUTS
        [bool]
    #>
    param(
        [string]$Name,
        [string]$Url,
        [string]$Dest
    )

    # ── Clone initial si le répertoire .git est absent ────────────────────────
    if (-not (Test-Path -LiteralPath (Join-Path $Dest '.git'))) {
        $cloneResult = Invoke-GitClone -Name $Name -Url $Url -Dest $Dest
        return $cloneResult
    }

    # ── Fetch ─────────────────────────────────────────────────────────────────
    $before = Get-ShortHash -RepoPath $Dest

    $fetchOutput = & git -C $Dest fetch origin --depth=1 2>&1
    $fetchOutput | ForEach-Object { Add-Content -LiteralPath $LOG_FILE -Value "$_" }

    if ($LASTEXITCODE -ne 0) {
        Write-Err "$Name : fetch échoué (code $LASTEXITCODE)"
        return $false
    }

    # ── Branche par défaut ────────────────────────────────────────────────────
    $branch = Get-DefaultBranch -RepoPath $Dest

    # ── Reset vers le remote ──────────────────────────────────────────────────
    # Note : git reset --hard n'accepte pas --quiet ; on supprime stderr/stdout
    $resetOutput = & git -C $Dest reset --hard "origin/$branch" 2>&1
    $resetOutput | ForEach-Object { Add-Content -LiteralPath $LOG_FILE -Value "$_" }

    if ($LASTEXITCODE -ne 0) {
        Write-Err "$Name : reset vers origin/$branch échoué"
        return $false
    }

    # ── Comparaison avant/après ───────────────────────────────────────────────
    $after = Get-ShortHash -RepoPath $Dest

    if ($before -ne $after -and $before -ne '?' -and $after -ne '?') {
        $rawMsg = & git -C $Dest log -1 --format='%s' 2>$null
        $msg    = if ($rawMsg -and $rawMsg.Length -gt 60) { $rawMsg.Substring(0, 60) + '…' } else { $rawMsg }
        Write-Host '  ' -NoNewline
        Write-Host 'v' -ForegroundColor Green  -NoNewline
        Write-Host " $Name : " -NoNewline
        Write-Host "$before → $after" -ForegroundColor Yellow -NoNewline
        Write-Host "  $msg"
        Add-Content -LiteralPath $LOG_FILE -Value "  [OK] $Name : $before → $after  $msg"
    }
    else {
        Write-Skip "$Name : à jour  ($after)"
    }

    # ── Submodules ────────────────────────────────────────────────────────────
    $gitmodules = Join-Path $Dest '.gitmodules'
    if ((Test-Path -LiteralPath $gitmodules) -and
        (Select-String -LiteralPath $gitmodules -Pattern '\[submodule' -Quiet)) {
        Write-Info "$Name : mise à jour des submodules..."
        $subOutput = & git -C $Dest submodule update --init --recursive --depth=1 2>&1
        $subOutput | ForEach-Object { Add-Content -LiteralPath $LOG_FILE -Value "$_" }
        if ($LASTEXITCODE -ne 0) {
            Write-Warn "$Name : submodules partiellement mis à jour"
        }
    }

    return $true
}

function Update-Group {
    <#
    .SYNOPSIS
        Met à jour un groupe de repos et retourne le nombre d'échecs.
    .OUTPUTS
        [int]
    #>
    param(
        [string]$Label,
        [string]$BaseDir,
        [array]$Repos
    )

    Write-Header $Label

    # Compteur isolé — évite la pollution du pipeline PowerShell
    [int]$failed = 0

    foreach ($r in $Repos) {
        Write-Host ''
        # Capturer explicitement le booléen via une variable intermédiaire
        [bool]$ok = Update-Repo -Name $r.Name -Url $r.Url -Dest (Join-Path $BaseDir $r.Name)
        if (-not $ok) { $failed++ }
    }

    return $failed
}

# ─── Status ──────────────────────────────────────────────────────────────────

function Show-StatusOne {
    param(
        [string]$Name,
        [string]$Dest
    )

    if (-not (Test-Path -LiteralPath (Join-Path $Dest '.git'))) {
        Write-Host "  x $Name  " -ForegroundColor Red -NoNewline
        Write-Host 'NON CLONÉ'   -ForegroundColor Red
        return
    }

    $commit = & git -C $Dest rev-parse --short HEAD 2>$null
    if (-not $commit) { $commit = '?' }

    $branch = & git -C $Dest branch --show-current 2>$null
    if (-not $branch) { $branch = '?' }

    $rawDate = & git -C $Dest log -1 --format='%ci' 2>$null
    $date    = if ($rawDate) { ($rawDate -split ' ')[0] } else { '?' }

    $rawMsg = & git -C $Dest log -1 --format='%s' 2>$null
    $msg    = if ($rawMsg -and $rawMsg.Length -gt 58) { $rawMsg.Substring(0, 58) + '…' } else { $rawMsg }

    # Comptage des fichiers hors .git (compatible Windows et UNC)
    $allFiles = Get-ChildItem -Recurse -File -Path $Dest -ErrorAction SilentlyContinue |
                Where-Object { $_.FullName -notmatch [regex]::Escape([IO.Path]::DirectorySeparatorChar + '.git' + [IO.Path]::DirectorySeparatorChar) }
    $fileCount = ($allFiles | Measure-Object).Count
    $rsCount   = ($allFiles | Where-Object { $_.Extension -eq '.rs' } | Measure-Object).Count
    $extra     = if ($rsCount -gt 0) { "  ${rsCount} .rs" } else { '' }

    Write-Host '  v ' -ForegroundColor Green -NoNewline
    Write-Host $Name  -ForegroundColor White  -NoNewline
    Write-Host "  [$branch@$commit | $date]$extra  " -NoNewline
    Write-Host "$fileCount files" -ForegroundColor DarkGray
    if ($msg) {
        Write-Host "     | $msg" -ForegroundColor DarkGray
    }
}

function Show-Status {
    Write-Header 'Etat du vendor Kyber'

    Write-Host ''
    Write-Host 'CORE  gitlab.com/kyber/core' -ForegroundColor White
    Write-Sep
    foreach ($r in $CORE_REPOS) {
        Show-StatusOne -Name $r.Name -Dest (Join-Path $VENDOR_DIR $r.Name)
    }

    Write-Host ''
    Write-Host 'DEPS  gitlab.com/kyber/deps' -ForegroundColor White
    Write-Sep
    foreach ($r in $DEPS_REPOS) {
        Show-StatusOne -Name $r.Name -Dest (Join-Path $DEPS_DIR $r.Name)
    }

    Write-Host ''
    Write-Sep

    # Taille totale
    $sumBytes = (Get-ChildItem -Recurse -File -Path $VENDOR_DIR -ErrorAction SilentlyContinue |
                 Measure-Object -Property Length -Sum).Sum
    $totalMB  = if ($sumBytes) { [math]::Round($sumBytes / 1MB, 1) } else { 0 }
    Write-Host '  Taille totale vendor/ : ' -NoNewline
    Write-Host "${totalMB} MB" -ForegroundColor White

    # Snapshots
    Write-Host ''
    [int]$snapCount = 0
    if (Test-Path -LiteralPath $SNAPSHOT_DIR) {
        $snapCount = (Get-ChildItem -Path $SNAPSHOT_DIR -Filter '*.zip' -ErrorAction SilentlyContinue |
                      Measure-Object).Count
    }

    if ($snapCount -gt 0) {
        Write-Host "  Snapshots ($snapCount) :" -ForegroundColor Cyan
        Get-ChildItem -Path $SNAPSHOT_DIR -Filter '*.zip' |
            Sort-Object LastWriteTime -Descending |
            Select-Object -First 5 |
            ForEach-Object {
                Write-Host "    $([math]::Round($_.Length / 1MB, 1)) MB  $($_.Name)"
            }
    }
    else {
        Write-Host '  Aucun snapshot — lancez : .\update_vendor.ps1 -Snapshot' -ForegroundColor Yellow
    }
}

# ─── Check ───────────────────────────────────────────────────────────────────

function Test-GroupUpdates {
    <#
    .SYNOPSIS
        Vérifie si des MAJ sont disponibles pour un groupe de repos.
        Retourne $true si au moins une MAJ est disponible.
    .OUTPUTS
        [bool]
    #>
    param(
        [string]$Label,
        [string]$BaseDir,
        [array]$Repos
    )

    Write-Host $Label -ForegroundColor White
    Write-Sep

    [bool]$anyUpdate = $false

    foreach ($r in $Repos) {
        $dest = Join-Path $BaseDir $r.Name

        if (-not (Test-Path -LiteralPath (Join-Path $dest '.git'))) {
            Write-Warn "$($r.Name) : non cloné"
            continue
        }

        # Fetch silencieux
        & git -C $dest fetch origin --depth=1 --quiet 2>$null
        if ($LASTEXITCODE -ne 0) {
            Write-Warn "$($r.Name) : impossible de vérifier (réseau ?)"
            continue
        }

        $branch  = Get-DefaultBranch -RepoPath $dest
        $localH  = & git -C $dest rev-parse HEAD 2>$null
        $remoteH = & git -C $dest rev-parse "origin/$branch" 2>$null

        if (-not $localH -or -not $remoteH) {
            Write-Warn "$($r.Name) : impossible de résoudre les refs"
            continue
        }

        if ($localH -ne $remoteH) {
            $ls = $localH.Substring(0,  [math]::Min(8, $localH.Length))
            $rs = $remoteH.Substring(0, [math]::Min(8, $remoteH.Length))
            Write-Host '  ! ' -ForegroundColor Yellow -NoNewline
            Write-Host "$($r.Name) : MAJ disponible  $ls → $rs" -ForegroundColor Yellow
            $anyUpdate = $true
        }
        else {
            $short = $localH.Substring(0, [math]::Min(8, $localH.Length))
            Write-Ok "$($r.Name) : à jour  ($short)"
        }
    }

    return $anyUpdate
}

function Invoke-CheckUpdates {
    Write-Header 'Vérification des mises à jour'

    # IMPORTANT : capturer les booléens avant le -or pour éviter que PowerShell
    # n'avale des sorties parasites dans le pipeline.
    [bool]$coreAny = Test-GroupUpdates -Label 'CORE' -BaseDir $VENDOR_DIR -Repos $CORE_REPOS
    Write-Host ''
    [bool]$depsAny = Test-GroupUpdates -Label 'DEPS' -BaseDir $DEPS_DIR   -Repos $DEPS_REPOS
    Write-Host ''

    if (-not ($coreAny -or $depsAny)) {
        Write-Ok 'Tous les repos sont à jour.'
    }
    else {
        Write-Warn 'Des mises à jour sont disponibles.'
        Write-Host '  Lancez : .\update_vendor.ps1'
    }
}

# ─── Snapshot ────────────────────────────────────────────────────────────────

function New-Snapshot {
    Write-Header 'Création d''un snapshot de sauvegarde'

    New-Item -ItemType Directory -Path $SNAPSHOT_DIR -Force | Out-Null

    $ts      = Get-Date -Format 'yyyyMMdd_HHmmss'
    $archive = Join-Path $SNAPSHOT_DIR "kyber_vendor_${ts}.zip"

    Write-Log "Archive    : $archive"
    Write-Log 'Compression en cours (peut prendre ~1 min pour vlc)...'

    # Patterns d'exclusion (chemins à ignorer dans l'archive)
    $excludePatterns = @(
        '*\contrib\work\*'
        '*\target\*'
        '*\.snapshots\*'
        '*\.update.log'
        '*.pack'          # pack git volumineux
    )

    $files = Get-ChildItem -Path $VENDOR_DIR -Recurse -File -ErrorAction SilentlyContinue |
             Where-Object {
                 $p = $_.FullName
                 # Exclure les objets pack git et les chemins listés
                 ($p -notmatch [regex]::Escape('.git\objects\pack')) -and
                 -not ($excludePatterns | Where-Object { $p -like $_ })
             }

    Add-Type -AssemblyName System.IO.Compression.FileSystem

    try {
        $zip = [System.IO.Compression.ZipFile]::Open($archive, 'Create')
        try {
            foreach ($file in $files) {
                $rel = $file.FullName.Substring($SCRIPT_DIR.Length).TrimStart('\', '/')
                [System.IO.Compression.ZipFileExtensions]::CreateEntryFromFile(
                    $zip, $file.FullName, $rel
                ) | Out-Null
            }
        }
        finally {
            $zip.Dispose()
        }
    }
    catch {
        Write-Err "Echec de la création du snapshot : $_"
        # Nettoyer l'archive corrompue si elle existe
        if (Test-Path -LiteralPath $archive) { Remove-Item -LiteralPath $archive -Force }
        exit 1
    }

    $sizeMB = [math]::Round((Get-Item -LiteralPath $archive).Length / 1MB, 1)
    Write-Host '  v ' -ForegroundColor Green -NoNewline
    Write-Host "Snapshot créé : $(Split-Path $archive -Leaf)  (${sizeMB} MB)"

    # Rotation : garder les 5 snapshots les plus récents
    $allSnaps = Get-ChildItem -Path $SNAPSHOT_DIR -Filter '*.zip' |
                Sort-Object LastWriteTime -Descending
    if ($allSnaps.Count -gt 5) {
        Write-Warn 'Rotation : suppression des anciens snapshots (garde 5 max)...'
        $allSnaps | Select-Object -Skip 5 | Remove-Item -Force
    }

    Write-Host ''
    Write-Host 'Snapshots disponibles :' -ForegroundColor Cyan
    Get-ChildItem -Path $SNAPSHOT_DIR -Filter '*.zip' |
        Sort-Object LastWriteTime -Descending |
        ForEach-Object {
            Write-Host "  $([math]::Round($_.Length / 1MB, 1)) MB  $($_.Name)"
        }
}

# ─── Help ────────────────────────────────────────────────────────────────────

function Show-Help {
    Write-Host ''
    Write-Host 'update_vendor.ps1' -ForegroundColor White -NoNewline
    Write-Host ' — Gestion des dépendances Kyber pour Syber'
    Write-Host ''
    Write-Host 'USAGE' -ForegroundColor White
    Write-Host '  .\update_vendor.ps1              Met à jour core + deps'
    Write-Host '  .\update_vendor.ps1 -Core        Met à jour uniquement les core repos'
    Write-Host '  .\update_vendor.ps1 -Deps        Met à jour uniquement les deps'
    Write-Host '  .\update_vendor.ps1 -Check       Vérifie si des MAJ sont disponibles'
    Write-Host '  .\update_vendor.ps1 -Snapshot    Crée une archive zip complète'
    Write-Host '  .\update_vendor.ps1 -Status      Affiche l''état de chaque repo'
    Write-Host '  .\update_vendor.ps1 -Help        Affiche cette aide'
    Write-Host ''
    Write-Host 'CORE REPOS' -ForegroundColor White
    foreach ($r in $CORE_REPOS) { Write-Host "  * $($r.Name)" }
    Write-Host ''
    Write-Host 'DEPS REPOS' -ForegroundColor White
    foreach ($r in $DEPS_REPOS) {
        $note = if (Test-IsLarge $r.Name) { '  [volumineux — clone partiel]' } else { '' }
        Write-Host "  * $($r.Name)$note"
    }
    Write-Host ''
    Write-Host 'CHEMINS' -ForegroundColor White
    Write-Host "  Core      : $VENDOR_DIR"
    Write-Host "  Deps      : $DEPS_DIR"
    Write-Host "  Snapshots : $SNAPSHOT_DIR"
    Write-Host "  Log       : $LOG_FILE"
    Write-Host ''
}

# ─── Main ────────────────────────────────────────────────────────────────────

# Créer les répertoires nécessaires
New-Item -ItemType Directory -Path $VENDOR_DIR -Force | Out-Null
New-Item -ItemType Directory -Path $DEPS_DIR   -Force | Out-Null

# Entête de session dans le log
Add-Content -LiteralPath $LOG_FILE -Value ''
Add-Content -LiteralPath $LOG_FILE -Value ('=' * 60)
Add-Content -LiteralPath $LOG_FILE -Value "$(Get-Date -Format 'yyyy-MM-dd HH:mm:ss')  update_vendor.ps1"
Add-Content -LiteralPath $LOG_FILE -Value ('=' * 60)

Test-Dependencies

# ── Commandes sans réseau ─────────────────────────────────────────────────────
if ($Help)   { Show-Help;   exit 0 }
if ($Status) { Show-Status; exit 0 }

# ── Commandes nécessitant le réseau ───────────────────────────────────────────
if ($Check) {
    if (-not (Test-Network)) { Write-Err 'Réseau indisponible'; exit 1 }
    Invoke-CheckUpdates
    exit 0
}

if ($Snapshot) {
    New-Snapshot
    exit 0
}

# ── Mise à jour ───────────────────────────────────────────────────────────────
if (-not (Test-Network)) { Write-Err 'Réseau indisponible'; exit 1 }

$stopwatch = [System.Diagnostics.Stopwatch]::StartNew()

[int]$totalFailed = 0

if ($Core) {
    $totalFailed += Update-Group `
        -Label   'CORE  (gitlab.com/kyber/core)' `
        -BaseDir $VENDOR_DIR `
        -Repos   $CORE_REPOS
}
elseif ($Deps) {
    $totalFailed += Update-Group `
        -Label   'DEPS  (gitlab.com/kyber/deps)' `
        -BaseDir $DEPS_DIR `
        -Repos   $DEPS_REPOS
}
else {
    # Tout mettre à jour
    $totalFailed += Update-Group `
        -Label   'CORE  (gitlab.com/kyber/core)' `
        -BaseDir $VENDOR_DIR `
        -Repos   $CORE_REPOS
    $totalFailed += Update-Group `
        -Label   'DEPS  (gitlab.com/kyber/deps)' `
        -BaseDir $DEPS_DIR `
        -Repos   $DEPS_REPOS
}

$stopwatch.Stop()
$elapsed = '{0:mm\:ss}' -f $stopwatch.Elapsed

Write-Host ''
Show-Status

Write-Host ''
if ($totalFailed -gt 0) {
    Write-Err "$totalFailed repo(s) ont échoué — voir $LOG_FILE"
    Write-Log "Terminé en $elapsed — $totalFailed échec(s)"
    exit 1
}
else {
    Write-Ok "Tous les repos Kyber sont à jour.  (${elapsed})"
    Write-Host '  Conseil : ' -NoNewline
    Write-Host 'sécurisez une copie avec .\update_vendor.ps1 -Snapshot' -ForegroundColor Yellow
    Write-Log "Terminé en $elapsed — OK"
    exit 0
}