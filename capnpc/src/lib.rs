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
//! This library allows you to do
//! [Cap'n Proto code generation](https://capnproto.org/otherlang.html#how-to-write-compiler-plugins)
//! within a Cargo build. You still need the `capnp` binary (implemented in C++).
//! (If you use a package manager, try looking for a package called
//! `capnproto`.)
//!
//! In your Cargo.toml:
//!
//! ```ignore
//! [package]
//! build = "build.rs"
//!
//! [build-dependencies]
//! capnpc = "0.14"
//! ```
//!
//! In your build.rs:
//!
//! ```ignore
//! fn main() {
//!     capnpc::CompilerCommand::new()
//!         .src_prefix("schema")
//!         .file("schema/foo.capnp")
//!         .file("schema/bar.capnp")
//!         .run().expect("schema compiler command");
//! }
//! ```
//!
//! This will be equivalent to executing the shell command
//!
//! ```ignore
//!   capnp compile -orust:$OUT_DIR --src-prefix=schema schema/foo.capnp schema/bar.capnp
//! ```
//!

/// Code generated from
/// [schema.capnp](https://github.com/capnproto/capnproto/blob/master/c%2B%2B/src/capnp/schema.capnp).
pub mod schema_capnp;

pub mod codegen;
pub mod codegen_types;
mod pointer_constants;

use std::path::{Path, PathBuf};

// Copied from capnp/src/lib.rs, where this conversion lives behind the "std" feature flag,
// which we don't want to depend on here.
pub(crate) fn convert_io_err(err: std::io::Error) -> capnp::Error {
    use std::io;
    let kind = match err.kind() {
        io::ErrorKind::TimedOut => capnp::ErrorKind::Overloaded,
        io::ErrorKind::BrokenPipe |
        io::ErrorKind::ConnectionRefused |
        io::ErrorKind::ConnectionReset |
        io::ErrorKind::ConnectionAborted |
        io::ErrorKind::NotConnected  => capnp::ErrorKind::Disconnected,
        _ => capnp::ErrorKind::Failed,
    };
    capnp::Error { description: format!("{}", err), kind: kind }
}

fn run_command(mut command: ::std::process::Command, mut code_generation_command: codegen::CodeGenerationCommand) -> ::capnp::Result<()> {
    let mut p = command.spawn().map_err(convert_io_err)?;
    code_generation_command.run(p.stdout.take().unwrap())?;
    let exit_status = p.wait().map_err(convert_io_err)?;
    if !exit_status.success() {
        Err(::capnp::Error::failed(format!(
            "Non-success exit status: {}",
            exit_status
        )))
    } else {
        Ok(())
    }
}

/// A builder object for schema compiler commands.
pub struct CompilerCommand {
    files: Vec<PathBuf>,
    src_prefixes: Vec<PathBuf>,
    import_paths: Vec<PathBuf>,
    no_standard_import: bool,
    executable_path: Option<PathBuf>,
    output_path: Option<PathBuf>,
    default_parent_module: Vec<String>,
    raw_code_generator_request_path: Option<PathBuf>,
}

impl CompilerCommand {
    /// Creates a new, empty command.
    pub fn new() -> CompilerCommand {
        CompilerCommand {
            files: Vec::new(),
            src_prefixes: Vec::new(),
            import_paths: Vec::new(),
            no_standard_import: false,
            executable_path: None,
            output_path: None,
            default_parent_module: Vec::new(),
            raw_code_generator_request_path: None,
        }
    }

    /// Adds a file to be compiled.
    pub fn file<P>(&mut self, path: P) -> &mut CompilerCommand
    where
        P: AsRef<Path>,
    {
        self.files.push(path.as_ref().to_path_buf());
        self
    }

    /// Adds a --src-prefix flag. For all files specified for compilation that start
    /// with `prefix`, removes the prefix when computing output filenames.
    pub fn src_prefix<P>(&mut self, prefix: P) -> &mut CompilerCommand
    where
        P: AsRef<Path>,
    {
        self.src_prefixes.push(prefix.as_ref().to_path_buf());
        self
    }

    /// Adds an --import_path flag. Adds `dir` to the list of directories searched
    /// for absolute imports.
    pub fn import_path<P>(&mut self, dir: P) -> &mut CompilerCommand
    where
        P: AsRef<Path>,
    {
        self.import_paths.push(dir.as_ref().to_path_buf());
        self
    }

    /// Adds the --no-standard-import flag, indicating that the default import paths of
    /// /usr/include and /usr/local/include should not bet included.
    pub fn no_standard_import(&mut self) -> &mut CompilerCommand {
        self.no_standard_import = true;
        self
    }

