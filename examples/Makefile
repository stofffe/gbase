port = 8000
wasm_module_name = game

# dll debug
hot_reload_compile:
	cargo build --features "gbase/hot_reload,gbase/asset_reload"

hot_reload_compile_run:
	make hot_reload_compile
	cargo run --features "gbase/hot_reload,gbase/asset_reload"

hot_reload_compile_release:
	cargo build --features "gbase/hot_reload" --release

hot_reload_compile_run_release:
	make hot_reload_compile_release
	cargo run --features "gbase/hot_reload" --release

# web
web_compile:
	@RUSTFLAGS=--cfg=web_sys_unstable_apis wasm-pack build --target web --out-name=$(wasm_module_name)

web_run:
	@python3 -m http.server

web_open:
	@open -na "Google Chrome" --args --incognito "http://localhost:$(port)"

# static
debug:
	cargo run --features "gbase/asset_reload"

release:
	cargo run --release
