
# keep this on top as default action
cargo:
	cargo build -p lakestream

# special build targets
web:
	wasm-pack build lakestream-web --release --target web --out-dir static/pkg
python:
	cd lakestream-py && maturin build --release --strip --out dist

