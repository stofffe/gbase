name = shadows
port = 8000

# dll debug
hot_reload_compile:
	cargo build --target-dir target --features "hot_reload"
	mv target/debug/lib$(name).dylib lib$(name).dylib

hot_reload_compile_run:
	make hot_reload_compile
	cargo run --features "hot_reload"

hot_reload_compile_release:
	cargo build --target-dir target --features "hot_reload" --release
	mv target/release/lib$(name).dylib lib$(name).dylib

hot_reload_compile_run_release:
	make hot_reload_compile_release
	cargo run --features "hot_reload" --release

# web
web_compile:
	@RUSTFLAGS=--cfg=web_sys_unstable_apis wasm-pack build --target web

web_run:
	@python3 -m http.server

web_open:
	@open -na "Google Chrome" --args --incognito "http://localhost:$(port)"

# static
debug:
	cargo run --debug

release:
	cargo run --release
