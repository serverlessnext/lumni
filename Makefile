
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

# documentation
.PHONY: html
html:
	@# clean up any previous build
	-rm -rf ./build/doctrees ./build/html
	docker rm -f `docker ps -qaf name=$(DOCS_CONTAINER)` 2>/dev/null || exit 0
	@# build the docs in a container
	docker build -t $(DOCS_CONTAINER)-image -f docker/Dockerfile.docs .
	docker run --name $(DOCS_CONTAINER) -t $(DOCS_CONTAINER)-image make html
	@# copy the docs from the container to the host
	[ -d ./build ] || mkdir ./build
	docker cp $(DOCS_CONTAINER):/build/ ./build/docs_temp && \
		(cd ./build/docs_temp && mv doctrees html ../) && rm -rf ./build/docs_temp
	@# clean up the build container
	docker rm $(DOCS_CONTAINER)

.PHONY: docs
docs: python html
