#!/usr/bin/env bash
# =============================================================================
# update_vendor.sh — Mise à jour / sauvegarde des dépôts Kyber (core + deps)
# =============================================================================
#
# Usage:
#   ./update_vendor.sh              Met à jour tous les repos
#   ./update_vendor.sh --core       Met à jour uniquement les core repos
#   ./update_vendor.sh --deps       Met à jour uniquement les deps
#   ./update_vendor.sh --check      Vérifie si des mises à jour sont disponibles
#   ./update_vendor.sh --snapshot   Crée une archive tar.gz de sauvegarde
#   ./update_vendor.sh --status     Affiche l'état de chaque repo
#   ./update_vendor.sh --help       Affiche l'aide
#
# Snapshot : sauvegarde complète de tous les sources Kyber, en cas de
# passage en closed source ou suppression des dépôts GitLab.
# =============================================================================

set -eo pipefail

# ─── Chemins ────────────────────────────────────────────────────────────────

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
VENDOR_DIR="$SCRIPT_DIR/vendor"
DEPS_DIR="$VENDOR_DIR/deps"
SNAPSHOT_DIR="$VENDOR_DIR/.snapshots"
LOG_FILE="$VENDOR_DIR/.update.log"

# ─── Couleurs ────────────────────────────────────────────────────────────────

RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
CYAN='\033[0;36m'
BOLD='\033[1m'
DIM='\033[2m'
NC='\033[0m'

# ─── Repos Kyber CORE ────────────────────────────────────────────────────────
# Format : "nom|url"

CORE_REPOS=(
    "kyutil|https://gitlab.com/kyber/core/kyutil.git"
    "kymux|https://gitlab.com/kyber/core/kymux.git"
    "kymedia|https://gitlab.com/kyber/core/kymedia.git"
    "kynput|https://gitlab.com/kyber/core/kynput.git"
    "kysdk|https://gitlab.com/kyber/core/kysdk.git"
)

# ─── Repos Kyber DEPS ────────────────────────────────────────────────────────
# Forks de bibliothèques tierces avec patches spécifiques Kyber

DEPS_REPOS=(
    "keycode|https://gitlab.com/kyber/deps/keycode.git"
    "libudev-sys|https://gitlab.com/kyber/deps/libudev-sys.git"
    "libvlcjni|https://gitlab.com/kyber/deps/libvlcjni.git"
    "rust-sdl2|https://gitlab.com/kyber/deps/rust-sdl2.git"
    "txproto|https://gitlab.com/kyber/deps/txproto.git"
    "vigem-client|https://gitlab.com/kyber/deps/vigem-client.git"
    "vlc|https://gitlab.com/kyber/deps/vlc.git"
    "vlc-rs|https://gitlab.com/kyber/deps/vlc-rs.git"
    "winit|https://gitlab.com/kyber/deps/winit.git"
)

# Repos volumineux → clonage partiel (--filter=blob:none)
LARGE_REPOS="vlc libvlcjni"

# ─── Helpers ─────────────────────────────────────────────────────────────────

log()    { echo -e "${BOLD}[$(date '+%H:%M:%S')]${NC} $*" | tee -a "$LOG_FILE"; }
ok()     { echo -e "  ${GREEN}✓${NC} $*" | tee -a "$LOG_FILE"; }
warn()   { echo -e "  ${YELLOW}⚠${NC} $*" | tee -a "$LOG_FILE"; }
err()    { echo -e "  ${RED}✗${NC} $*" | tee -a "$LOG_FILE"; }
info()   { echo -e "  ${CYAN}→${NC} $*" | tee -a "$LOG_FILE"; }
skip()   { echo -e "  ${DIM}–  $*${NC}" | tee -a "$LOG_FILE"; }
header() { echo -e "\n${BOLD}${BLUE}══ $* ══${NC}" | tee -a "$LOG_FILE"; }
sep()    { echo -e "${DIM}   ──────────────────────────────────────────${NC}"; }

is_large() {
    local name="$1"
    echo "$LARGE_REPOS" | grep -qw "$name"
}

check_deps() {
    local missing=""
    for cmd in git curl tar; do
        command -v "$cmd" >/dev/null 2>&1 || missing="$missing $cmd"
    done
    if [ -n "$missing" ]; then
        err "Dépendances manquantes :$missing"
        exit 1
    fi
}

check_network() {
    curl -s --max-time 6 https://gitlab.com >/dev/null 2>&1
}

# ─── Clone ───────────────────────────────────────────────────────────────────

