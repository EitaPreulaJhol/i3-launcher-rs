# i3-launcher-rs — atalhos de desenvolvimento
#
#   make            # build release
#   make install    # instala em ~/.local/bin
#   make test       # testes
#   make lint       # clippy + fmt --check
#   make run        # executa

PREFIX ?= $(HOME)/.local
BINDIR ?= $(PREFIX)/bin
CARGO  ?= cargo

.PHONY: all build install uninstall test lint fmt run clean

all: build

build:
	$(CARGO) build --release

install: build
	install -Dm755 target/release/i3-launcher $(DESTDIR)$(BINDIR)/i3-launcher

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
