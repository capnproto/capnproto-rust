## v0.19.7
- Add `Results::set_pipeline()` and `ResultsHook::set_pipeline()`.

## v0.19.6
- Fix ExactSizeIterator implementations so that they return the number of
  remaining elements instead of the total length of the underlying list.

## v0.19.5
- Fix bug in dynamic reflection where `get_named()` and `has_named()` could
  panic on a field that is not present in the schema.

## v0.19.4
- Fix possible undefined behavior in primitive_list::as_slice() on empty lists.
- Disable primitive_list::as_slice() when T is larger than one byte and the
  `unaligned` feature is enabled.
- Enable primitive_list::as_slice() for big-endian targets when T is at most
  one byte.

## v0.19.3
- Rename ReadSegmentTableResult to NoAllocSegmentTableInfo and make it public.
- Rename NoAllocBufferSegments::from_segment_table() to
  NoAllocBufferSegments::from_segment_table_info() and make it public.

## v0.19.2
- Revert SingleSegmentAllocator generalization because it was unsound.

## v0.19.1
- Implement SetterInput<text::Owned> for all T : AsRef<str>.

## v0.19.0
- Use binary search instead of linear scan in DynamicStruct::get_named().
- Rename SetPointerBuilder to SetterInput.
- Add Receiver type parameter to SetterInput.
- Support setting text fields by text::Reader or &str, via the SetterInput tactic.
  This will break code that uses the into() to convert from str to text::Reader
  in the arguments of such methods.
- Also support setting primitive list fields by native Rust slices, and text list
  fields by slices of AsRef<str>.
- Update embedded-io dependency to version 0.6.1.
- Use AsRef<[u8]> instead of Deref<Target=[u8]> in NoAllocBufferSegments.
- Generalize SingleSegmentAllocator to take any type that implements AsMut<[u8]>.

## v0.18.13
- Add PartialEq impls for text::Reader <-> String.

## v0.18.12
- Regenerate schema_capnp.rs after fixing overly-restrictive lifetimes for struct lists.

## v0.18.11
- Add PartialOrd impls for text::Reader.

## v0.18.10
- Add Debug impl for primitive_list::Reader, struct_list::Reader, and others.

## v0.18.9
- Add support for List(Void) in primitive_list::as_slice().

## v0.18.8
- Deprecate StructBuilder::get_pointer_field_mut().
- Improve docstring on dynamic_struct::Reader::has().

## v0.18.7
- Update try_push_segment() to avoid possible overflow panic in 32-bit mode.

## v0.18.6
- Add overflow checking during segment table reading, to prevent some potential denial
  of service attacks on 32-bit targets.
- Deprecate SegmentLengthsBuilder::push_segment() in favor of try_push_segment().

## v0.18.5
- Add read_message_no_alloc() and try_read_message_no_alloc() in serialize and serialize_packed.
- Enable write_message() in no-alloc mode.

## v0.18.4
- Map std::io::ErrorKind::UnexpectedEof to capnp::ErrorKind::PrematureEndOfFile.

## v0.18.3
- Make BuilderArena usable in no-alloc contexts. Only single-segment messages
  are supported.

## v0.18.2
- Add SingleSegmentAllocator, for use in no-alloc contexts.

## v0.18.1
- Add #[inline] attribute to many text::Reader and text::Builder methods.

## v0.18.0
- Add optional (default-enabled) `alloc` feature to allow no-alloc mode.
- Lazier UTF-8 validation.
- Add missing #[inline] attributes for f32 and f64.
- Add optional `embedded-io` feature.
- Make SliceSegments a special case of BufferSegments.

## v0.17.2
- Fix indexing bug in `schema::FieldSubset`.

## v0.17.1
- Fix type mismatch copy/paste bug in `dynamic_list::Builder::set()`.

## v0.17.0
- Add support for reflection. See dynamic_value.rs and schema.rs.

## v0.16.1
- Fix "stacked borrow" errors found by miri.

## v0.16.0
- Remove deprecated `HasTypeId::type_id()` method.
- Remove deprecated `MessageSize::plus_eq` method.
- Remove `RefCell` from builder arena. Should result in minor performance boost.
- Mark `Allocator::deallocate_segment` as `unsafe`.
- Remove `ToU16` and `FromU16` traits in favor of `core::convert` traits.
- Remove `FromStructBuilder` and `FromStructReader` traits in favor of `core::convert` traits.

