# debug
compile_debug:
	cargo build --target-dir target --features "hot_reload"
	mv target/debug/libhot_reload.dylib libhot_reload.dylib

run_debug:
	cargo run --features "hot_reload"

compile_run_debug:
	make compile_debug
	make run_debug

# release
compile_release:
	cargo build --target-dir target --features "hot_reload" --release
	mv target/release/libhot_reload.dylib libhot_reload.dylib

run_release:
	cargo run --features "hot_reload" --release

compile_run_release:
	make compile_release
	make run_release

# final

release:
	cargo run --release

# install_name_tool -id '' temp.dylib
