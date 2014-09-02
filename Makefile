RUSTC = rustc -O

CAPNPC_DIR=$(shell dirname $(shell which capnpc-c++))

.PHONY : capnp clean all capnpc-rust-test check benchmark install

all : examples/addressbook/addressbook

clean :
	cargo clean
	rm -rf benchmark/*_capnp.rs benchmark/benchmark

target/capnpc-rust :
	cargo build

examples/addressbook/addressbook : target/capnpc-rust examples/addressbook/addressbook.rs
	capnpc -o ./target/capnpc-rust examples/addressbook/addressbook.capnp
	$(RUSTC) -Ltarget examples/addressbook/addressbook.rs --out-dir examples/addressbook


capnpc-rust-test :
	capnpc -o./target/capnpc-rust compiler_tests/test.capnp
	$(RUSTC) --test -Ltarget compiler_tests/test.rs --out-dir compiler_tests
	./compiler_tests/test

install : target/capnpc-rust
	cp target/capnpc-rust $(CAPNPC_DIR)

benchmark :
	capnpc -o ./target/capnpc-rust benchmark/carsales.capnp benchmark/catrank.capnp benchmark/eval.capnp
	$(RUSTC) -Ltarget benchmark/benchmark.rs --out-dir benchmark

