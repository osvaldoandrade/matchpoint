#!/usr/bin/env bash
# MatchPoint source installation for macOS, Linux and WSL2.
set -euo pipefail

INSTALL_REPOSITORY=https://github.com/osvaldoandrade/matchpoint.git
INSTALL_PREFIX=${MATCHPOINT_PREFIX:-"$HOME/.local"}
INSTALL_REF=main
INSTALL_SYSTEM_DEPS=1
INSTALL_PATH=1
INSTALL_STAGE=preflight
INSTALL_TEMP=
INSTALL_LOCK=
INSTALL_GH_VERSION=2.102.1
INSTALL_CMAKE_VERSION=4.4.4
INSTALL_CARGO=(cargo)

info() { printf 'MatchPoint: %s\n' "$*"; }
die() { printf 'MatchPoint: %s\n' "$*" >&2; exit 1; }
need() { command -v "$1" >/dev/null 2>&1; }

cleanup() {
    local result=$?
    [[ -z "$INSTALL_TEMP" ]] || rm -rf -- "$INSTALL_TEMP"
    [[ -z "$INSTALL_LOCK" ]] || rmdir -- "$INSTALL_LOCK" 2>/dev/null || true
    printf 'MatchPoint: result=%s stage=%s elapsed_seconds=%s\n' "$result" "$INSTALL_STAGE" "$SECONDS"
}

