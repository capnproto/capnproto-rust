RUSTC = rustc -O -Z debug-info

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
    capnp/pointer_helpers.rs \
    capnp/serialize.rs \
    capnp/serialize_packed.rs

CAPNP_RPC_SOURCES= \
	capnp-rpc/ez_rpc.rs \
    capnp-rpc/lib.rs \
    capnp-rpc/rpc.rs

CAPNP_COMPILATION_MARKER=capnp/compilation-marker
CAPNP_RPC_COMPILATION_MARKER=capnp-rpc/compilation-marker

.PHONY : capnp capnp-rpc clean all capnp-test capnpc-rust-test check benchmark

all : examples/addressbook/addressbook

clean :
	rm -rf capnp/libcapnp* $(CAPNP_COMPILATION_MARKER) capnpc-rust/capnpc-rust
	rm -rf benchmark/*_capnp.rs benchmark/benchmark
	rm -rf capnp-rpc/libcapnp* $(CAPNP_RPC_COMPILATION_MARKER)

capnp : $(CAPNP_COMPILATION_MARKER)

$(CAPNP_COMPILATION_MARKER) : $(CAPNP_SOURCES)
	$(RUSTC) capnp/lib.rs
	touch $(CAPNP_COMPILATION_MARKER)

capnpc-rust/capnpc-rust : $(CAPNP_COMPILATION_MARKER) capnpc-rust/main.rs capnpc-rust/schema_capnp.rs
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


capnp-rpc : $(CAPNP_RPC_COMPILATION_MARKER)

$(CAPNP_RPC_COMPILATION_MARKER) : capnpc-rust/capnpc-rust $(CAPNP_RPC_SOURCES)
	capnp compile -o./capnpc-rust/capnpc-rust capnp-rpc/rpc.capnp capnp-rpc/rpc-twoparty.capnp
	$(RUSTC) -L./capnp capnp-rpc/lib.rs
	touch $(CAPNP_RPC_COMPILATION_MARKER)

examples/calculator/calculator : capnpc-rust/capnpc-rust $(CAPNP_RPC_COMPILATION_MARKER) \
                                 examples/calculator/main.rs examples/calculator/client.rs \
                                 examples/calculator/calculator.capnp
	capnp compile -o./capnpc-rust/capnpc-rust examples/calculator/calculator.capnp
	$(RUSTC) -L./capnp -L./capnp-rpc examples/calculator/main.rs

