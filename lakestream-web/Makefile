
.PHONY: serve serve_release css css_live tests

serve:
	@# this runs in dev mode, so any debug_assertions will be true
	@# note: in dev-mode need to install trunk from git to have all-features working
	@# cargo install trunk --git https://github.com/thedodd/trunk
	trunk serve --all-features
serve_release:
	@# this runs in release mode, so any debug_assertions will be false
	trunk serve --release

css:
	npx tailwindcss -i ./input.css -o ./style/output.css

css_live:
	npx tailwindcss -i ./input.css -o ./style/output.css --watch

tests:
	wasm-pack test --headless --firefox

