
.DEFAULT_GOAL := help

.PHONY: help
help:
	@echo "Available targets:"
	@echo "  minio_build      - Build MinIO container image"
	@echo "  minio_run        - Run MinIO container"
	@echo "  minio_stop       - Stop MinIO container"
	@echo "  minio_clean      - Stop and remove MinIO container"
	@echo "  docs_html        - Build HTML documentation"


## Minio targets
MINIO_IMAGE := minio-dev-image
MINIO_CONTAINER := minio-dev-run
MINIO_DOCKERFILE := Dockerfile.minio
MINIO_SERVER_URL := https://localhost:8443

.PHONY: all minio_build minio_run minio_stop minio_clean

all: minio_run

minio_build:
	docker build -q -t $(MINIO_IMAGE) -f $(MINIO_DOCKERFILE) .

minio_run: minio_build
	@if [ -z "`docker ps -q -f name=$(MINIO_CONTAINER)`" ]; then \
		if [ -n "`docker ps -aq -f status=exited -f name=$(MINIO_CONTAINER)`" ]; then \
			docker rm $(MINIO_CONTAINER); \
		fi; \
		docker run -d --name $(MINIO_CONTAINER) -p 8443:8443 -p 9001:9001 \
		-e MINIO_SERVER_URL=$(MINIO_SERVER_URL) \
		$(MINIO_IMAGE) server /mnt/localhost --certs-dir /.minio/certs --address :8443 --console-address :9001; \
	fi
	@echo "MinIO container is running!"
	@echo "You can now access the MinIO web interface:"
	@echo ""
	@echo "Minio Server URL: $(MINIO_SERVER_URL)"
	@docker logs $(MINIO_CONTAINER)


minio_stop:
	-docker stop $(MINIO_CONTAINER)

minio_clean: minio_stop
	-docker rm $(MINIO_CONTAINER)


## Documentation targets
DOCS_IMAGE := lakestream-docs-image
DOCS_CONTAINER := lakestream-docs-run

.PHONY: docs_html

docs_html:
	@# clean up any previous build
	-rm -rf ../build/doctrees ../build/html
	docker rm -f `docker ps -qaf name=$(DOCS_CONTAINER)` 2>/dev/null || exit 0
	@# build the docs in a container
	docker build -t $(DOCS_IMAGE) -f Dockerfile.docs ../
	docker run --name $(DOCS_CONTAINER) -t $(DOCS_IMAGE) make html
	@# copy the docs from the container to the host
	[ -d ../build ] || mkdir ../build
	docker cp $(DOCS_CONTAINER):/build/ ../build/docs_temp && \
		(cd ../build/docs_temp && mv doctrees html ../) && rm -rf ../build/docs_temp
	@# clean up the build container
	docker rm $(DOCS_CONTAINER)

