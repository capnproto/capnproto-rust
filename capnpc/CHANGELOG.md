## v0.25.0
- Use new GeneratedCodeArena to avoid need for `unsafe` in generated code.

## v0.24.1
- Add local `#[allow(unsafe_code)]` annotations to generated code.

## v0.24.0
- Adjust generated code to used new constructors for constants and RawStructSchema.

## v0.23.3
- Adjust generated dispatch code to properly disambiguate when an 'extends' interface method
  conflicts with another method.

## v0.23.2
- Remove `async` block in generated code for streaming methods.

## v0.23.1
- Use `f64::INFINITY` instead of `::core::f64::INFINITY` in generated code.

## v0.23.0
- Update generated code of `Server` trait methods to take `self` as `Rc<Self>`.

## v0.22.0
- Update generated code of `Server` traits to support `async fn` methods.
  What previously was a `&mut self` parameter is now `&self`. Therefore
  RPC objects must now add their own interior mutability as needed.
  `Cell` or `RefCell` should suffice in most cases.
- Remove support for rustc editions older than 2021 (when `await` was added).

## v0.21.4
- Update minimum required version of capnp base crate, to account for new
  usage of impl Clone for capnp:: capability::Client.

## v0.21.3
- Fix code generation for non-finite float point constants.

## v0.21.2
- Use new `capability::DynClientHook` alias to avoid warnings in generated code.

## v0.21.1
- Use new  `introspect::panic_invalid_field_index()` and
  `introspect::panic_invalid_annotation_indices()` functions so that
  generated code can pass Clippy while still working on Rust 2015.

## v0.21.0
- Follow v0.21.0 release of other capnp crates.

## v0.20.1
- Elide more lifetimes in generated code to make Clippy happy.

## v0.20.0
- Add support for `stream` keyword.

## v0.19.0
- Include new members_by_name field of RawStructSchema.
- Generalize text, primitive_list, and enum_list setters using impl SetterInput.

## v0.18.1
- Fix overly-restrictive lifetimes in setters of certain list fields.

## v0.18.0
- Update for lazier utf-8 validation.

## v0.17.2
- Add the `$Rust.option` annotation for declaring fields to be optional.
- Add `CompilerCommand::crate_provides()`, allowing cross-crate imports.

## v0.17.1
- Fix setters of enum fields with defaults.

## v0.17.0
- Add support for reflection.
- Implement `Debug` for all generated struct `Reader` types.

## v0.16.5
- Use `core::marker` instead of `std::marker` for pointer constants, for no_std compat.

## v0.16.4
- Generate explicit Clone and Copy impls for Reader structs.
- Fully-qualify `::capnp::Word` in generated code.
- Add `capnp --version` invocation before `capnp compile`, for better error reporting.
- Clear PWD env variable, to silence warning from kj/filesystem-disk-unix.c++.

## v0.16.3
- Generate `*_has()` methods for capability fields.

## v0.16.2
- Avoid ambiguous associated item in TryFrom implementations.

## v0.16.1
- Fix clippy warnings in generated code.

## v0.16.0
- Update code generation for removal of `To16`, `FromU16`, `FromStructReader`, `FromStructBuilder`.

## v0.15.2
- Apply clippy lifetime elision suggestion in set_pointer_builder() in generated code.

## v0.15.1
- Lots of style fixes and linting, including for generated code.

## v0.15.0
- Support trait changes in capnp::traits.
- Remove deprecated function.

## v0.14.9
- Fix Clippy warnings in generated code.

## v0.14.8
- Include name of method in `unimplemented` error messages.
- Fix super interface lookup in case of transitive `extends` chain.

## v0.14.7
- Canonicalize order of type parameters from the bugfix for issue 260.

## v0.14.6
- Fix bug in code generation for unions that don't use all of their enclosing struct's generic params.

## v0.14.5
- Fix bug in code generation for generic groups.
- Add CompilerCommand.raw_code_generator_request_path().

## v0.14.4
- Check that schema files exist before attempting to invoke the schema compiler.

## v0.14.3
- Include LICENSE in published crate.

