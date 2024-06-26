.PHONY: update-changelog
.SHELL: /usr/bin/bash
.ONESHELL:

TARGET ?= x86_64-unknown-linux-gnu

update-changelog:
	git cliff > CHANGELOG.md

check-changelog:
	@git-cliff --output tmp.md
	@if diff tmp.md CHANGELOG.md; then
		@echo "CHANGELOG OK"
		@rm tmp.md
	@else
		@echo "CHANGELOG KO"
		@rm tmp.md
		@exit 1
	@fi

local-build:
	export RUSTFLAGS="-D warnings"
	cargo build

release-build:
	cross build --bin feed2imap --profile dist --target $(TARGET)

release-commit:
	$(if $(value VERSION),, $(error VERSION is not defined))
	sed -i -r -e 's/^version *= *.*$$/version = "$(VERSION)"/g' Cargo.toml
	git-cliff --tag $(VERSION) > CHANGELOG.md
	git commit --all --message "release v$(VERSION)"
	git tag v$(VERSION)

clean:
	cargo clean