## v0.15.3
- Deprecate `HasTypeId::type_id()` in favor of `HasTypeId::TYPE_ID`.

## v0.15.2
- Remove list pointer munging.

## v0.15.1
- Add `rust-version` field in Cargo.toml, for better error messages when somone uses and old rustc.
- Add `is_empty()` methods.
- Deprecate `MessageSize::plus_eq` in favor of `AddAssign`.
- Add some `Default` impls.
- Lots of linting and formatting changes that should not have an observeable effect.

## v0.15.0
- Move HasStructSize::struct_size() into a constant HasStructSize::STRUCT_SIZE.
- Move HasTypeId::type_id() into a constant HasTypeId::TYPE_ID.
- Updated minimum supported rustc versino to 1.65.0.
- Use generic associated types in Owned and OwnedStruct.
- Add capability::get_resolved_cap().

## v0.14.10
- Handle case when `alloc::alloc_zeroed()` returns null.

## v0.14.9
- Add `try_get()` method for the lists.
- Add missing bounds checking in `text_list::Builder`.
- Improve documentation.

## v0.14.8
- Fix potential integer overflows in `set_list_pointer()` and `zero_object_helper()`.

## v0.14.7
- Add serialize::read_message_from_flat_slice_no_alloc().

## v0.14.6
- Update rpc_try feature to work with try_trait_v2

## v0.14.5
- Add capnp::serialize::BufferSegments.

## v0.14.4
- Add capnp::message::TypedBuilder.
- Add as_slice() methods for primitive_list. (These are only enabled for little endian targets.)

## v0.14.3
- Add list_list::Builder::set().

## v0.14.2
- Add HeapAllocator::max_segment_words().
- Avoid potential integer overflows that could cause too many segments to be allocated.

## v0.14.1
- Include LICENSE in published crate.

## v0.14.0
- Add `sync_reader` feature, which allows multithreaded reading of a shared message.
- Change `ReaderOptions.traversal_limit_in_words` from a `u64` to an `Option<usize>`.
- Remove unneeded `To` type parameter of `SetPointerBuilder`.

## v0.13.6
- Add blanket impl Allocator for &mut A where A: Allocator, allowing easier reuse of ScratchSpaceHeapAllocator.

## v0.13.5
- Fix incorrect calculation in `capnp::serialize::compute_serialized_size_in_words()`.

## v0.13.4
- Deprecate unsafe functions `data::new_reader()` and `data::new_builder()`.

## v0.13.3
- Add `impl <S> ReaderSegments for &S where S: ReaderSegments`.

## v0.13.2
- Fix bug where `read_segment_table()` wrongly handled short reads.

## v0.13.1
- Add alignment check in `ScratchSpaceHeapAllocator::new()`.

## v0.13.0
- Add no_std support, via a new "std" feature flag.
- Simplify `message::Allocator` trait and `ScratchSpaceHeapAllocator`.
- Add `serialize.try_read_message()` and `serialize_packed::try_read_message()`.
- Remove deprecated `ServerHook` trait.

## v0.12.3
- Fix bug where ScratchSpaceHeapAllocator returned an incorrect buffer length.

## v0.12.2
- Add capability::FromServer trait.

## v0.12.1
- Fix buggy Iterator::nth() implementation for ListIter.

## v0.12.0
- Add "unaligned" feature flag to allow use of unaligned memory.
- Remove `Word::bytes_to_words()` and `Word::bytes_to_words_mut()`.
- Change a bunch of interfaces to use `u8` instead of `Word`.
- Remove `read_message_from_words()` in favor of `read_message_from_flat_slice()`.
- Add new `serialize::SegmentLengthsBuilder` API.
- Bump minimum required rustc version to 1.40.0.

## v0.11.2
- Deprecate `read_message_from_words()` in favor of `read_message_from_flat_slice()`.
- Remove incorrect doc comments on `bytes_to_words()`. (Misaligned access is never okay.)

## v0.11.1
- Remove internal capnp::map::Map and use async/await instead.

## v0.11.0
- Remove the "futures" feature and the optional futures 0.1 dependency, in favor of std::future::Future.
- Bump minimum support rustc version to 1.39.0.

## v0.10.3
- Add serialize::read_message_from_flat_slice().

## v0.10.2
- Allow buffer passed to read_message_from_words() to be larger than the actual message.

## v0.10.1
- Remove dependency on byteorder crate, in favor of from_le_bytes() and to_le_bytes().

