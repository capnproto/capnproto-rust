RUSTC = rustc -O

CAPNP_INCLUDE_DIR=/usr/local/include


.PHONY : capnp clean all capnp-test capnpc-rust-test check benchmark install

all : examples/addressbook/addressbook

clean :
	rm -rf capnp/libcapnp* $(CAPNP_COMPILATION_MARKER) capnpc-rust/capnpc-rust
	rm -rf benchmark/*_capnp.rs benchmark/benchmark

capnp : $(CAPNP_COMPILATION_MARKER)

examples/addressbook/addressbook : capnpc-rust/capnpc-rust examples/addressbook/addressbook.rs
	capnpc -o ./capnpc-rust/capnpc-rust examples/addressbook/addressbook.capnp
	$(RUSTC) -L. examples/addressbook/addressbook.rs --out-dir examples/addressbook


capnpc-rust-test :
	capnpc -o./target/capnpc-rust compiler_tests/test.capnp
	$(RUSTC) --test -Ltarget compiler_tests/test.rs --out-dir compiler_tests
	./compiler_tests/test

install : capnpc-rust/capnpc-rust
	cp capnpc-rust/capnpc-rust /usr/local/bin

benchmark :
	capnpc -o ./target/capnpc-rust benchmark/carsales.capnp benchmark/catrank.capnp benchmark/eval.capnp
	$(RUSTC) -Ltarget benchmark/benchmark.rs --out-dir benchmark

