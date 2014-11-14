/*
 * Copyright (c) 2013-2014, David Renshaw (dwrenshaw@gmail.com)
 *
 * See the LICENSE file in the capnproto-rust root directory.
 */

#![feature(globs)]

#![crate_name="capnpc"]
#![crate_type = "lib"]

extern crate capnp;

pub mod schema_capnp;
pub mod codegen;

pub fn compile(prefix : Path, files : &[Path]) {
    let out_dir = Path::new(::std::os::getenv("OUT_DIR").unwrap());
    let cwd = ::std::os::getcwd();
    ::std::os::change_dir(&out_dir);

    let mut command = ::std::io::Command::new("capnp");
    command
        .arg("compile")
        .arg("-o/bin/cat")
        .arg(format!("--src-prefix={}", prefix.display()));

    for file in files.iter() {
        command.arg(format!("{}", cwd.join(file).display()));
    }

    command.stdout(::std::io::process::CreatePipe(false, true));

    match command.spawn() {
        Ok(ref mut p) =>  {
            let mut child_stdout = p.stdout.take().unwrap();
            ::codegen::main(&mut child_stdout).unwrap();
            p.wait().unwrap();
        }
        Err(e) => {
            panic!("could not start process: {}", e);
        }
    }

}