## v0.14.2
- Add CompilerCommand::default_parent_module() option.
- Add codegen::CodeGenerationCommand and deprecate codegen::generate_code().

## v0.14.1
- Get generated code to pass the elided_lifetimes_in_paths lint.

## v0.14.0
- Update for `SetPointerBuilder` no longer having a `To` type parameter.
- Make generated `Owned` structs unconstructable. They are only intented to be used as types, not values.

## v0.13.1
- Fix some more clippy warnings in generated code.

## v0.13.0
- Update to work without requiring "std" feature of capnp base crate.
- Refer to `core` instead of `std` in generated code.
- Remove deprecated `ToClient` structs from generated code.

## v0.12.4
- Add `CompilerCommand.capnp_executable()`.
- Remove obsolete `RustEdition` enum.

## v0.12.3
- Generate code for new capnp::capability::FromServer trait.

## v0.12.2
- Add `parentModule` annotation to allow generated code to be included in a submodule.

## v0.12.1
- Add rust.capnp file with `name` annotation for renaming items in generated Rust code.

## v0.12.0
- Remove deprecated item.

## v0.11.1
- Avoid generating some superfluous parentheses.

## v0.11.0
- Remove unused experimental `schema` module.
- Bump minimum supported rustc version to 1.39.0.

## v0.10.2
- Include the string "@generated" generated code.
- Don't write output files if their content is unchanged.

## v0.10.1
- Allow CompilerCommand to work even when OUT_DIR is not set.

## v0.10.0
- Simplify handling of pointer defaults.
- Use new const fn ::capnp::word() instead of capnp_word!() macro.
- Remove deprecated items.
- Use dyn keyword for trait objects.
- Deprecate edition() configuration. Apparently the code we generate for Rust 2018 also works for Rust 2015 now.
- Update to 2018 edition.
- Update minimum required rustc version to 1.35.

## v0.9.5
- Fix bug in code generation for generic interfaces.

## v0.9.4
- Add support for pointer field defaults.

## v0.9.3
- Generate impls of new IntoInternalStructReader trait, to support set_with_caveats.
- Update deprecation attributes, to satisfy clippy.

## v0.9.2
- Rename a bunch of as_reader() methods to into_reader(), to satisfy clippy.

## v0.9.1
- Add support for Rust 2018.
- Fix a bunch of clippy warnings.

## v0.9.0
- Remove deprecated items.

## v0.8.9
- Deprecate borrow() in favor of reborrow().

## v0.8.8
- Support List(Interface).

## v0.8.7
- Eliminate `use` statements in generated code to avoid naming conflicts.

## v0.8.6
- Improve error message for snake_case method names.
- Eliminate floating point literals in match statements.

## v0.8.5
- Implement enum defaults.
- Emit "UNIMPLEMENTED" warnings on struct and list defaults.

## v0.8.4
- Implement struct, list, and enum constants.

## v0.8.3
- Fix bug where schemas with non-trivial relative filesystem paths could fail to compile.

## v0.8.2
- Fix bug where `initn_*()` methods of generic unions failed to set the discriminant.

## v0.8.1
- Fix several formatting issues in generated code.
- Remove some unneccesary trait bounds in generated code.
- Add `import_path()` and `no_std_import()` options to `CompilerCommand`.

## v0.8.0
- Remove deprecated `compile()` and `compile_with_src_prefixes()` functions.

## v0.7.5
- Fix bug that prevented compilation of interfaces with generic superclasses.
- More robust error handling.

## v0.7.4
- Deprecate `compile()` and `compile_with_src_prefixes()` in favor of `CompilerCommand`.

## v0.7.3
- `capnpc -orust ./foo/bar/baz.capnp` now correctly writes to `./foo/bar/baz_capnp.rs` rather than
  just `./baz_capnp.rs`. If you were depending on the old behavior you can use the `--src-prefix`
  flag for finer-grained control of the output location.

## v0.7.2
- Nicer formatting for floating point literals.

## v0.7.1
- Fix bug that prevented pipelining on an AnyPointer field.
