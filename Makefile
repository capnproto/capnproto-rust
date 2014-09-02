RUSTC = rustc -O

CAPNP_PATH=$(shell which capnp)
CAPNP_INCLUDE_DIR=$(shell dirname $(CAPNP_PATH))/../include

.PHONY : generated

generated : $(OUT_DIR)/rpc_capnp.rs

SCHEMA_SOURCES= $(CAPNP_INCLUDE_DIR)/capnp/rpc.capnp $(CAPNP_INCLUDE_DIR)/capnp/rpc-twoparty.capnp

$(OUT_DIR)/rpc_capnp.rs : $(SCHEMA_SOURCES)
	capnp compile -orust:$(OUT_DIR) --src-prefix=$(CAPNP_INCLUDE_DIR)/capnp \
      $(CAPNP_INCLUDE_DIR)/capnp/rpc.capnp $(CAPNP_INCLUDE_DIR)/capnp/rpc-twoparty.capnp
	cp capnp_rpc_include_generated.rs $(OUT_DIR)
	rustc -L$(OUT_DIR)/../../deps $(OUT_DIR)/capnp_rpc_include_generated.rs --out-dir $(OUT_DIR)


examples/calculator/calculator :  examples/calculator/main.rs \
	                             examples/calculator/client.rs examples/calculator/server.rs \
                                 examples/calculator/calculator.capnp
	capnp compile -orust examples/calculator/calculator.capnp
	$(RUSTC) -Ltarget -Ltarget/deps examples/calculator/main.rs --out-dir examples/calculator