arguments() {
    while [[ $# -gt 0 ]]; do
        case "$1" in
            --prefix|--ref)
                [[ $# -ge 2 && -n "$2" ]] || die "$1 requires a value."
                case "$1" in --prefix) INSTALL_PREFIX=$2 ;; --ref) INSTALL_REF=$2 ;; esac
                shift 2 ;;
            --skip-system-deps) INSTALL_SYSTEM_DEPS=0; shift ;;
            --no-path) INSTALL_PATH=0; shift ;;
            --help)
                printf '%s\n' 'Usage: bash install.sh [--prefix DIR] [--ref REF] [--skip-system-deps] [--no-path]'
                exit 0 ;;
            *) die "Unknown option: $1" ;;
        esac
    done
    [[ "$INSTALL_PREFIX" == /* && "$INSTALL_PREFIX" != / ]] || die 'Use an absolute user installation prefix.'
    case "$INSTALL_PREFIX" in *$'\n'*|*$'\r'*|*:*) die 'Invalid installation prefix.' ;; esac
    case "$INSTALL_REF" in ''|[-.]*|*[!a-zA-Z0-9._/-]*) die 'Invalid Git ref.' ;; esac
}

download() {
    curl --proto '=https' --tlsv1.2 --fail --silent --show-error --location \
        --connect-timeout 15 --max-time 180 --output "$2" "$1"
}

verify_checksum() {
    local expected actual
    expected=$(awk -v name="$(basename "$1")" '$2 == name {print $1}' "$2")
    [[ "$expected" =~ ^[a-fA-F0-9]{64}$ ]] || die 'Download checksum was not found.'
    if need sha256sum; then actual=$(sha256sum "$1"); else actual=$(shasum -a 256 "$1"); fi
    [[ "${actual%% *}" == "$expected" ]] || die 'Download checksum mismatch.'
}

privileged() {
    if [[ "$(id -u)" == 0 ]]; then "$@"; else
        need sudo || die 'sudo is required to install missing system build tools.'
        sudo "$@"
    fi
}

system_dependencies() {
    local missing=0 tool
    for tool in git curl cc c++ make cmake perl; do need "$tool" || missing=1; done
    [[ "$missing" == 1 ]] || return 0
    [[ "$INSTALL_SYSTEM_DEPS" == 1 ]] || die 'Missing build tools; rerun without --skip-system-deps.'
    INSTALL_STAGE=system-tools
    if [[ "$INSTALL_OS" == Darwin ]]; then
        if ! xcode-select -p >/dev/null 2>&1; then
            xcode-select --install || true
            die 'Complete the Apple Command Line Tools dialog, then rerun this installer.'
        fi
        if ! need cmake; then
            local asset="cmake-${INSTALL_CMAKE_VERSION}-macos-universal.tar.gz"
            local base="https://github.com/Kitware/CMake/releases/download/v${INSTALL_CMAKE_VERSION}"
            download "$base/$asset" "$INSTALL_TEMP/$asset"
            download "$base/cmake-${INSTALL_CMAKE_VERSION}-SHA-256.txt" "$INSTALL_TEMP/cmake-checksums"
            verify_checksum "$INSTALL_TEMP/$asset" "$INSTALL_TEMP/cmake-checksums"
            mkdir "$INSTALL_TEMP/cmake"
            tar -xzf "$INSTALL_TEMP/$asset" -C "$INSTALL_TEMP/cmake" --strip-components=1
            mkdir -p "$INSTALL_PREFIX/lib/matchpoint/tools"
            mv "$INSTALL_TEMP/cmake/CMake.app" "$INSTALL_PREFIX/lib/matchpoint/tools/CMake.app"
            export PATH="$INSTALL_PREFIX/lib/matchpoint/tools/CMake.app/Contents/bin:$PATH"
        fi
    elif need apt-get; then
        privileged apt-get update
        privileged apt-get install -y build-essential git curl ca-certificates cmake perl pkg-config
    elif need dnf; then
        privileged dnf install -y gcc gcc-c++ make git curl ca-certificates cmake perl pkgconf-pkg-config
    elif need pacman; then
        privileged pacman -S --needed --noconfirm base-devel git curl ca-certificates cmake perl pkgconf
    elif need zypper; then
        privileged zypper --non-interactive install gcc gcc-c++ make git curl ca-certificates cmake perl pkg-config
    else die 'Install git, curl, a C/C++ compiler, make, cmake and perl with your system package manager.'
    fi
    for tool in git curl cc c++ make cmake perl; do need "$tool" || die "Build tool still missing: $tool"; done
}

github_cli() {
    need gh && return 0
    local platform=linux extension=tar.gz architecture asset base
    [[ "$INSTALL_OS" != Darwin ]] || { platform=macOS; extension=zip; }
    case "$(uname -m)" in arm64|aarch64) architecture=arm64 ;; x86_64|amd64) architecture=amd64 ;;
        *) die 'Install GitHub CLI from cli.github.com for this architecture.' ;; esac
    asset="gh_${INSTALL_GH_VERSION}_${platform}_${architecture}.${extension}"
    base="https://github.com/cli/cli/releases/download/v${INSTALL_GH_VERSION}"
    download "$base/$asset" "$INSTALL_TEMP/$asset"
    download "$base/gh_${INSTALL_GH_VERSION}_checksums.txt" "$INSTALL_TEMP/checksums"
    verify_checksum "$INSTALL_TEMP/$asset" "$INSTALL_TEMP/checksums"
    mkdir "$INSTALL_TEMP/gh"
    if [[ "$extension" == zip ]]; then
        unzip -q "$INSTALL_TEMP/$asset" -d "$INSTALL_TEMP/gh"
    else tar -xzf "$INSTALL_TEMP/$asset" -C "$INSTALL_TEMP/gh"; fi
    local extracted="$INSTALL_TEMP/gh/gh_${INSTALL_GH_VERSION}_${platform}_${architecture}/bin/gh"
    [[ -x "$extracted" ]] || die 'GitHub CLI archive has an unexpected layout.'
    mkdir -p "$INSTALL_PREFIX/lib/matchpoint/tools"
    install -m 755 "$extracted" "$INSTALL_PREFIX/lib/matchpoint/tools/gh"
    export PATH="$INSTALL_PREFIX/lib/matchpoint/tools:$PATH"
}

source_access() {
    INSTALL_STAGE=source-access
    if GIT_TERMINAL_PROMPT=0 git ls-remote "$INSTALL_REPOSITORY" HEAD >/dev/null 2>&1; then return; fi
    github_cli
    if ! gh auth status --hostname github.com >/dev/null 2>&1; then
        [[ -r /dev/tty ]] || die 'Authorize GitHub access in an interactive terminal, then rerun.'
        info 'Sign in to GitHub in the browser; your account must have access to MatchPoint.'
        gh auth login --hostname github.com --git-protocol https --web </dev/tty
    fi
    gh auth setup-git --hostname github.com
    GIT_TERMINAL_PROMPT=0 git ls-remote "$INSTALL_REPOSITORY" HEAD >/dev/null 2>&1 \
        || die 'The signed-in GitHub account cannot read osvaldoandrade/matchpoint.'
}

source_checkout() {
    INSTALL_STAGE=source
    INSTALL_SOURCE="$INSTALL_PREFIX/share/matchpoint/source"
    if [[ -e "$INSTALL_SOURCE" || -L "$INSTALL_SOURCE" ]]; then
        [[ ! -L "$INSTALL_SOURCE" && -d "$INSTALL_SOURCE/.git" ]] || die 'Source destination is not a managed clone.'
        [[ "$(git -C "$INSTALL_SOURCE" config --get matchpoint.managed)" == true ]] \
            || die 'Refusing to change an unmanaged source checkout.'
        [[ "$(git -C "$INSTALL_SOURCE" config --get remote.origin.url)" == "$INSTALL_REPOSITORY" ]] \
            || die 'Managed source origin does not match MatchPoint.'
        [[ -z "$(git -C "$INSTALL_SOURCE" status --porcelain)" ]] || die 'Source has local edits; preserve them before updating.'
    else
        mkdir -p "$(dirname "$INSTALL_SOURCE")"
        GIT_TERMINAL_PROMPT=0 git clone --no-checkout "$INSTALL_REPOSITORY" "$INSTALL_SOURCE"
        git -C "$INSTALL_SOURCE" config matchpoint.managed true
    fi
    GIT_TERMINAL_PROMPT=0 git -C "$INSTALL_SOURCE" -c http.lowSpeedLimit=1 -c http.lowSpeedTime=60 \
        fetch --depth 1 origin "$INSTALL_REF"
    git -C "$INSTALL_SOURCE" checkout --detach FETCH_HEAD
    INSTALL_COMMIT=$(git -C "$INSTALL_SOURCE" rev-parse HEAD)
    [[ "$INSTALL_COMMIT" =~ ^[a-f0-9]{40}$ ]] || die 'Invalid source commit identity.'
}

rust_toolchain() {
    INSTALL_STAGE=rust
    INSTALL_RUST=$(sed -n 's/^channel = "\([0-9][0-9.]*\)"$/\1/p' "$INSTALL_SOURCE/rust-toolchain.toml")
    [[ "$INSTALL_RUST" =~ ^[0-9]+\.[0-9]+\.[0-9]+$ ]] || die 'Expected an exact Rust toolchain version.'
    export PATH="${CARGO_HOME:-$HOME/.cargo}/bin:$PATH"
    if ! need rustup && need cargo && need rustc && [[ "$(rustc --version)" == "rustc $INSTALL_RUST "* ]]; then
        return 0
    fi
    if ! need rustup; then
        download https://sh.rustup.rs "$INSTALL_TEMP/rustup.sh"
        sh "$INSTALL_TEMP/rustup.sh" -y --profile minimal --default-toolchain none --no-modify-path
    fi
    if ! rustup run "$INSTALL_RUST" rustc --version >/dev/null 2>&1; then
        rustup toolchain install "$INSTALL_RUST" --profile minimal
    fi
    INSTALL_CARGO=(cargo "+$INSTALL_RUST")
}

owned_link() {
    local path=$1 target=$2 current
    if [[ -e "$path" || -L "$path" ]]; then
        [[ -L "$path" ]] || die "Refusing to replace an unrelated executable: $path"
        current=$(readlink "$path")
        [[ "$current" == "$INSTALL_PREFIX/lib/matchpoint/versions/"*/pgm-data ]] \
            || die "Refusing to replace an unrelated link: $path"
    fi
    local staged="$path.matchpoint-$$"
    [[ ! -e "$staged" && ! -L "$staged" ]] || die "Install staging path already exists: $staged"
    ln -s "$target" "$staged"
    mv -f "$staged" "$path"
}

