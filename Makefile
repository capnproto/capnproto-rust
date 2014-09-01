RUSTC = rustc -O

CAPNP_INCLUDE_DIR=/usr/local/include

SCHEMA_SOURCES= $(CAPNP_INCLUDE_DIR)/capnp/rpc.capnp $(CAPNP_INCLUDE_DIR)/capnp/rpc-twoparty.capnp

src/rpc_capnp.rs : $(SCHEMA_SOURCES)
	capnp compile -orust:$(OUT_DIR) --src-prefix=$(CAPNP_INCLUDE_DIR)/capnp \
      $(CAPNP_INCLUDE_DIR)/capnp/rpc.capnp $(CAPNP_INCLUDE_DIR)/capnp/rpc-twoparty.capnp
	cp include_generated.rs $(OUT_DIR)
	echo $(PWD) > ~/Desktop/pwd.txt
	rustc -L./target/deps $(OUT_DIR)/include_generated.rs --out-dir $(OUT_DIR)


examples/calculator/calculator :  examples/calculator/main.rs \
	                             examples/calculator/client.rs examples/calculator/server.rs \
                                 examples/calculator/calculator.capnp
	capnp compile -orust examples/calculator/calculator.capnp
	$(RUSTC) -Ltarget -Ltarget/deps examples/calculator/main.rs --out-dir examples/calculator
