
COMPILATION_MARKER=capnprust/compilation-marker

.PHONY : capnprust clean all

all : samples

clean :
	rm -rf capnprust/libcapnprust* $(COMPILATION_MARKER) compiler/capnpc-rust

capnprust : $(COMPILATION_MARKER)

$(COMPILATION_MARKER) :
	rustc capnprust/capnprust.rs
	touch $(COMPILATION_MARKER)

compiler/capnpc-rust : $(COMPILATION_MARKER)
	rustc -L./capnprust compiler/capnpc-rust.rs

samples : compiler/capnpc-rust
	capnpc -o ./compiler/capnpc-rust:samples samples/addressbook.capnp
	rustc -L./capnprust samples/addressbook.rs
