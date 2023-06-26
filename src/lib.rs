//! Download and/or build official Cap-n-Proto compiler (capnp) release for the current OS and architecture

use anyhow::anyhow;
use convert_case::{Case, Casing};
use proc_macro::TokenStream;
use proc_macro2::TokenStream as TokenStream2;
use quote::{format_ident, quote};
use std::path::PathBuf;
use std::str::FromStr;
use std::{env, fs, path::Path};
use syn::parse::Parser;
use syn::{LitStr, Token};
use walkdir::WalkDir;
use wax::{BuildError, Walk};

use anyhow::Context;

include!(concat!(env!("OUT_DIR"), "/extract_bin.rs"));

/// `capnp_import!(pattern_1, pattern_2, ..., pattern_n)` compiles all the .capnp files at the locations of those files
/// and replaces itself with the resulting contents wrapped in appropriate module structure.
/// Resulting rust files from that compilation are then deleted.
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
    let output_dir = commandhandle().context("could not create temporary capnp binary")?;
    let cmdpath = output_dir.path().join("capnp");

    let globs = path_patterns
        .into_iter()
        .flat_map(|s| {
            wax::walk(s.as_ref(), ".")
                .map_err(BuildError::into_owned)
                .map(Walk::into_owned)
        })
        .flatten();

    for entry_result in globs {
        let entry: PathBuf = entry_result?.into_path();
        //dbg!(&entry);
        if entry.is_file() {
            compile_capnp_file(&entry, &output_dir, &cmdpath)?;
        }
    }
    let helperfile = construct_module_tree(output_dir.path(), true)?;
    Ok(helperfile)
    // When TempDir goes out of scope, it gets deleted
}

/// Takes a path of a .capnp file, compiles that file and returns the result to the `output_dir`.
/// For example: `compile_capnp_file(Path::new("a/b/c.capnp"), Path::new("/d"))` would output `/d/a/b/c_capnp.rs` file,
/// with `["a".into(), "b".into()]` being applied to `default_parent_module`, which makes contents of the file accessible with `a::b::c_capnp` module path.
fn compile_capnp_file<P>(file_path: &Path, output_dir: P, cmdpath: &Path) -> anyhow::Result<()>
where
    P: AsRef<Path>,
{
    //assert!(file_path.is_file());
    let mut cmd = capnpc::CompilerCommand::new();
    cmd.capnp_executable(cmdpath);
    cmd.output_path(output_dir);
    let module_path = file_path
        .components()
        .skip(1)
        .map(|component| component.as_os_str().to_str().unwrap().to_case(Case::Snake))
        .take_while(|name| !name.ends_with(".capnp"))
        .collect::<Vec<String>>();
    //dbg!(&module_path);
    cmd.default_parent_module(module_path);
    cmd.file(file_path);
    cmd.run()?;

    Ok(())
}

/// Recursively goes through a directory tree and contents of rust files in it and wraps them in modules based on files they're in and their location.
/// If skip_root is true it doesn't construct a top-level module (useful when it's something like .tmp-foobar)
fn construct_module_tree(root: &Path, skip_root: bool) -> anyhow::Result<TokenStream2> {
    //dbg!(root);
    if root.is_file() {
        // Read file contents and return as module
        let contents = TokenStream2::from_str(&fs::read_to_string(root)?)
            .map_err(|_| anyhow!("Couldn't get file contents as TokenStream"))?;
        let module_name = format_ident!(
            "{}",
            root.file_stem()
                .ok_or(anyhow!("Module name can't be empty"))?
                .to_str()
                .ok_or(anyhow!("Module name must be valid UTF-8"))?
                .to_case(Case::Snake)
        );
        let res = quote! {
            pub mod #module_name {
                #contents
            }
        };
        //dbg!(res.to_string());
        Ok(res)
    } else if root.is_dir() {
        let mut contents = TokenStream2::new();
        for element in WalkDir::new(root).min_depth(1).max_depth(1) {
            // Skip our temporary capnp executable
            let element = element?;
            if element.file_name() != "capnp" {
                contents.extend(construct_module_tree(element.path(), false)?);
            }
        }
        if skip_root {
            //dbg!(contents.to_string());
            return Ok(contents);
        }
        let module_name = root
            .components()
            .last()
            .ok_or(anyhow!("Module name can't be empty"))?
            .as_os_str()
            .to_str()
            .ok_or(anyhow!("Module name must be valid UTF-8"))?
            .to_case(Case::Snake);
        let module_name = format_ident!("{}", module_name);
        let res = quote! {
            pub mod #module_name {
                #contents
            }
        };
        //dbg!(res.to_string());
        return Ok(res);
    } else {
        Ok(TokenStream2::new())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    #[test]
    fn basic_file_test() -> anyhow::Result<()> {
        //println!("{:?}", std::env::current_dir().unwrap());
        let contents = process_inner(["tests/example.capnp"])?.to_string();
        assert!(contents.starts_with("pub mod tests { pub mod example_capnp {"));
        assert!(contents.ends_with("}"));
        Ok(())
    }

    #[test]
    fn glob_test() -> anyhow::Result<()> {
        let contents = process_inner(["tests/**/*.capnp"])?;
        // We expect a following structure (in some order):
        // pub mod tests {
        //     pub mod example_capnp {
        //         ..
        //     }
        //     pub mod folder_test {
        //         ..
        //     }
        // }
        let tests_module: syn::ItemMod = syn::parse2(contents)?;
        assert_eq!(tests_module.ident, "tests");
        let submodule_idents: HashSet<String> = tests_module
            .content
            .ok_or(anyhow!("tests module shouldn't be empty"))?
            .1
            .into_iter()
            .map(|submodule| match submodule {
                syn::Item::Mod(module) => module.ident.to_string(),
                _ => panic!("tests module only has submodules"),
            })
            .collect();
        assert_eq!(
            submodule_idents,
            HashSet::from(["example_capnp".to_string(), "folder_test".to_string()])
        );
        Ok(())
    }
}
