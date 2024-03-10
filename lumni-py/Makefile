
.PHONY: build
build:
	maturin build --release --strip --out dist

.PHONY: install
install:
	pip install --force-reinstall dist/*.whl

.PHONY: publish
publish:
	maturin publish \
		-i python3.7 python3.8 python3.9 python3.10 python3.11 --manylinux \
		--username __token__ --password $TWINE_PASSWORD
