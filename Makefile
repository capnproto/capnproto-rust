RUSTC = rustc -O

CAPNP_INCLUDE_DIR=/usr/local/include

SCHEMA_SOURCES= $(CAPNP_INCLUDE_DIR)/capnp/rpc.capnp $(CAPNP_INCLUDE_DIR)/capnp/rpc-twoparty.capnp

src/rpc_capnp.rs : $(SCHEMA_SOURCES)
	capnp compile -orust:src --src-prefix=$(CAPNP_INCLUDE_DIR)/capnp \
      $(CAPNP_INCLUDE_DIR)/capnp/rpc.capnp $(CAPNP_INCLUDE_DIR)/capnp/rpc-twoparty.capnp


examples/calculator/calculator :  examples/calculator/main.rs \
	                             examples/calculator/client.rs examples/calculator/server.rs \
                                 examples/calculator/calculator.capnp
	capnp compile -orust examples/calculator/calculator.capnp
	$(RUSTC) -Ltarget -Ltarget/deps examples/calculator/main.rs --out-dir examples/calculator