clone_repo() {
    local name="$1"
    local url="$2"
    local dest="$3"

    local flags="--depth=1"
    if is_large "$name"; then
        flags="--depth=1 --filter=blob:none"
        info "$name : repo volumineux — clone partiel activé"
    fi

    log "Clonage de $name..."
    # shellcheck disable=SC2086
    if git clone $flags "$url" "$dest" 2>&1 | tee -a "$LOG_FILE"; then
        local commit
        commit=$(git -C "$dest" log --oneline -1 2>/dev/null || echo "?")
        ok "$name cloné  [$commit]"
        return 0
    else
        err "Échec du clonage de $name"
        return 1
    fi
}

# ─── Update ──────────────────────────────────────────────────────────────────

update_repo() {
    local name="$1"
    local url="$2"
    local dest="$3"

    # Clone si absent
    if [ ! -d "$dest/.git" ]; then
        clone_repo "$name" "$url" "$dest"
        return $?
    fi

    local before
    before=$(git -C "$dest" rev-parse --short HEAD 2>/dev/null || echo "?")

    # Fetch silencieux
    if ! git -C "$dest" fetch origin --depth=1 --quiet 2>&1 | tee -a "$LOG_FILE"; then
        err "$name : fetch échoué"
        return 1
    fi

    # Branche par défaut
    local branch
    branch=$(git -C "$dest" remote show origin 2>/dev/null | \
             grep 'HEAD branch' | awk '{print $NF}')
    branch="${branch:-main}"

    # Reset hard sur origin
    git -C "$dest" reset --hard "origin/$branch" --quiet 2>&1 | tee -a "$LOG_FILE" || true

    local after
    after=$(git -C "$dest" rev-parse --short HEAD 2>/dev/null || echo "?")

    if [ "$before" != "$after" ]; then
        local msg
        msg=$(git -C "$dest" log -1 --format="%s" 2>/dev/null | cut -c1-60)
        ok "$name : ${YELLOW}$before → $after${NC}  $msg"
    else
        skip "$name : à jour  ($after)"
    fi

    # Submodules
    if [ -f "$dest/.gitmodules" ] && grep -q '\[submodule' "$dest/.gitmodules" 2>/dev/null; then
        info "$name : submodules..."
        git -C "$dest" submodule update --init --recursive --depth=1 --quiet 2>&1 \
            | tee -a "$LOG_FILE" || warn "$name : submodules partiellement mis à jour"
    fi

    return 0
}

update_group() {
    local label="$1"
    local base_dir="$2"
    shift 2
    local entries=("$@")
    local failed=0

    header "$label"

    for entry in "${entries[@]}"; do
        local name url
        name="${entry%%|*}"
        url="${entry##*|}"
        echo ""
        update_repo "$name" "$url" "$base_dir/$name" || failed=$((failed + 1))
    done

    return $failed
}

# ─── Status ──────────────────────────────────────────────────────────────────

status_one() {
    local name="$1"
    local dest="$2"

    if [ ! -d "$dest/.git" ]; then
        echo -e "  ${RED}✗${NC} ${BOLD}$name${NC}  ${RED}NON CLONÉ${NC}"
        return
    fi

    local commit branch date msg files rs_count
    commit=$(git -C "$dest" rev-parse --short HEAD 2>/dev/null || echo "?")
    branch=$(git -C "$dest" branch --show-current 2>/dev/null || echo "?")
    date=$(git -C "$dest" log -1 --format="%ci" 2>/dev/null | cut -c1-10 || echo "?")
    msg=$(git -C "$dest" log -1 --format="%s" 2>/dev/null | cut -c1-58 || echo "?")
    files=$(find "$dest" -not -path '*/.git/*' -type f 2>/dev/null | wc -l | tr -d ' ')
    rs_count=$(find "$dest" -name "*.rs" -not -path '*/.git/*' 2>/dev/null | wc -l | tr -d ' ')

    local extra=""
    [ "$rs_count" -gt 0 ] && extra=" ${DIM}${rs_count}.rs${NC}"

    echo -e "  ${GREEN}✓${NC} ${BOLD}$name${NC}  [$branch@$commit | $date]${extra}  ${DIM}$files files${NC}"
    echo -e "     ${DIM}└ $msg${NC}"
}

