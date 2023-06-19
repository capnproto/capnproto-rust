//! Download and/or build official Cap-n-Proto compiler (capnp) release for the current OS and architecture

use proc_macro::TokenStream;
use proc_macro2::TokenStream as TokenStream2;
use quote::{format_ident, quote};
use std::str::FromStr;
use std::{env, fs, path::Path};
use syn::parse::Parser;
use syn::{LitStr, Token};
use tempfile::tempdir;
use walkdir::WalkDir;
use wax::{BuildError, Walk};

include!(concat!(env!("OUT_DIR"), "/binary_decision.rs"));

#[proc_macro]
pub fn capnp_import(input: TokenStream) -> TokenStream {
    let parser = syn::punctuated::Punctuated::<LitStr, Token![,]>::parse_separated_nonempty;
    let path_patterns = parser.parse(input).unwrap();
    let path_patterns = path_patterns.into_iter().map(|item| item.value());
    let result = process_inner(path_patterns).unwrap();
    result.into()
}

fn process_inner<I>(path_patterns: I) -> anyhow::Result<TokenStream2>
where
    I: IntoIterator,
    I::Item: AsRef<str>,
{
    let cmdpath = CAPNP_BIN_PATH;
    let mut helperfile = TokenStream2::new();
    let output_dir = tempdir()?;

    let mut cmd = capnpc::CompilerCommand::new();
    cmd.capnp_executable(cmdpath);
    cmd.output_path(&output_dir);
    let globs = path_patterns
        .into_iter()
        .flat_map(|s| {
            wax::walk(s.as_ref(), ".")
                .map_err(BuildError::into_owned)
                .map(Walk::into_owned)
        })
        .flatten();
    for entry_result in globs {
        let entry = entry_result?.into_path();
        println!("Processing path: {:?}", entry.to_str());
        if entry.is_file() {
            println!("Processing {:?}", entry);
            cmd.file(entry);
        }
    }
    cmd.run()?;
    for entry in WalkDir::new(output_dir.path()) {
        let file_path = entry.unwrap().into_path();
        if file_path.is_file() {
            println!("File created: {:?}", file_path);
            helperfile.extend(append_path(&file_path)?);
        }
    }

    return Ok(helperfile);
}

fn append_path(file_path: &Path) -> anyhow::Result<TokenStream2> {
    let file_stem = file_path.file_stem().unwrap().to_str().unwrap(); //TODO unwraps due to Options - convert to Result instead
    let contents = TokenStream2::from_str(&fs::read_to_string(&file_path)?).unwrap(); //TODO This one unwrap due to not being thread-safe and something about Rc
    let module_name = format_ident!("{}", file_stem);
    let helperfile = quote! {
        mod #module_name {
            #contents
        }
    };
    Ok(helperfile)
}

#[test]
fn basic_file_test() -> anyhow::Result<()> {
    //println!("{:?}", std::env::current_dir().unwrap());
    let contents = process_inner(["tests/example.capnp"])?.to_string();
    assert!(contents.starts_with("mod example_capnp {"));
    assert!(contents.ends_with("}"));
    Ok(())
}

#[test]
fn glob_test() -> anyhow::Result<()> {
    // TODO This test produces two copies of modules named the same, which is invalid
    //println!("{:?}", std::env::current_dir().unwrap());
    let contents = process_inner(["tests/**/*.capnp"])?.to_string();
    assert!(contents.starts_with("mod example_capnp {"));
    assert!(contents.ends_with("}"));
    Ok(())
}
