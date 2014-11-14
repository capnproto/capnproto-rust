RUSTC = rustc -O

examples/calculator/calculator :  examples/calculator/main.rs \
	                             examples/calculator/client.rs examples/calculator/server.rs \
                                 examples/calculator/calculator.capnp
	capnp compile -orust examples/calculator/calculator.capnp
	$(RUSTC) -Ltarget -Ltarget/deps -Ltarget/native/$(shell ls target/native) examples/calculator/main.rs --out-dir examples/calculator
