# i3-launcher-rs — atalhos de desenvolvimento
#
#   make            # build release
#   make install    # compila e instala em ~/.local/bin
#   make uninstall  # remove o binário instalado
#   make test       # testes
#   make lint       # clippy + fmt --check
#   make run        # executa
#
# Instalação em outro local:
#   make install PREFIX=/usr/local
#   make install PREFIX=/usr DESTDIR=/tmp/pkg   # empacotamento

PREFIX ?= $(HOME)/.local
BINDIR ?= $(PREFIX)/bin
CARGO  ?= cargo

.PHONY: all build install uninstall test lint fmt run clean

all: build

build:
	$(CARGO) build --release

install:
	PREFIX="$(PREFIX)" BINDIR="$(BINDIR)" DESTDIR="$(DESTDIR)" CARGO="$(CARGO)" ./install.sh

uninstall:
	rm -f $(DESTDIR)$(BINDIR)/i3-launcher

test:
	$(CARGO) test

lint:
	$(CARGO) clippy --all-targets -- -D warnings
	$(CARGO) fmt --check

fmt:
	$(CARGO) fmt

run:
	$(CARGO) run --release

clean:
	$(CARGO) clean
