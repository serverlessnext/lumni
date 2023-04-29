
NAME := lakestream
BUILD_VERSION ?= 0.0.3-alpha
DOCS_CONTAINER := $(NAME)-docs

DEFAULT_CRATES := lakestream lakestream-cli
EXTRA_CRATES := lakestream-web lakestream-py

export BUILD_VERSION

.PHONY: all $(DEFAULT_CRATES) $(EXTRA_CRATES)
all: $(DEFAULT_CRATES) $(EXTRA_CRATES)

$(DEFAULT_CRATES):
	cargo build -p $@ --release

# wasm32 build
lakestream-web:
	@echo "lakestream-web build temp disabled"
	@echo "  try: cd lakestream-web && trunk serve --open"
	@#wasm-pack build lakestream-web --release --target web --out-dir static/pkg

# python bindings
lakestream-py:
	cd lakestream-py && maturin build --release --strip --out dist

