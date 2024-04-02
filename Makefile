
NAME := lumni
BUILD_VERSION ?= 0.0.4

DEFAULT_CRATES := lumni lumni_cli
EXTRA_CRATES := lumni_web # lumni_py

export BUILD_VERSION

.PHONY: all $(DEFAULT_CRATES) $(EXTRA_CRATES)
all: $(DEFAULT_CRATES) $(EXTRA_CRATES)

$(DEFAULT_CRATES):
	cargo build -p $@ --release

# wasm32 build
lumni_web:
	@echo "lumni-web build temp disabled"
	@echo "  try: cd lumni-web && trunk serve --open"
	@#wasm-pack build lumni-web --release --target web --out-dir static/pkg

# python bindings
lumni_py:
	cd lumni-py && maturin build --release --strip --out dist

tests:
	cargo test --package lumni
	cd lumni-web && wasm-pack test --headless --firefox

