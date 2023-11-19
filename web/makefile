copy_window:
	@cp ../examples/window.rs ./src/lib.rs
	@make convert

compile:
	@cp -r ../assets .
	@wasm-pack build --target web

convert:
	@sed -i "" "1s/^/use gbase::log;\n/" src/lib.rs
	@sed -i "" "1s/^/use gbase::winit;\n/" src/lib.rs
	@sed -i "" "1s/^/use gbase::wgpu;\n/" src/lib.rs
	@sed -i "" "1s/^/use gbase::glam;\n/" src/lib.rs
	@sed -i "" "s/#\[pollster::main\]/#\[wasm_bindgen::prelude::wasm_bindgen\]/" src/lib.rs
	@sed -i "" "s/main/run/" src/lib.rs

run:
	@python3 -m http.server

open:
	@open -na "Google Chrome" --args --incognito "http://localhost:8000"

