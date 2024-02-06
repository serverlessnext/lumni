
NAME := xlatti
BUILD_VERSION ?= 0.0.4-alpha

DEFAULT_CRATES := xlatti xlatti-cli
EXTRA_CRATES := xlatti-web xlatti-py

#export BUILD_VERSION

.PHONY: all $(DEFAULT_CRATES) $(EXTRA_CRATES)
all: $(DEFAULT_CRATES) $(EXTRA_CRATES)

$(DEFAULT_CRATES):
	cargo build -p $@ --release

# wasm32 build
xlatti-web:
	@echo "xlatti-web build temp disabled"
	@echo "  try: cd xlatti-web && trunk serve --open"
	@#wasm-pack build xlatti-web --release --target web --out-dir static/pkg

# python bindings
xlatti-py:
	cd xlatti-py && maturin build --release --strip --out dist

tests:
	cargo test --package xlatti
	cd xlatti-web && wasm-pack test --headless --firefox

