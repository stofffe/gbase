copy_window:
	@cp ../examples/window.rs ./src/lib.rs
	@make convert

copy_input:
	@cp ../examples/input.rs ./src/lib.rs
	@make convert

copy_time:
	@cp ../examples/time.rs ./src/lib.rs
	@make convert

copy_filesystem:
	@cp ../examples/filesystem.rs ./src/lib.rs
	@make convert

copy_audio:
	@cp ../examples/audio.rs ./src/lib.rs
	@make convert

copy_render:
	@cp ../examples/render.rs ./src/lib.rs
	@make convert

copy_triangle:
	@cp ../examples/triangle.rs ./src/lib.rs
	@make convert

copy_camera:
	@cp ../examples/camera.rs ./src/lib.rs
	@make convert

copy_transform:
	@cp ../examples/transform.rs ./src/lib.rs
	@make convert

copy_grass:
	@cp -p ../examples/grass.rs ./src/lib.rs
	@make convert

compile:
	@cp -r ../assets .
	@RUSTFLAGS=--cfg=web_sys_unstable_apis wasm-pack build --target web

convert:
	@sed -i "" "1s/^/use gbase::log;\n/" src/lib.rs
	@sed -i "" "1s/^/use gbase::winit;\n/" src/lib.rs
	@sed -i "" "1s/^/use gbase::wgpu;\n/" src/lib.rs
	@sed -i "" "1s/^/use gbase::glam;\n/" src/lib.rs
	@sed -i "" "1s/^/use gbase::bytemuck;\n/" src/lib.rs
	@sed -i "" "1s/^/use gbase::encase;\n/" src/lib.rs
	@sed -i "" "s/#\[pollster::main\]/#\[wasm_bindgen::prelude::wasm_bindgen\]/" src/lib.rs
	@sed -i "" "s/fn main/fn run/" src/lib.rs

run:
	@python3 -m http.server

open:
	@open -na "Google Chrome" --args --incognito "http://localhost:8000"

