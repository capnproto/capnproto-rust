RUSTC = rustc -O -Z debug-info

CAPNPRUST_SOURCES=capnp/arena.rs capnp/common.rs capnp/endian.rs \
	capnp/layout.rs capnp/list.rs capnp/mask.rs capnp/message.rs \
	capnp/serialize.rs capnp/serialize_packed.rs capnp/blob.rs \
	capnp/io.rs capnp/any.rs capnp/pointer_helpers.rs

COMPILATION_MARKER=capnp/compilation-marker

.PHONY : capnprust clean all capnp-test capnpc-rust-test check benchmark

all : examples/addressbook/addressbook

clean :
	rm -rf capnp/libcapnp* $(COMPILATION_MARKER) capnpc-rust/capnpc-rust
	rm -rf benchmark/*_capnp.rs benchmark/benchmark

capnprust : $(COMPILATION_MARKER)

$(COMPILATION_MARKER) : $(CAPNPRUST_SOURCES)
	$(RUSTC) capnp/lib.rs
	touch $(COMPILATION_MARKER)

capnpc-rust/capnpc-rust : $(COMPILATION_MARKER) capnpc-rust/main.rs capnpc-rust/schema_capnp.rs
	$(RUSTC) -L./capnp capnpc-rust/main.rs

examples/addressbook/addressbook : capnpc-rust/capnpc-rust examples/addressbook/addressbook.rs
	capnpc -o ./capnpc-rust/capnpc-rust examples/addressbook/addressbook.capnp
	$(RUSTC) -L./capnp examples/addressbook/addressbook.rs

capnp-test :
	$(RUSTC) --test capnp/lib.rs
	./capnp/capnp

capnpc-rust-test : capnpc-rust/capnpc-rust
	capnpc -o ./capnpc-rust/capnpc-rust capnpc-rust/test.capnp
	$(RUSTC) --test -L./capnp capnpc-rust/test.rs
	./capnpc-rust/test

check : capnp-test capnpc-rust-test

benchmark : capnpc-rust/capnpc-rust
	capnpc -o ./capnpc-rust/capnpc-rust benchmark/carsales.capnp benchmark/catrank.capnp benchmark/eval.capnp
	$(RUSTC) -L./capnp benchmark/benchmark.rs