build_and_install() {
    INSTALL_STAGE=build
    local version="$INSTALL_PREFIX/lib/matchpoint/versions/$INSTALL_COMMIT"
    [[ ! -L "$INSTALL_PREFIX/lib/matchpoint/versions" && ! -L "$version" && ! -L "$version/pgm-data" ]] \
        || die 'Refusing a symlinked version destination.'
    # Validate ownership before spending time building or replacing launchers.
    for name in matchpoint pgm-data; do
        local path="$INSTALL_PREFIX/bin/$name"
        if [[ -e "$path" || -L "$path" ]]; then
            [[ -L "$path" && "$(readlink "$path")" == "$INSTALL_PREFIX/lib/matchpoint/versions/"*/pgm-data ]] \
                || die "Refusing to replace an unrelated executable: $path"
        fi
    done
    (cd "$INSTALL_SOURCE"; "${INSTALL_CARGO[@]}" build --locked --release --bin pgm-data --target-dir target)
    "$INSTALL_SOURCE/target/release/pgm-data" --version
    INSTALL_STAGE=publish
    mkdir -p "$version" "$INSTALL_PREFIX/bin"
    [[ ! -e "$version/.pgm-data-$$" && ! -L "$version/.pgm-data-$$" ]] || die 'Binary staging path already exists.'
    install -m 755 "$INSTALL_SOURCE/target/release/pgm-data" "$version/.pgm-data-$$"
    mv "$version/.pgm-data-$$" "$version/pgm-data"
    owned_link "$INSTALL_PREFIX/bin/matchpoint" "$version/pgm-data"
    owned_link "$INSTALL_PREFIX/bin/pgm-data" "$version/pgm-data"
    printf 'commit=%s\nrust=%s\n' "$INSTALL_COMMIT" "$INSTALL_RUST" \
        > "$INSTALL_PREFIX/share/matchpoint/installation.txt"
}

