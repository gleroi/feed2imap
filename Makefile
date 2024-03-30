.PHONY: update-changelog
.SHELL: /usr/bin/bash
.ONESHELL:

TARGET ?= x86_64-unknown-linux-gnu

update-changelog:
	git cliff > CHANGELOG.md

local-build:
	export RUSTFLAGS="-D warnings"
	cargo build

release-build:
	cross build --bin feed2imap --profile dist --target $(TARGET)

clean:
	cargo clean
