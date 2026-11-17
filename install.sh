#!/bin/sh
# Instala o i3-launcher-rs no sistema.
#
# Uso:
#   ./install.sh                 # compila (release) e instala em ~/.local/bin
#   ./install.sh --no-build      # instala o binário já compilado
#   PREFIX=/usr/local ./install.sh
#   DESTDIR=/tmp/pkg PREFIX=/usr ./install.sh   # empacotamento
#
# Variáveis: PREFIX, BINDIR, DESTDIR, CARGO, CARGO_TARGET_DIR.

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
Uso: $0 [--no-build] [--help]

Compila o projeto (release) e instala o binário '$BIN'.

Variáveis de ambiente:
  PREFIX           diretório base de instalação (padrão: \$HOME/.local)
  BINDIR           diretório do binário (padrão: \$PREFIX/bin)
  DESTDIR          prefixo de empacotamento (padrão: vazio)
  CARGO            comando do cargo (padrão: cargo)
  CARGO_TARGET_DIR diretório de build do cargo
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
            echo "install.sh: opção desconhecida '$arg'" >&2
            usage >&2
            exit 2
            ;;
    esac
done

if [ "$BUILD" -eq 1 ]; then
    echo "==> compilando (release)"
    "$CARGO" build --release --locked --manifest-path "$SCRIPT_DIR/Cargo.toml"
fi

SOURCE="$TARGET_DIR/release/$BIN"
DESTINATION="$DESTDIR$BINDIR/$BIN"

if [ ! -f "$SOURCE" ]; then
    echo "install.sh: '$SOURCE' não existe; rode sem --no-build" >&2
    exit 1
fi

echo "==> instalando em $DESTINATION"
install -Dm755 "$SOURCE" "$DESTINATION"

if [ -z "$DESTDIR" ] && [ "$BINDIR" != "$HOME/.local/bin" ]; then
    case ":$PATH:" in
        *":$BINDIR:"*) ;;
        *) echo "dica: adicione '$BINDIR' ao PATH" ;;
    esac
fi

echo "pronto. No ~/.config/i3/config:"
echo "    bindsym \$mod+d exec --no-startup-id $BIN"
