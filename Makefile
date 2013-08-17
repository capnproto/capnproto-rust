
CAPNPRUST_SOURCES=capnprust/arena.rs capnprust/common.rs capnprust/endian.rs \
	capnprust/layout.rs capnprust/list.rs capnprust/mask.rs capnprust/message.rs \
	capnprust/serialize.rs capnprust/serialize_packed.rs

COMPILATION_MARKER=capnprust/compilation-marker

.PHONY : capnprust clean all

all : samples/addressbook

clean :
	rm -rf capnprust/libcapnprust* $(COMPILATION_MARKER) compiler/capnpc-rust

capnprust : $(COMPILATION_MARKER)

$(COMPILATION_MARKER) : $(CAPNPRUST_SOURCES)
	rustc capnprust/capnprust.rs
	touch $(COMPILATION_MARKER)

compiler/capnpc-rust : $(COMPILATION_MARKER) compiler/capnpc-rust.rs compiler/schema_capnp.rs
	rustc -L./capnprust compiler/capnpc-rust.rs

samples/addressbook : compiler/capnpc-rust samples/addressbook.rs
	capnpc -o ./compiler/capnpc-rust:samples samples/addressbook.capnp
	rustc -L./capnprust samples/addressbook.rs
