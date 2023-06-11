
.PHONY: serve serve_release css css_live tests

serve:
	@# this runs in dev mode, so any debug_assertions will be true
	trunk serve

serve_release:
	@# this runs in release mode, so any debug_assertions will be false
	trunk serve --release

css:
	npx tailwindcss -i ./input.css -o ./style/output.css

css_live:
	npx tailwindcss -i ./input.css -o ./style/output.css --watch

tests:
	wasm-pack test --headless --firefox

