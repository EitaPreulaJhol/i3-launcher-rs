# launcher-rs — development shortcuts
#
#   make            # release build
#   make install    # build and install into ~/.local/bin
#   make uninstall  # remove the installed binary and manual page
#   make test       # tests
#   make lint       # clippy + fmt --check
#   make bench      # criterion benchmarks (target/criterion/report)
#   make man        # preview the manual page
#   make run        # run
#
# Installing elsewhere:
#   make install PREFIX=/usr/local
#   make install PREFIX=/usr DESTDIR=/tmp/pkg   # packaging

PREFIX ?= $(HOME)/.local
BINDIR ?= $(PREFIX)/bin
MANDIR ?= $(PREFIX)/share/man
CARGO  ?= cargo

.PHONY: all build install uninstall test lint fmt bench man run clean

all: build

build:
	$(CARGO) build --release

install:
	PREFIX="$(PREFIX)" BINDIR="$(BINDIR)" MANDIR="$(MANDIR)" DESTDIR="$(DESTDIR)" CARGO="$(CARGO)" ./install.sh

uninstall:
	rm -f "$(DESTDIR)$(BINDIR)/launcher-rs"
	rm -f "$(DESTDIR)$(MANDIR)/man1/launcher-rs.1"

test:
	$(CARGO) test

lint:
	$(CARGO) clippy --all-targets -- -D warnings
	$(CARGO) fmt --check

fmt:
	$(CARGO) fmt

bench:
	$(CARGO) bench

man:
	man -l docs/launcher-rs.1

run:
	$(CARGO) run --release

clean:
	$(CARGO) clean
