#!/bin/sh
# TEMM1E — Linux system dependency installer
#
# Installs all system libraries needed to BUILD TEMM1E from source and/or
# RUN the pre-built desktop binary. Supports Debian/Ubuntu (apt), Fedora/
# RHEL (dnf), and Arch (pacman).
#
# Usage:
#   sh scripts/install-linux-deps.sh            # runtime + build deps
#   sh scripts/install-linux-deps.sh --runtime  # runtime only (to run pre-built)
#   sh scripts/install-linux-deps.sh --build    # build only (to compile)
#
# These packages cover:
#   - enigo  (libxdo, libxkbcommon, libwayland)     — input simulation
#   - xcap   (libxcb, libpipewire, libspa, libegl)  — screen capture
#   - misc   (libdrm, libgbm, libdbus, libxrandr)   — dependency closure
#
# Matches the CI deps in .github/workflows/{ci,release}.yml.

set -e

MODE="all"
for arg in "$@"; do
    case "$arg" in
        --runtime) MODE="runtime" ;;
        --build)   MODE="build" ;;
        --help|-h)
            echo "Usage: $0 [--runtime|--build]"
            echo ""
            echo "  (default)  Install both runtime and build dependencies"
            echo "  --runtime  Install only runtime libraries (to run pre-built binaries)"
            echo "  --build    Install only build dependencies (to compile from source)"
            exit 0
            ;;
        *)
            echo "Unknown argument: $arg"
            echo "Run '$0 --help' for usage."
            exit 1
            ;;
    esac
done

info()  { printf "> %s\n" "$1"; }
warn()  { printf "! %s\n" "$1"; }
error() { printf "x %s\n" "$1"; exit 1; }

if [ "$(uname -s)" != "Linux" ]; then
    error "This script is for Linux only. macOS gets its system libs via Xcode Command Line Tools."
fi

# Detect package manager
if command -v apt-get >/dev/null 2>&1; then
    PKG_MGR="apt"
elif command -v dnf >/dev/null 2>&1; then
    PKG_MGR="dnf"
elif command -v pacman >/dev/null 2>&1; then
    PKG_MGR="pacman"
else
    error "Unsupported package manager. Install manually — see README for details."
fi

info "Package manager: ${PKG_MGR}"
info "Mode: ${MODE}"

case "$PKG_MGR" in
    apt)
        APT_RUNTIME="libwayland-client0 libwayland-cursor0 libwayland-egl1 libxcb1 libxcb-randr0 libxcb-shm0 libxkbcommon0 libpipewire-0.3-0 libspa-0.2-modules libegl1 libgbm1 libdrm2 libxdo3 libdbus-1-3 libxrandr2"
        APT_BUILD="build-essential pkg-config libclang-dev libwayland-dev libxcb1-dev libxcb-randr0-dev libxcb-shm0-dev libxkbcommon-dev libpipewire-0.3-dev libspa-0.2-dev libegl1-mesa-dev libgbm-dev libdrm-dev libxdo-dev libdbus-1-dev libxrandr-dev"
        case "$MODE" in
            runtime) PKGS="$APT_RUNTIME" ;;
            build)   PKGS="$APT_BUILD" ;;
            all)     PKGS="$APT_RUNTIME $APT_BUILD" ;;
        esac
        info "sudo apt-get update"
        sudo apt-get update
        info "sudo apt-get install -y <packages>"
        # shellcheck disable=SC2086
        sudo apt-get install -y $PKGS
        ;;
    dnf)
        DNF_RUNTIME="wayland-libs-client libxkbcommon libxcb pipewire-libs mesa-libEGL mesa-libgbm libdrm dbus-libs libXrandr libxdo"
        DNF_BUILD="gcc make pkgconfig clang-devel wayland-devel libxkbcommon-devel libxcb-devel pipewire-devel mesa-libEGL-devel mesa-libgbm-devel libdrm-devel libxdo-devel dbus-devel libXrandr-devel"
        case "$MODE" in
            runtime) PKGS="$DNF_RUNTIME" ;;
            build)   PKGS="$DNF_BUILD" ;;
            all)     PKGS="$DNF_RUNTIME $DNF_BUILD" ;;
        esac
        info "sudo dnf install -y <packages>"
        # shellcheck disable=SC2086
        sudo dnf install -y $PKGS
        ;;
    pacman)
        # Arch ships headers inside the main package — no split runtime/build.
        PAC_PKGS="wayland libxkbcommon libxcb pipewire mesa libdrm dbus libxrandr xdotool"
        case "$MODE" in
            build|all) PAC_PKGS="$PAC_PKGS base-devel clang pkgconf" ;;
        esac
        info "sudo pacman -S --needed <packages>"
        # shellcheck disable=SC2086
        sudo pacman -S --needed $PAC_PKGS
        ;;
esac

echo ""
info "All dependencies installed"
echo ""
echo "  Next steps:"
echo "    • Install pre-built:  curl -sSfL https://raw.githubusercontent.com/temm1e-labs/temm1e/main/install.sh | sh"
echo "    • Build from source:  cargo build --release"
echo ""
