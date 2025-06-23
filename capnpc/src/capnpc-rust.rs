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

//! # Cap'n Proto Schema Compiler Plugin Executable
//!
//! [See this.](https://capnproto.org/otherlang.html#how-to-write-compiler-plugins)
//!
//!

pub fn main() {
    //! Generates Rust code according to a `schema_capnp::code_generator_request` read from stdin.

    let mut cmd = ::capnpc::codegen::CodeGenerationCommand::new();
    cmd.output_directory(::std::path::Path::new("."));

    if let Ok(parent_module) = std::env::var("CAPNPC_RUST_DEFAULT_PARENT_MODULE") {
        let modules = parent_module.split("::").map(ToString::to_string).collect();
        cmd.default_parent_module(modules);
    }

    cmd.run(::std::io::stdin())
        .expect("failed to generate code");
}
