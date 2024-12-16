# debug
compile_debug:
	cargo build --target-dir target --features "gbase/hot_reload"
	cp target/debug/libhot_reload.dylib temp.dylib
	install_name_tool -id '' temp.dylib
	mv temp.dylib libhot_reload.dylib

run_debug:
	cargo run --features "gbase/hot_reload"

compile_run_debug:
	make compile_debug
	make run_debug

# release

release:
	cargo run --release
