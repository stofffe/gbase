all:
	@make copy
	@make compile
	@make open
	@make run
	
copy:
	@cp -p ../$(path) src/lib.rs
	@sed -i "" "1s/^/use gbase::log;\n/" src/lib.rs
	@sed -i "" "1s/^/use gbase::winit;\n/" src/lib.rs
	@sed -i "" "1s/^/use gbase::wgpu;\n/" src/lib.rs
	@sed -i "" "1s/^/use gbase::glam;\n/" src/lib.rs
	@sed -i "" "1s/^/use gbase::bytemuck;\n/" src/lib.rs
	@sed -i "" "1s/^/use gbase::encase;\n/" src/lib.rs
	@sed -i "" "s/#\[pollster::main\]/#\[wasm_bindgen::prelude::wasm_bindgen\]/" src/lib.rs
	@sed -i "" "s/fn main/fn run/" src/lib.rs

compile:
	@RUSTFLAGS=--cfg=web_sys_unstable_apis wasm-pack build --target web

run:
	@python3 -m http.server

open:
	@open -na "Google Chrome" --args --incognito "http://localhost:8000"