## v0.10.0
- Simplify handling of pointer defaults by adding default parameter to FromPointerReader.
- Add IntoInternalStructReader as a bound on OwnedStruct::Reader.
- Remove capnp_word!() macro in favor of const fn ::capnp::word().
- Remove deprecated items.
- Update to 2018 edition.
- Use dyn keyword for trait objects.
- Update minimum required rustc version to 1.35.

## v0.9.5
- Implement DerefMut for text::Builder
- Add any_pointer_list and raw::get_struct_pointer_section().
- Add support for pointer field defaults.

## v0.9.4
- Add optional rpc_try feature, implementing std::ops::Try for Promise.
- Add 'raw' module with get_struct_data_section(), get_list_bytes(), and other functions.
- Avoid potential undefined behavior in canonicalizaion.
- Update a bunch of internal usages of `try!()` to `?`.

## v0.9.3
- Add IntoInternalStructReader trait and struct_list::Builder::set_with_caveats() method.
- Update deprecation attributes, to satisfy clippy.

## v0.9.2
- Rename a bunch of as_reader() methods to into_reader(), to satisfy clippy.

## v0.9.1
- Avoid some unnecessary heap allocation that could occur when reading multisegment messages.

## v0.9.0
- Add message::Builder::set_root_canonical() method. Relies on a new signature for SetPointerBuilder.
- Mark bytes_to_words() and bytes_to_words_mut() as unsafe, due to possible alignment issues. Please
  refer to https://github.com/capnproto/capnproto-rust/issues/101 for discussion.
- Delete deprecated items.
- Drop support for automatically imbuing message builders with capabilities (was unsafe). You should
  use capnp_rpc::ImbuedMessageBuilder now if you want that functionality. See the calculator example.
- Bump minimum supported rustc version to 1.26.0.

## v0.8.17
- Deprecate borrow() in favor of reborrow().

## v0.8.16
- Add serialize::write_message_segments().
- Fix bug where is_canonical() could sometimes erroneously return true.

## v0.8.15
- Add message::Builder::into_reader() and message::Reader::into_typed().

## v0.8.14
- Add message::TypedReader.
- Appease new "tyvar_behind_raw_pointer" lint (see https://github.com/rust-lang/rust/issues/46906).

## v0.8.13
- Implement capability_list, to support List(Interface).

## v0.8.12
- Avoid constructing (zero-length) slices from null pointers, as it seems to be a possible
  source of undefined behavior.
- Add some IntoIterator implementations.

## v0.8.11
- Avoid some situations where we would construct (but not dereference) out-of-bounds pointers.

## v0.8.10
- Deprecate Word::from() in favor of capnp_word!().
- Add constant::Reader to support struct and list constants.

## v0.8.9
- In canonicalization, account the possibility of nonzero padding in primitive lists.
- Do bounds-checking by (ptr, size) pairs rather than (ptr, end_ptr) pairs.

## v0.8.8
- Fix some canonicalization bugs.

## v0.8.7
- Implement `as_reader()` for lists.
- Implement `canonicalize()` and `is_canonical()`.
- Fix bug where `total_size()` returned wrong answer on empty struct lists.

## v0.8.6
- Implement struct list upgrades.
- Fix bug where `message.init_root::<any_pointer::Builder>()` did not clear the old value.

## v0.8.5
- Eliminate possible void-list-amplification in total_size().

## v0.8.4
- Eliminate panics in total_size() and set_root().
- Eliminate possible void-list-amplification in zero_object_helper().

## v0.8.3
- Prevent integer overflow possible with very long struct lists on 32-bit systems.
- Fix bug where the capnp_word!() macro was not exported for big endian targets.

## v0.8.2
- Shave some bytes off the representation of StructReader and friends.
- Fix some potential integer overflows.

## v0.8.1
- Redesign segment arenas to require less unsafe code.

## v0.8.0
- Replace optional GJ dependency with futures-rs.
- Remove `ResultsDoneHook` hack.
- No breaking changes for non-RPC users.

## v0.7.5
- Implement DoubleEndedIter for ListIter.
- Implement From<std::str::Utf8Error> for ::capnp::Error.
- Address some new linter warnings.

## v0.7.4
- Fix rare case where `serialize_packed::read()` could fail on valid input.

## v0.7.3
- Get `message::Builder::get_root_as_reader()` to work on empty messages.

## v0.7.2
- Implement `From<std::string::FromUtf8Error>` for `capnp::Error`
- More and better iterators.
