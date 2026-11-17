#!/bin/sh
# Install i3-launcher-rs on the system.
#
# Usage:
#   ./install.sh                 # build (release) and install into ~/.local/bin
#   ./install.sh --no-build      # install the already-built binary
#   PREFIX=/usr/local ./install.sh
#   DESTDIR=/tmp/pkg PREFIX=/usr ./install.sh   # packaging
#
# Variables: PREFIX, BINDIR, DESTDIR, CARGO, CARGO_TARGET_DIR.

set -eu

SCRIPT_DIR=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)
CARGO=${CARGO:-cargo}
PREFIX=${PREFIX:-"$HOME/.local"}
BINDIR=${BINDIR:-"$PREFIX/bin"}
DESTDIR=${DESTDIR:-}
TARGET_DIR=${CARGO_TARGET_DIR:-"$SCRIPT_DIR/target"}
BIN=i3-launcher

usage() {
    cat <<EOF
Usage: $0 [--no-build] [--help]

Builds the project (release) and installs the '$BIN' binary.

Environment variables:
  PREFIX           base installation directory (default: \$HOME/.local)
  BINDIR           binary directory (default: \$PREFIX/bin)
  DESTDIR          packaging prefix (default: empty)
  CARGO            cargo command (default: cargo)
  CARGO_TARGET_DIR cargo build directory
EOF
}

BUILD=1
for arg in "$@"; do
    case "$arg" in
        --no-build) BUILD=0 ;;
        -h|--help)
            usage
            exit 0
            ;;
        *)
            echo "install.sh: unknown option '$arg'" >&2
            usage >&2
            exit 2
            ;;
    esac
done

if [ "$BUILD" -eq 1 ]; then
    echo "==> building (release)"
    "$CARGO" build --release --locked --manifest-path "$SCRIPT_DIR/Cargo.toml"
fi

SOURCE="$TARGET_DIR/release/$BIN"
DESTINATION="$DESTDIR$BINDIR/$BIN"

if [ ! -f "$SOURCE" ]; then
    echo "install.sh: '$SOURCE' does not exist; rerun without --no-build" >&2
    exit 1
fi

echo "==> installing into $DESTINATION"
install -Dm755 "$SOURCE" "$DESTINATION"

if [ -z "$DESTDIR" ] && [ "$BINDIR" != "$HOME/.local/bin" ]; then
    case ":$PATH:" in
        *":$BINDIR:"*) ;;
        *) echo "hint: add '$BINDIR' to PATH" ;;
    esac
fi

echo "done. In ~/.config/i3/config:"
echo "    bindsym \$mod+d exec --no-startup-id $BIN"
