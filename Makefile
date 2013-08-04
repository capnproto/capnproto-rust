
COMPILATION_MARKER=capnprust/compilation-marker

.PHONY : capnprust clean all

all : samples/addressbook

clean :
	rm -rf capnprust/libcapnprust* $(COMPILATION_MARKER) compiler/capnpc-rust

capnprust : $(COMPILATION_MARKER)

$(COMPILATION_MARKER) :
	rustc capnprust/capnprust.rs
	touch $(COMPILATION_MARKER)

compiler/capnpc-rust : $(COMPILATION_MARKER) compiler/capnpc-rust.rs
	rustc -L./capnprust compiler/capnpc-rust.rs

samples/addressbook : compiler/capnpc-rust samples/addressbook.rs
	capnpc -o ./compiler/capnpc-rust:samples samples/addressbook.capnp
	rustc -L./capnprust samples/addressbook.rs
