CAPNP_INCLUDE_DIR=/usr/local/include

SCHEMA_SOURCES= $(CAPNP_INCLUDE_DIR)/capnp/rpc.capnp $(CAPNP_INCLUDE_DIR)/capnp/rpc-twoparty.capnp

src/rpc_capnp.rs : $(SCHEMA_SOURCES)
	capnp compile -orust:src --src-prefix=$(CAPNP_INCLUDE_DIR)/capnp \
      $(CAPNP_INCLUDE_DIR)/capnp/rpc.capnp $(CAPNP_INCLUDE_DIR)/capnp/rpc-twoparty.capnp
