// Copyright (c) 2017 Sandstorm Development Group, Inc. and contributors
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

#[macro_use]
extern crate capnp;

use capnp::{Word, message};

#[test]
fn canonical_non_null_empty_struct_field() {
    let segment: &[Word] = &[
        // Struct pointer, body immediately follows, two pointer fields, no data.
        capnp_word!(0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x02, 0x00),

        // First pointer field, struct, offset of 1, data size 1, no pointers.
        capnp_word!(0x04, 0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00),

        // Non-null pointer to empty struct.
        capnp_word!(0xfc, 0xff, 0xff, 0xff, 0x00, 0x00, 0x00, 0x00),

        // Body of struct filled with non-zero data.
        capnp_word!(0xee, 0xee, 0xee, 0xee, 0xee, 0xee, 0xee, 0xee),
    ];

    let segments = &[segment];
    let segment_array = message::SegmentArray::new(segments);
    let message = message::Reader::new(segment_array, Default::default());
    assert!(message.is_canonical().unwrap());
}

#[test]
fn pointer_to_empty_struct_preorder_not_canonical() {
    let segment: &[Word] = &[
        // Struct pointer, body immediately follows, two pointer fields, no data.
        capnp_word!(0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x02, 0x00),

        // First pointer field, struct, offset of 1, data size 1, no pointers.
        capnp_word!(0x04, 0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00),

        // Non-null pointer to empty struct. Offset puts it in "preorder". Would need to have
        // an offset of -1 to be canonical.
        capnp_word!(0x04, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00),

        // Body of struct filled with non-zero data.
        capnp_word!(0xee, 0xee, 0xee, 0xee, 0xee, 0xee, 0xee, 0xee),
    ];

    let segments = &[segment];
    let segment_array = message::SegmentArray::new(segments);
    let message = message::Reader::new(segment_array, Default::default());
    assert!(!message.is_canonical().unwrap());
}

#[test]
fn is_canonical_requires_pointer_preorder() {
    let segment: &[Word] = &[
        //Struct pointer, data immediately follows, two pointer fields, no data
        capnp_word!(0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x02, 0x00),

        //Pointer field 1, pointing to the last entry, data size 1, no pointer
        capnp_word!(0x08, 0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00),

        //Pointer field 2, pointing to the next entry, data size 2, no pointer
        capnp_word!(0x00, 0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00),

        //Data for field 2
        capnp_word!(0x00, 0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07),

        //Data for field 1
        capnp_word!(0x07, 0x06, 0x05, 0x04, 0x03, 0x02, 0x01, 0x00),
    ];

    let segments = &[segment];
    let segment_array = message::SegmentArray::new(segments);
    let message = message::Reader::new(segment_array, Default::default());
    assert!(!message.is_canonical().unwrap());
}

#[test]
fn is_canonical_requires_dense_packing() {
    let segment: &[Word] = &[
        //Struct pointer, data after a gap
        capnp_word!(0x03, 0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00),

        //The gap
        capnp_word!(0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00),

        //Data for field 1
        capnp_word!(0x00, 0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07),
    ];

    let segments = &[segment];
    let segment_array = message::SegmentArray::new(segments);
    let message = message::Reader::new(segment_array, Default::default());
    assert!(!message.is_canonical().unwrap());
}

#[test]
fn is_canonical_rejects_multisegment_messages() {
    let segment0: &[Word] = &[
        //Far pointer to next segment
        capnp_word!(0x02, 0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00),
    ];

    let segment1: &[Word] = &[
        //Struct pointer (needed to make the far pointer legal)
        capnp_word!(0x00, 0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00),

        //Dummy data
        capnp_word!(0x00, 0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07),
    ];

    let segments = &[segment0, segment1];
    let segment_array = message::SegmentArray::new(segments);
    let message = message::Reader::new(segment_array, Default::default());
    assert!(!message.is_canonical().unwrap());
}

#[test]
fn is_canonical_requires_truncation_of_0_valued_struct_fields() {
    let segment: &[Word] = &[
        //Struct pointer, data immediately follows
        capnp_word!(0x00, 0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00),

        //Default data value, should have been truncated
        capnp_word!(0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00),
    ];

    let segments = &[segment];
    let segment_array = message::SegmentArray::new(segments);
    let message = message::Reader::new(segment_array, Default::default());
    assert!(!message.is_canonical().unwrap());
}

#[test]
fn is_canonical_rejects_unused_trailing_words() {
    let segment: &[Word] = &[
        // Struct pointer, data in next word
        capnp_word!(0x00, 0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00),

        // Data section of struct
        capnp_word!(0x00, 0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07),

        // Trailing zero word
        capnp_word!(0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00),
    ];

    let segments = &[segment];
    let segment_array = message::SegmentArray::new(segments);
    let message = message::Reader::new(segment_array, Default::default());
    assert!(!message.is_canonical().unwrap());
}

#[test]
fn is_canonical_accepts_empty_inline_composite_list_of_0_sized_structs() {
    let segment: &[Word] = &[
        // Struct pointer, pointer in next word
        capnp_word!(0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x01, 0x00),

        // List pointer, inline composite, zero words long
        capnp_word!(0x01, 0x00, 0x00, 0x00, 0x07, 0x00, 0x00, 0x00),

        // Tag word
        capnp_word!(0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00),
    ];

    let segments = &[segment];
    let segment_array = message::SegmentArray::new(segments);
    let message = message::Reader::new(segment_array, Default::default());
    assert!(message.is_canonical().unwrap());
}

#[test]
fn is_canonical_rejects_inline_composite_list_with_inaccurate_word_length() {
    let segment: &[Word] = &[
        // Struct pointer, no offset, pointer section has two entries
        capnp_word!(0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x02, 0x00),

        // List pointer, offset of one, inline composite, two words long
        // (The list only needs to be one word long to hold its actual elements;
        // therefore this message is not canonical.)
        capnp_word!(0x05, 0x00, 0x00, 0x00, 0x17, 0x00, 0x00, 0x00),

        // Struct pointer, offset two, data section has one word
        capnp_word!(0x08, 0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00),

        // Tag word, struct, one element, one word data section
        capnp_word!(0x04, 0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00),

        // Data section of struct element of list
        capnp_word!(0x00, 0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07),

        // Data section of struct field in top-level struct
        capnp_word!(0x07, 0x06, 0x05, 0x04, 0x03, 0x02, 0x01, 0x00),
    ];

    let segments = &[segment];
    let segment_array = message::SegmentArray::new(segments);
    let message = message::Reader::new(segment_array, Default::default());
    assert!(!message.is_canonical().unwrap());
}
