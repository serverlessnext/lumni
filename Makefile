
NAME := lakestream
DOCS_CONTAINER := $(NAME)-docs

# keep this on top as default action
.PHONY: cargo
cargo:
	cargo build -p lakestream

# special build targets
.PHONY: web
web:
	wasm-pack build lakestream-web --release --target web --out-dir static/pkg

.PHONY: python
python:
	cd lakestream-py && maturin build --release --strip --out dist

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
