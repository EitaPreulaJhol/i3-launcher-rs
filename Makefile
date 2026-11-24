# launcher-rs — development shortcuts
#
#   make            # release build
#   make install    # build and install into ~/.local/bin
#   make uninstall  # remove the installed binary
#   make test       # tests
#   make lint       # clippy + fmt --check
#   make run        # run
#
# Installing elsewhere:
#   make install PREFIX=/usr/local
#   make install PREFIX=/usr DESTDIR=/tmp/pkg   # packaging

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
	rm -f $(DESTDIR)$(BINDIR)/launcher-rs

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
