compile:
	cargo build
	cp ../../target/debug/libhot_reload.dylib temp.dylib
	install_name_tool -id '' temp.dylib
	mv temp.dylib libhot_reload.dylib

run:
	cargo run

compile_run:
	make compile
	make run
