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

release:
	cargo run --release

# install_name_tool -id '' temp.dylib