print_status() {
    header "État du vendor Kyber"

    echo -e "\n${BOLD}CORE${NC}  ${DIM}gitlab.com/kyber/core${NC}"
    sep
    for entry in "${CORE_REPOS[@]}"; do
        local name="${entry%%|*}"
        status_one "$name" "$VENDOR_DIR/$name"
    done

    echo -e "\n${BOLD}DEPS${NC}  ${DIM}gitlab.com/kyber/deps${NC}"
    sep
    for entry in "${DEPS_REPOS[@]}"; do
        local name="${entry%%|*}"
        status_one "$name" "$DEPS_DIR/$name"
    done

    echo ""
    sep
    local total
    total=$(du -sh "$VENDOR_DIR" 2>/dev/null | cut -f1)
    echo -e "  Taille totale vendor/ : ${BOLD}$total${NC}"

    echo ""
    local snap_count=0
    if ls "$SNAPSHOT_DIR"/*.tar.gz >/dev/null 2>&1; then
        snap_count=$(ls -1 "$SNAPSHOT_DIR"/*.tar.gz | wc -l | tr -d ' ')
    fi
    if [ "$snap_count" -gt 0 ]; then
        echo -e "  ${CYAN}Snapshots ($snap_count) :${NC}"
        ls -lht "$SNAPSHOT_DIR"/*.tar.gz 2>/dev/null | head -5 | \
            awk '{printf "    %s  %s\n", $5, $9}'
    else
        echo -e "  ${YELLOW}Aucun snapshot — lancez : ./update_vendor.sh --snapshot${NC}"
    fi
}

# ─── Check updates ───────────────────────────────────────────────────────────

check_group_updates() {
    local label="$1"
    local base_dir="$2"
    shift 2
    local entries=("$@")
    local any=0

    echo -e "${BOLD}$label${NC}"
    sep

    for entry in "${entries[@]}"; do
        local name="${entry%%|*}"
        local dest="$base_dir/$name"

        if [ ! -d "$dest/.git" ]; then
            warn "$name : non cloné"
            continue
        fi

        local branch
        branch=$(git -C "$dest" branch --show-current 2>/dev/null || echo "main")

        git -C "$dest" fetch origin --depth=1 --quiet 2>/dev/null || {
            warn "$name : impossible de vérifier (réseau ?)"
            continue
        }

        local local_h remote_h
        local_h=$(git -C "$dest" rev-parse HEAD)
        remote_h=$(git -C "$dest" rev-parse "origin/$branch" 2>/dev/null || echo "")

        if [ -n "$remote_h" ] && [ "$local_h" != "$remote_h" ]; then
            warn "$name : ${YELLOW}MAJ disponible${NC}  ${local_h:0:8} → ${remote_h:0:8}"
            any=1
        else
            ok "$name : à jour  (${local_h:0:8})"
        fi
    done

    return $any
}

check_updates() {
    header "Vérification des mises à jour"
    local any=0

    check_group_updates "CORE" "$VENDOR_DIR" "${CORE_REPOS[@]}" || any=1
    echo ""
    check_group_updates "DEPS" "$DEPS_DIR" "${DEPS_REPOS[@]}" || any=1

    echo ""
    if [ $any -eq 0 ]; then
        ok "Tous les repos sont à jour."
    else
        warn "Des mises à jour sont disponibles."
        echo -e "  Lancez : ${BOLD}./update_vendor.sh${NC}"
    fi
}

# ─── Snapshot ────────────────────────────────────────────────────────────────

create_snapshot() {
    header "Création d'un snapshot de sauvegarde"
    mkdir -p "$SNAPSHOT_DIR"

    local ts
    ts=$(date '+%Y%m%d_%H%M%S')
    local archive="$SNAPSHOT_DIR/kyber_vendor_${ts}.tar.gz"

    log "Archive    : $archive"
    log "Contenu    : vendor/  (core + deps, sans artefacts de build)"
    log "Compression en cours (peut prendre ~1 min pour vlc)..."

    tar -czf "$archive" \
        --exclude="*/contrib/work" \
        --exclude="*/target" \
        --exclude="*/.snapshots" \
        --exclude="*/.update.log" \
        --exclude="*/.git/objects/pack/*.pack" \
        -C "$SCRIPT_DIR" \
        "vendor" 2>/dev/null || \
    tar -czf "$archive" \
        --exclude="*/contrib/work" \
        --exclude="*/target" \
        --exclude="*/.snapshots" \
        --exclude="*/.update.log" \
        -C "$SCRIPT_DIR" \
        "vendor"

    local size
    size=$(du -sh "$archive" 2>/dev/null | cut -f1)
    ok "Snapshot créé : ${BOLD}$(basename "$archive")${NC}  (${size})"

    # Rotation : garde les 5 derniers
    local count=0
    if ls "$SNAPSHOT_DIR"/*.tar.gz >/dev/null 2>&1; then
        count=$(ls -1 "$SNAPSHOT_DIR"/*.tar.gz | wc -l | tr -d ' ')
    fi
    if [ "$count" -gt 5 ]; then
        warn "Rotation : suppression des anciens snapshots (garde 5 max)..."
        ls -1t "$SNAPSHOT_DIR"/*.tar.gz | tail -n +6 | xargs rm -f
    fi

    echo ""
    echo -e "${CYAN}Snapshots disponibles :${NC}"
    ls -lht "$SNAPSHOT_DIR"/*.tar.gz 2>/dev/null | \
        awk '{printf "  %s  %s\n", $5, $9}' || true
}

# ─── Help ─────────────────────────────────────────────────────────────────────

print_help() {
    echo -e "${BOLD}update_vendor.sh${NC} — Gestion des dépendances Kyber pour Syber"
    echo ""
    echo -e "${BOLD}USAGE${NC}"
    echo "  ./update_vendor.sh              Met à jour core + deps (fetch + reset)"
    echo "  ./update_vendor.sh --core       Met à jour uniquement les core repos"
    echo "  ./update_vendor.sh --deps       Met à jour uniquement les deps"
    echo "  ./update_vendor.sh --check      Vérifie si des MAJ sont disponibles"
    echo "  ./update_vendor.sh --snapshot   Crée une archive tar.gz complète"
    echo "  ./update_vendor.sh --status     Affiche l'état de chaque repo"
    echo "  ./update_vendor.sh --help       Affiche cette aide"
    echo ""
    echo -e "${BOLD}CORE REPOS${NC}  (gitlab.com/kyber/core)"
    for entry in "${CORE_REPOS[@]}"; do
        echo "  • ${entry%%|*}"
    done
    echo ""
    echo -e "${BOLD}DEPS REPOS${NC}  (gitlab.com/kyber/deps)"
    for entry in "${DEPS_REPOS[@]}"; do
        local name="${entry%%|*}"
        local note=""
        is_large "$name" && note="  ${YELLOW}[volumineux — clone partiel]${NC}"
        echo -e "  • $name$note"
    done
    echo ""
    echo -e "${BOLD}CHEMINS${NC}"
    echo "  Core      : $VENDOR_DIR/{kyutil,kymux,kymedia,kynput,kysdk}"
    echo "  Deps      : $DEPS_DIR/{keycode,libudev-sys,...,vlc,vlc-rs,winit}"
    echo "  Snapshots : $SNAPSHOT_DIR/"
    echo "  Log       : $LOG_FILE"
    echo ""
    echo -e "${BOLD}CONSEIL SÉCURITÉ${NC}"
    echo "  Kyber est sous licence AGPL + commerciale. En cas de passage"
    echo "  en closed source, lancez --snapshot pour conserver une copie"
    echo "  complète et utilisable de toutes les sources."
}

# ─── Main ────────────────────────────────────────────────────────────────────

main() {
    mkdir -p "$VENDOR_DIR" "$DEPS_DIR"

    # Init log
    {
        echo ""
        printf '═%.0s' {1..50}; echo ""
        echo "$(date '+%Y-%m-%d %H:%M:%S')  update_vendor.sh ${1:-}"
        printf '═%.0s' {1..50}; echo ""
    } >> "$LOG_FILE"

    check_deps

    local mode="${1:-all}"

    case "$mode" in

        --help|-h)
            print_help
            ;;

        --status|-s)
            print_status
            ;;

        --check|-c)
            check_network || { err "Réseau indisponible"; exit 1; }
            check_updates
            ;;

        --snapshot|-b|--backup)
            create_snapshot
            ;;

        --core)
            check_network || { err "Réseau indisponible"; exit 1; }
            update_group "CORE  (gitlab.com/kyber/core)" \
                "$VENDOR_DIR" "${CORE_REPOS[@]}"
            ;;

        --deps)
            check_network || { err "Réseau indisponible"; exit 1; }
            update_group "DEPS  (gitlab.com/kyber/deps)" \
                "$DEPS_DIR" "${DEPS_REPOS[@]}"
            ;;

        all|"")
            check_network || { err "Réseau indisponible"; exit 1; }
            local total_failed=0

            update_group "CORE  (gitlab.com/kyber/core)" \
                "$VENDOR_DIR" "${CORE_REPOS[@]}" || total_failed=$((total_failed + $?))

            update_group "DEPS  (gitlab.com/kyber/deps)" \
                "$DEPS_DIR" "${DEPS_REPOS[@]}" || total_failed=$((total_failed + $?))

            echo ""
            print_status

            if [ "$total_failed" -gt 0 ]; then
                err "$total_failed repo(s) ont échoué — voir $LOG_FILE"
                exit 1
            else
                echo ""
                ok "Tous les repos Kyber sont à jour."
                echo -e "\n  ${YELLOW}Conseil :${NC} sécurisez une copie avec ${BOLD}./update_vendor.sh --snapshot${NC}"
            fi
            ;;

        *)
            err "Option inconnue : $mode"
            print_help
            exit 1
            ;;
    esac
}

main "$@"
