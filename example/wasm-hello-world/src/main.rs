use wasmer::{imports, Value};

pub mod wasm_hello_world_capnp {
    include!(concat!(env!("OUT_DIR"), "/wasm_hello_world_capnp.rs"));
}

static WASM: &'static [u8] =
    include_bytes!("../wasm-app/target/wasm32-unknown-unknown/release/wasm_app.wasm");

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let import_object = imports! {
        "env" => {},
    };
    let mut store = wasmer::Store::default();
    let module = wasmer::Module::new(&store, WASM)?;
    let instance = wasmer::Instance::new(&mut store, &module, &import_object)?;
    let memory = instance.exports.get_memory("memory")?;

    let mut expected_total: i32 = 0;
    let mut message = capnp::message::Builder::new_default();
    {
        let root: wasm_hello_world_capnp::foo::Builder = message.init_root();
        let mut numbers = root.init_numbers(10);
        let len = numbers.len();
        for ii in 0..len {
            numbers.set(ii, ii as i16);
            expected_total += ii as i32;
        }
    }

    // Don't pass the wasm app a slice that starts at zero, as that can cause undefined behavior.
    const START_BYTE: usize = 1;

    let segment_byte_size = {
        let output_segments = message.get_segments_for_output();
        assert_eq!(output_segments.len(), 1);
        let segment_size = output_segments[0].len();
        assert!(memory.view(&store).size().bytes().0 >= START_BYTE + segment_size);
        let view = memory.view(&store);
        view.write(START_BYTE as u64, output_segments[0])?;
        segment_size
    };

    let add_numbers = instance.exports.get_function("add_numbers")?;

    let result = add_numbers.call(
        &mut store,
        &[
            Value::I32(START_BYTE as i32),
            Value::I32(segment_byte_size as i32),
        ],
    )?;

    assert_eq!(result[0], Value::I32(expected_total));
    println!("success!");
    Ok(())
}
