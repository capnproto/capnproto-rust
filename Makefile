RUSTC = rustc -O -Z debug-info

CAPNPRUST_SOURCES=capnprust/arena.rs capnprust/common.rs capnprust/endian.rs \
	capnprust/layout.rs capnprust/list.rs capnprust/mask.rs capnprust/message.rs \
	capnprust/serialize.rs capnprust/serialize_packed.rs capnprust/blob.rs

COMPILATION_MARKER=capnprust/compilation-marker

.PHONY : capnprust clean all check benchmark

all : samples/addressbook

clean :
	rm -rf capnprust/libcapnprust* $(COMPILATION_MARKER) compiler/capnpc-rust

capnprust : $(COMPILATION_MARKER)

$(COMPILATION_MARKER) : $(CAPNPRUST_SOURCES)
	$(RUSTC) capnprust/capnprust.rs
	touch $(COMPILATION_MARKER)

compiler/capnpc-rust : $(COMPILATION_MARKER) compiler/capnpc-rust.rs compiler/schema_capnp.rs
	$(RUSTC) -L./capnprust compiler/capnpc-rust.rs

samples/addressbook : compiler/capnpc-rust samples/addressbook.rs
	capnpc -o ./compiler/capnpc-rust samples/addressbook.capnp
	$(RUSTC) -L./capnprust samples/addressbook.rs

check : compiler/capnpc-rust
	capnpc -o ./compiler/capnpc-rust compiler/test.capnp
	$(RUSTC) --test -L./capnprust compiler/test.rs

benchmark : compiler/capnpc-rust
	capnpc -o ./compiler/capnpc-rust benchmark/carsales.capnp benchmark/catrank.capnp
	$(RUSTC) -L./capnprust benchmark/benchmark.rs
