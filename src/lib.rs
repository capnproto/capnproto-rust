// Copyright (c) 2013-2014 Sandstorm Development Group, Inc. and contributors
// Licensed under the MIT License:
//
// Permission is hereby granted, free of charge, to any person obtaining a copy
// of this software and associated documentation files (the "Software"), to deal
// in the Software without restriction, including without limitation the rights
// to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
// copies of the Software, and to permit persons to whom the Software is
// furnished to do so, subject to the following conditions:
//
// The above copyright notice and this permission notice shall be included in
// all copies or substantial portions of the Software.
//
// THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
// IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
// FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
// AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
// LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
// OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN
// THE SOFTWARE.

//! # Cap'n Proto Schema Compiler Plugin Library
//!
//! This library allows you to do Cap'n Proto code generation within a Cargo build.
//!
//! In your Cargo.toml:
//!
//! ```ignore
//!   [build-dependencies]
//!   capnpc = "0.1.5"
//! ```
//!
//! In your build.rs, call
//!
//! ```ignore
//! ::capnpc::compile(Path::new("schema"),
//!                   &[Path::new("schema/foo.capnp"),
//!                     Path::new("schema/bar.capnp")]);
//! ```
//!
//! This will be equivalent to executing the shell command
//!
//! ```ignore
//!   capnp compile -orust:$OUT_DIR --src-prefix=schema schema/foo.capnp schema/bar.capnp
//! ```
//!

#![crate_name="capnpc"]
#![crate_type = "lib"]
#![feature(box_syntax, core)]

extern crate capnp;

pub mod schema_capnp;
pub mod codegen;
pub mod schema;

pub fn compile(prefix : &::std::path::Path, files : &[&::std::path::Path]) -> ::std::io::Result<()> {

    let out_dir = ::std::path::PathBuf::new(::std::env::var("OUT_DIR").unwrap().as_slice());
    let cwd = ::std::env::current_dir().unwrap();
    ::std::env::set_current_dir(&out_dir).unwrap();

    // ::std::Path does not normalize "foo/." to "foo/", and the schema compiler does not recognize
    // "foo/." as a prefix of "foo/bar.capnp". So we handle this case specially.
    let src_prefix = if prefix == ::std::path::Path::new(".") {
        cwd.clone()
    } else {
        cwd.clone().join(&prefix)
    };

    let mut command = ::std::process::Command::new("capnp");
    command
        .arg("compile")
        .arg("-o/bin/cat")
        .arg(format!("--src-prefix={}", src_prefix.display()).as_slice());

    for file in files.iter() {
        command.arg(format!("{}", cwd.join(file).display()).as_slice());
    }

    command.stdout(::std::process::Stdio::piped());
    command.stderr(::std::process::Stdio::inherit());

    let mut p =  try!(command.spawn());
    let child_stdout = ::capnp::io::ReadInputStream::new(p.stdout.take().unwrap());
    try!(::codegen::main(child_stdout));
    try!(p.wait());
    return Ok(());
}

