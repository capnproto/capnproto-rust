extern crate capnpc;

fn main() {

    // See https://github.com/rust-lang/cargo/issues/879
    //::capnpc::compile(Path::new("."), [Path::new("test.capnp")]);

    let mut command = ::std::io::Command::new("capnp");
    command
        .arg("compile")
        .arg("-o/bin/cat")
        .arg("test.capnp");

    command.stdout(::std::io::process::CreatePipe(false, true));

    match command.spawn() {
        Ok(ref mut p) =>  {
            let mut child_stdout = p.stdout.take().unwrap();
            ::capnpc::codegen::main(&mut child_stdout).unwrap();
            p.wait().unwrap();
        }
        Err(e) => {
            panic!("could not start process: {}", e);
        }
    }

}
