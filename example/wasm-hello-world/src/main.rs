use wasmer_runtime::{imports, Value};

pub mod wasm_hello_world_capnp {
  include!(concat!(env!("OUT_DIR"), "/wasm_hello_world_capnp.rs"));
}

static WASM: &'static [u8] =
    include_bytes!("../wasm-app/target/wasm32-unknown-unknown/release/wasm_app.wasm");

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let import_object = imports! {
        "env" => {},
    };
    let mut instance = wasmer_runtime::instantiate(WASM, &import_object)?;
    let memory = instance.context_mut().memory(0);

    let mut expected_total: i32 = 0;
    let mut message = capnp::message::Builder::new_default();
    {
        let root: wasm_hello_world_capnp::foo::Builder = message.init_root();
        let mut numbers = root.init_numbers(10);
        let len = numbers.len();
        for ii in 0 .. len {
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
        assert!(memory.size().bytes().0 >= START_BYTE + segment_size);
        for (&byte, cell) in output_segments[0]
            .iter()
            .zip(memory.view()[START_BYTE .. START_BYTE + segment_size].iter())
        {
            cell.set(byte);
        }
        segment_size
    };


    let result = instance.call(
        "add_numbers",
        &[Value::I32(START_BYTE as i32), Value::I32(segment_byte_size as i32)])?;

    assert_eq!(result[0], Value::I32(expected_total));
    println!("success!");
    Ok(())
}
