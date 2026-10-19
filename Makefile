.PHONY: help doctor fmt fmt-check lint test test-integration test-chaos bench build run audit dist pkg dmg sign notarize verify-dist install uninstall clean

PREFIX ?= $(HOME)/.local
VERSION ?= 0.1.0
TARGET ?= aarch64-apple-darwin
BIN := target/release/pgm-data
DIST := dist
DEV_ID_APP ?= $(shell security find-identity -v -p codesigning 2>/dev/null | sed -n 's/.*"\(Developer ID Application:[^"]*\)".*/\1/p' | head -1)
DEV_ID_INSTALLER ?= $(shell security find-identity -v 2>/dev/null | sed -n 's/.*"\(Developer ID Installer:[^"]*\)".*/\1/p' | head -1)
NOTARY_PROFILE ?=

help:
	@echo "pgm-data — local CSV/Parquet Bloom matching. Build and test do not use sudo and do not install global packages."
	@echo "Prerequisites: Rust $(shell rustc --version 2>/dev/null || echo 'stable 1.94+'), Xcode command line tools for pkgbuild/hdiutil."
	@echo "Data lives in ~/Library/Application Support/pgm-data and is not part of the install tree."
	@echo ""
	@echo "help               Show this text"
	@echo "doctor             Run pgm-data doctor against a temporary data dir"
	@echo "fmt / fmt-check    rustfmt write / check"
	@echo "lint               clippy, warnings denied"
	@echo "test               Unit and integration tests (simulated HTTP only)"
	@echo "test-integration   Tests under tests/"
	@echo "test-chaos         Fault, lock, and crash-window tests"
	@echo "bench              Local 1M-row synthetic Bloom run. No network calls."
	@echo "build              Release binary for this machine"
	@echo "run                Run the release binary"
	@echo "audit              cargo audit"
	@echo "dist               Release binary plus SHA-256"
	@echo "pkg                Unsigned component package (no install)"
	@echo "dmg                Disk image containing the package and README"
	@echo "sign               Developer ID Application signature when the identity exists"
	@echo "notarize           notarytool when NOTARY_PROFILE is set"
	@echo "verify-dist        Checksums and package payload"
	@echo "install            Copy the binary to PREFIX ($(PREFIX)); no sudo"
	@echo "uninstall          Remove only PREFIX/bin/pgm-data"
	@echo "clean              Remove target/ and dist/ only"

doctor:
	PGM_DATA_HOME=$(CURDIR)/target/doctor-home cargo run --quiet -- doctor

fmt:
	cargo fmt --all

fmt-check:
	cargo fmt --all -- --check

lint:
	cargo clippy --all-targets --all-features -- -D warnings

test:
	cargo test --all-targets --all-features

test-integration:
	cargo test --tests --all-features

test-chaos:
	cargo test --test chaos --all-features

bench:
	cargo run --release --bin pgm-bench -- --rows 1000000

build:
	cargo build --release

run: build
	./$(BIN) --help

audit:
	cargo audit

dist: build
	mkdir -p $(DIST)
	cp $(BIN) $(DIST)/pgm-data-$(TARGET)
	shasum -a 256 $(DIST)/pgm-data-$(TARGET) > $(DIST)/SHA256SUMS

pkg: dist
	mkdir -p $(DIST)/root/usr/local/pgm-data/bin $(DIST)/scripts
	cp $(DIST)/pgm-data-$(TARGET) $(DIST)/root/usr/local/pgm-data/bin/pgm-data
	chmod 755 $(DIST)/root/usr/local/pgm-data/bin/pgm-data
	cp packaging/macos/README.txt $(DIST)/root/usr/local/pgm-data/README.txt
	cp packaging/macos/scripts/uninstall.sh $(DIST)/root/usr/local/pgm-data/uninstall.sh
	chmod 755 $(DIST)/root/usr/local/pgm-data/uninstall.sh
	pkgbuild --root $(DIST)/root --identifier com.pgm-data.cli --version $(VERSION) \
		--install-location / --scripts packaging/macos/scripts \
		$(DIST)/pgm-data-$(VERSION)-$(TARGET)-unsigned.pkg
	shasum -a 256 $(DIST)/pgm-data-$(VERSION)-$(TARGET)-unsigned.pkg >> $(DIST)/SHA256SUMS

dmg: pkg
	rm -rf $(DIST)/dmg && mkdir -p $(DIST)/dmg
	cp $(DIST)/pgm-data-$(VERSION)-$(TARGET)-unsigned.pkg $(DIST)/dmg/
	cp packaging/macos/README.txt $(DIST)/dmg/README.txt
	cp $(DIST)/SHA256SUMS $(DIST)/dmg/SHA256SUMS
	hdiutil create -volname "pgm-data $(VERSION)" -srcfolder $(DIST)/dmg -ov -format UDZO \
		$(DIST)/pgm-data-$(VERSION)-$(TARGET).dmg
	shasum -a 256 $(DIST)/pgm-data-$(VERSION)-$(TARGET).dmg >> $(DIST)/SHA256SUMS

sign:
	@if [ -z "$(DEV_ID_APP)" ]; then echo "sign pending: Developer ID Application identity not found"; exit 0; fi
	codesign --force --options runtime --timestamp --sign "$(DEV_ID_APP)" $(BIN)
	codesign --verify --strict --verbose=2 $(BIN)

notarize:
	@if [ -z "$(NOTARY_PROFILE)" ]; then echo "notarize pending: set NOTARY_PROFILE to a notarytool keychain profile"; exit 0; fi
	@test -f $(DIST)/pgm-data-$(VERSION)-$(TARGET).dmg
	xcrun notarytool submit $(DIST)/pgm-data-$(VERSION)-$(TARGET).dmg --keychain-profile "$(NOTARY_PROFILE)" --wait
	xcrun stapler staple $(DIST)/pgm-data-$(VERSION)-$(TARGET).dmg

verify-dist:
	shasum -a 256 -c $(DIST)/SHA256SUMS
	pkgutil --payload-files $(DIST)/pgm-data-$(VERSION)-$(TARGET)-unsigned.pkg | sort

install: build
	mkdir -p $(PREFIX)/bin
	install -m 755 $(BIN) $(PREFIX)/bin/pgm-data

uninstall:
	rm -f $(PREFIX)/bin/pgm-data

clean:
	cargo clean
	rm -rf $(DIST)
