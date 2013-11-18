RUSTC = rustc -O -Z debug-info

CAPNPRUST_SOURCES=capnp/arena.rs capnp/common.rs capnp/endian.rs \
	capnp/layout.rs capnp/list.rs capnp/mask.rs capnp/message.rs \
	capnp/serialize.rs capnp/serialize_packed.rs capnp/blob.rs \
	capnp/io.rs

COMPILATION_MARKER=capnp/compilation-marker

.PHONY : capnprust clean all check benchmark

all : samples/addressbook

clean :
	rm -rf capnp/libcapnp* $(COMPILATION_MARKER) compiler/capnpc-rust
	rm -rf benchmark/*_capnp.rs benchmark/benchmark

capnprust : $(COMPILATION_MARKER)

$(COMPILATION_MARKER) : $(CAPNPRUST_SOURCES)
	$(RUSTC) capnp/lib.rs
	touch $(COMPILATION_MARKER)

compiler/capnpc-rust : $(COMPILATION_MARKER) compiler/capnpc-rust.rs compiler/schema_capnp.rs compiler/macros.rs
	$(RUSTC) -L./capnp compiler/capnpc-rust.rs

samples/addressbook : compiler/capnpc-rust samples/addressbook.rs
	capnpc -o ./compiler/capnpc-rust samples/addressbook.capnp
	$(RUSTC) -L./capnp samples/addressbook.rs

check : compiler/capnpc-rust
	capnpc -o ./compiler/capnpc-rust compiler/test.capnp
	$(RUSTC) --test -L./capnp compiler/test.rs

benchmark : compiler/capnpc-rust
	capnpc -o ./compiler/capnpc-rust benchmark/carsales.capnp benchmark/catrank.capnp benchmark/eval.capnp
	$(RUSTC) -L./capnp benchmark/benchmark.rs
