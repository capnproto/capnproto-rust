RUSTC = rustc -O -Z debug-info

CAPNPRUST_SOURCES=capnp/arena.rs capnp/common.rs capnp/endian.rs \
	capnp/layout.rs capnp/list.rs capnp/mask.rs capnp/message.rs \
	capnp/serialize.rs capnp/serialize_packed.rs capnp/blob.rs \
	capnp/io.rs capnp/any.rs capnp/pointer_helpers.rs

COMPILATION_MARKER=capnp/compilation-marker

.PHONY : capnprust clean all check benchmark

all : samples/addressbook

clean :
	rm -rf capnp/libcapnp* $(COMPILATION_MARKER) capnpc-rust/capnpc-rust
	rm -rf benchmark/*_capnp.rs benchmark/benchmark

capnprust : $(COMPILATION_MARKER)

$(COMPILATION_MARKER) : $(CAPNPRUST_SOURCES)
	$(RUSTC) capnp/lib.rs
	touch $(COMPILATION_MARKER)

capnpc-rust/capnpc-rust : $(COMPILATION_MARKER) capnpc-rust/main.rs capnpc-rust/schema_capnp.rs
	$(RUSTC) -L./capnp capnpc-rust/main.rs

samples/addressbook : capnpc-rust/capnpc-rust samples/addressbook.rs
	capnpc -o ./capnpc-rust/capnpc-rust samples/addressbook.capnp
	$(RUSTC) -L./capnp samples/addressbook.rs

check : capnpc-rust/capnpc-rust
	capnpc -o ./capnpc-rust/capnpc-rust capnpc-rust/test.capnp
	$(RUSTC) --test capnp/lib.rs
	$(RUSTC) --test -L./capnp capnpc-rust/test.rs
	./capnp/capnp
	./capnpc-rust/test

benchmark : capnpc-rust/capnpc-rust
	capnpc -o ./capnpc-rust/capnpc-rust benchmark/carsales.capnp benchmark/catrank.capnp benchmark/eval.capnp
	$(RUSTC) -L./capnp benchmark/benchmark.rs
