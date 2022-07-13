### v0.14.9
- Fix Clippy warnings in generated code.

### v0.14.8
- Include name of method in `unimplemented` error messages.
- Fix super interface lookup in case of transitive `extends` chain.

### v0.14.7
- Canonicalize order of type parameters from the bugfix for issue 260.

### v0.14.6
- Fix bug in code generation for unions that don't use all of their enclosing struct's generic params.

### v0.14.5
- Fix bug in code generation for generic groups.
- Add CompilerCommand.raw_code_generator_request_path().

### v0.14.4
- Check that schema files exist before attempting to invoke the schema compiler.

### v0.14.3
- Include LICENSE in published crate.

### v0.14.2
- Add CompilerCommand::default_parent_module() option.
- Add codegen::CodeGenerationCommand and deprecate codegen::generate_code().

### v0.14.1
- Get generated code to pass the elided_lifetimes_in_paths lint.

### v0.14.0
- Update for `SetPointerBuilder` no longer having a `To` type parameter.
- Make generated `Owned` structs unconstructable. They are only intented to be used as types, not values.

### v0.13.1
- Fix some more clippy warnings in generated code.

### v0.13.0
- Update to work without requiring "std" feature of capnp base crate.
- Refer to `core` instead of `std` in generated code.
- Remove deprecated `ToClient` structs from generated code.

### v0.12.4
- Add `CompilerCommand.capnp_executable()`.
- Remove obsolete `RustEdition` enum.

### v0.12.3
- Generate code for new capnp::capability::FromServer trait.

### v0.12.2
- Add `parentModule` annotation to allow generated code to be included in a submodule.

### v0.12.1
- Add rust.capnp file with `name` annotation for renaming items in generated Rust code.

### v0.12.0
- Remove deprecated item.

### v0.11.1
- Avoid generating some superfluous parenthesis.

### v0.11.0
- Remove unused experimental `schema` module.
- Bump minimum supported rustc version to 1.39.0.

### v0.10.2
- Include the string "@generated" generated code.
- Don't write output files if their content is unchanged.

### v0.10.1
- Allow CompilerCommand to work even when OUT_DIR is not set.

### v0.10.0
- Simplify handling of pointer defaults.
- Use new const fn ::capnp::word() instead of capnp_word!() macro.
- Remove deprecated items.
- Use dyn keyword for trait objects.
- Deprecate edition() configuration. Apparently the code we generate for Rust 2018 also works for Rust 2015 now.
- Update to 2018 edition.
- Update minimum required rustc version to 1.35.

### v0.9.5
- Fix bug in code generation for generic interfaces.

### v0.9.4
- Add support for pointer field defaults.

### v0.9.3
- Generate impls of new IntoInternalStructReader trait, to support set_with_caveats.
- Update deprecation attributes, to satisfy clippy.

### v0.9.2
- Rename a bunch of as_reader() methods to into_reader(), to satisfy clippy.

### v0.9.1
- Add support for Rust 2018.
- Fix a bunch of clippy warnings.

### v0.9.0
- Remove deprecated items.

### v0.8.9
- Deprecate borrow() in favor of reborrow().

### v0.8.8
- Support List(Interface).

### v0.8.7
- Eliminate `use` statements in generated code to avoid naming conflicts.

### v0.8.6
- Improve error message for snake_case method names.
- Eliminate floating point literals in match statements.

### v0.8.5
- Implement enum defaults.
- Emit "UNIMPLEMENTED" warnings on struct and list defaults.

### v0.8.4
- Implement struct, list, and enum constants.

### v0.8.3
- Fix bug where schemas with non-trivial relative filesystem paths could fail to compile.

### v0.8.2
- Fix bug where `initn_*()` methods of generic unions failed to set the discriminant.

### v0.8.1
- Fix several formatting issues in generated code.
- Remove some unneccesary trait bounds in generated code.
- Add `import_path()` and `no_std_import()` options to `CompilerCommand`.

### v0.8.0
- Remove deprecated `compile()` and `compile_with_src_prefixes()` functions.

### v0.7.5
- Fix bug that prevented compilation of interfaces with generic superclasses.
- More robust error handling.

### v0.7.4
- Deprecate `compile()` and `compile_with_src_prefixes()` in favor of `CompilerCommand`.

### v0.7.3
- `capnpc -orust ./foo/bar/baz.capnp` now correctly writes to `./foo/bar/baz_capnp.rs` rather than
  just `./baz_capnp.rs`. If you were depending on the old behavior you can use the `--src-prefix`
  flag for finer-grained control of the output location.

### v0.7.2
- Nicer formatting for floating point literals.

### v0.7.1
- Fix bug that prevented pipelining on an AnyPointer field.
