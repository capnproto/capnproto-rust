RUSTC = rustc -O

CAPNP_INCLUDE_DIR=/usr/local/include

CAPNP_SOURCES= \
    capnp/any.rs \
    capnp/arena.rs \
    capnp/blob.rs \
    capnp/capability.rs \
    capnp/common.rs \
    capnp/endian.rs \
    capnp/io.rs \
    capnp/layout.rs \
    capnp/lib.rs \
    capnp/list.rs \
    capnp/mask.rs \
    capnp/message.rs \
    capnp/serialize.rs \
    capnp/serialize_packed.rs

CAPNP_COMPILATION_MARKER=capnp/compilation-marker

.PHONY : capnp clean all capnp-test capnpc-rust-test check benchmark install

all : examples/addressbook/addressbook

clean :
	rm -rf capnp/libcapnp* $(CAPNP_COMPILATION_MARKER) capnpc-rust/capnpc-rust
	rm -rf benchmark/*_capnp.rs benchmark/benchmark

capnp : $(CAPNP_COMPILATION_MARKER)

$(CAPNP_COMPILATION_MARKER) : $(CAPNP_SOURCES)
	$(RUSTC) capnp/lib.rs
	touch $(CAPNP_COMPILATION_MARKER)

capnpc-rust/capnpc-rust : $(CAPNP_COMPILATION_MARKER) capnpc-rust/codegen.rs capnpc-rust/main.rs capnpc-rust/schema_capnp.rs
	$(RUSTC) -L. capnpc-rust/main.rs --out-dir capnpc-rust

examples/addressbook/addressbook : capnpc-rust/capnpc-rust examples/addressbook/addressbook.rs
	capnpc -o ./capnpc-rust/capnpc-rust examples/addressbook/addressbook.capnp
	$(RUSTC) -L. examples/addressbook/addressbook.rs --out-dir examples/addressbook

capnp-test :
	$(RUSTC) --test capnp/lib.rs --out-dir capnp
	./capnp/capnp

capnpc-rust-test : capnpc-rust/capnpc-rust
	capnpc -o ./capnpc-rust/capnpc-rust capnpc-rust/test.capnp
	$(RUSTC) --test -L. capnpc-rust/test.rs --out-dir capnpc-rust
	./capnpc-rust/test

check : capnp-test capnpc-rust-test

install : capnpc-rust/capnpc-rust
	cp capnpc-rust/capnpc-rust /usr/local/bin

benchmark : capnpc-rust/capnpc-rust
	capnpc -o ./capnpc-rust/capnpc-rust benchmark/carsales.capnp benchmark/catrank.capnp benchmark/eval.capnp
	$(RUSTC) -L. benchmark/benchmark.rs --out-dir benchmark