    /// Sets the output directory of generated code. Default is OUT_DIR
    pub fn output_path<P>(&mut self, path: P) -> &mut CompilerCommand
    where
        P: AsRef<Path>,
    {
        self.output_path = Some(path.as_ref().to_path_buf());
        self
    }

    /// Specify the executable which is used for the 'capnp' tool. When this method is not called, the command looks for a name 'capnp'
    /// on the system (e.g. in working directory or in PATH environment variable).
    pub fn capnp_executable<P>(&mut self, path: P) -> &mut CompilerCommand
    where
        P: AsRef<Path>
    {
        self.executable_path = Some(path.as_ref().to_path_buf());
        self
    }

    /// Sets the default parent module. This indicates the scope in your crate where you will
    /// add a module containing the generated code. For example, if you set this option to
    /// `vec!["foo".into(), "bar".into()]`, and you are generating code for `baz.capnp`, then your crate
    /// should have this structure:
    ///
    /// ```ignore
    /// pub mod foo {
    ///    pub mod bar {
    ///        pub mod baz_capnp {
    ///            include!(concat!(env!("OUT_DIR"), "/baz_capnp.rs"));
    ///        }
    ///    }
    /// }
    /// ```
    ///
    /// This option can be overridden by the `parentModule` annotation defined in `rust.capnp`.
    ///
    /// If this option is unset, the default is the crate root.
    pub fn default_parent_module(&mut self, default_parent_module: Vec<String>) -> &mut CompilerCommand
    {
        self.default_parent_module = default_parent_module;
        self
    }

    /// If set, the generator will also write a file containing the raw code generator request to the
    /// specified path.
    pub fn raw_code_generator_request_path<P>(&mut self, path: P) -> &mut CompilerCommand
    where
        P: AsRef<Path>,
    {
        self.raw_code_generator_request_path = Some(path.as_ref().to_path_buf());
        self
    }

    /// Runs the command.
    /// Returns an error if `OUT_DIR` or a custom output directory was not set, or if `capnp compile` fails.
    pub fn run(&mut self) -> ::capnp::Result<()> {
        let mut command = if let Some(executable) = &self.executable_path {
            ::std::process::Command::new(executable)
        } else {
            ::std::process::Command::new("capnp")
        };

        command.arg("compile").arg("-o").arg("-");

        if self.no_standard_import {
            command.arg("--no-standard-import");
        }

        for import_path in &self.import_paths {
            command.arg(&format!("--import-path={}", import_path.display()));
        }

        for src_prefix in &self.src_prefixes {
            command.arg(&format!("--src-prefix={}", src_prefix.display()));
        }

        for file in &self.files {
            std::fs::metadata(file).map_err(|error| {
                let current_dir = match std::env::current_dir() {
                    Ok(current_dir) => format!("`{}`", current_dir.display()),
                    Err(..) => "<unknown working directory>".to_string(),
                };

                ::capnp::Error::failed(format!(
                    "Unable to stat capnp input file `{}` in working directory {}: {}.  \
                     Please check that the file exists and is accessible for read.",
                    file.display(),
                    current_dir,
                    error
                ))
            })?;

            command.arg(file);
        }

        let output_path = if let Some(output_path) = &self.output_path {
            output_path.clone()
        } else {
            // Try `OUT_DIR` by default
            PathBuf::from(::std::env::var("OUT_DIR").map_err(|error| {
                ::capnp::Error::failed(format!(
                    "Could not access `OUT_DIR` environment variable: {}. \
                     You might need to set it up or instead create you own output \
                     structure using `CompilerCommand::output_path`",
                    error
                ))
            })?)
        };

        command.stdout(::std::process::Stdio::piped());
        command.stderr(::std::process::Stdio::inherit());

        let mut code_generation_command = crate::codegen::CodeGenerationCommand::new();
        code_generation_command
            .output_directory(output_path)
            .default_parent_module(self.default_parent_module.clone());
        if let Some(raw_code_generator_request_path) = &self.raw_code_generator_request_path {
            code_generation_command.raw_code_generator_request_path(raw_code_generator_request_path.clone());
        }

        run_command(command, code_generation_command).map_err(|error| {
            ::capnp::Error::failed(format!(
                "Error while trying to execute `capnp compile`: {}.  \
                 Please verify that version 0.5.2 or higher of the capnp executable \
                 is installed on your system. See https://capnproto.org/install.html",
                error
            ))
        })
    }
}

#[test]
fn compiler_command_new_no_out_dir() {
    let error = CompilerCommand::new().run().unwrap_err().description;
    assert!(error.starts_with("Could not access `OUT_DIR` environment variable"));
}

#[test]
fn compiler_command_with_output_path_no_out_dir() {
    let error = CompilerCommand::new().output_path("foo").run().unwrap_err().description;
    assert!(error.starts_with("Error while trying to execute `capnp compile`"));
}