configure_path() {
    [[ "$INSTALL_PATH" == 1 ]] || return 0
    INSTALL_STAGE=path
    local envfile="$INSTALL_PREFIX/share/matchpoint/env" profile quoted
    # Single-quote paths so shell metacharacters remain literal in profiles.
    quoted=$(printf '%s' "$INSTALL_PREFIX/bin" | sed "s/'/'\\\\''/g")
    printf "export PATH='%s':\"\$PATH\"\n" "$quoted" > "$envfile"
    quoted=$(printf '%s' "$envfile" | sed "s/'/'\\\\''/g")
    for profile in "${@:-$HOME/.profile}"; do
        if [[ -L "$profile" ]]; then
            info "Preserving symlinked profile: $profile; source the generated env file manually."
            continue
        fi
        local line="[ ! -f '$quoted' ] || . '$quoted' # MatchPoint installer"
        if [[ ! -f "$profile" ]] || ! grep -Fqx "$line" "$profile"; then
            printf '\n%s\n' "$line" >> "$profile"
        fi
    done
    info "Open a new terminal, or run: . '$quoted'"
}

main() {
    arguments "$@"
    INSTALL_OS=$(uname -s)
    case "$INSTALL_OS" in Darwin|Linux) ;; *) die 'Use macOS/Linux, or install.ps1 for Windows with WSL2.' ;; esac
    [[ "$(id -u)" != 0 ]] || die 'Run as your normal user; only system package installation uses sudo.'
    umask 077
    local directory marker
    for directory in "$INSTALL_PREFIX" "$INSTALL_PREFIX/lib" "$INSTALL_PREFIX/share" "$INSTALL_PREFIX/bin"; do
        [[ ! -L "$directory" ]] || die "Refusing a symlinked installation directory: $directory"
    done
    for directory in "$INSTALL_PREFIX/lib/matchpoint" "$INSTALL_PREFIX/share/matchpoint"; do
        marker="$directory/.installer-owned"
        [[ ! -L "$directory" && ! -L "$marker" ]] || die 'Refusing a symlinked installer namespace.'
        if [[ -e "$directory" ]]; then
            [[ -f "$marker" && "$(cat "$marker")" == matchpoint-source-installer-v1 ]] \
                || die "Refusing an unmanaged installation directory: $directory"
        else
            mkdir -p "$directory"
            printf 'matchpoint-source-installer-v1\n' > "$marker"
        fi
    done
    export PATH="$INSTALL_PREFIX/lib/matchpoint/tools/CMake.app/Contents/bin:$INSTALL_PREFIX/lib/matchpoint/tools:$PATH"
    INSTALL_LOCK="$INSTALL_PREFIX/lib/matchpoint/install.lock"
    mkdir "$INSTALL_LOCK" 2>/dev/null || { INSTALL_LOCK=; die 'Another installation is running; inspect install.lock.'; }
    trap cleanup EXIT
    trap 'exit 130' INT
    trap 'exit 143' TERM
    INSTALL_TEMP=$(mktemp -d "${TMPDIR:-/tmp}/matchpoint-install.XXXXXXXX")
    info "Installing for $INSTALL_OS in $INSTALL_PREFIX"
    system_dependencies
    source_access
    source_checkout
    rust_toolchain
    build_and_install
    configure_path "$HOME/.profile" "$HOME/.bash_profile" "$HOME/.bashrc" "$HOME/.zshrc"
    INSTALL_STAGE=complete
    info "Installed commit $INSTALL_COMMIT. Start with: matchpoint tui"
    info 'Application credentials, DAT settings and user jobs were not changed.'
}

if [[ "${BASH_SOURCE[0]:-$0}" == "$0" ]]; then main "$@"; fi
