// Copyright (c) 2013-2015 Sandstorm Development Group, Inc. and contributors
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

use private::units::*;
use private::endian::WireValue;
use message::*;
use private::arena;
use io;
use Word;

pub struct OwnedSpaceMessageReader {
    options : ReaderOptions,
    arena : Box<arena::ReaderArena>,
    segment_slices : Vec<(usize, usize)>,
    owned_space : Vec<Word>,
}

impl MessageReader for OwnedSpaceMessageReader {
    fn get_segment(&self, id : usize) -> &[Word] {
        let (a,b) = self.segment_slices.as_slice()[id];
        &self.owned_space[a .. b]
    }

    fn arena(&self) -> &arena::ReaderArena { &*self.arena }
    fn mut_arena(&mut self) -> &mut arena::ReaderArena { &mut *self.arena }

    fn get_options(&self) -> &ReaderOptions {
        return &self.options;
    }
}

fn invalid_input<T>(desc : &'static str) -> ::std::old_io::IoResult<T> {
    return Err(::std::old_io::IoError{ kind : ::std::old_io::InvalidInput,
                                       desc : desc,
                                       detail : None});
}

pub fn new_reader<U : ::std::old_io::Reader>(input_stream : &mut U,
                                             options : ReaderOptions)
                                             -> ::std::old_io::IoResult<OwnedSpaceMessageReader> {

    let first_word = try!(input_stream.read_exact(8));

    let segment_count : u32 =
        unsafe {let p : *const WireValue<u32> = ::std::mem::transmute(first_word.as_ptr());
                (*p).get() + 1
    };

    let segment0_size =
        if segment_count == 0 { 0 } else {
        unsafe {let p : *const WireValue<u32> = ::std::mem::transmute(first_word.get_unchecked(4));
                (*p).get()
        }
    };

    let mut total_words = segment0_size;

    if segment_count >= 512 {
        return invalid_input("too many segments");
    }

    let mut more_sizes : Vec<u32> = Vec::with_capacity((segment_count & !1) as usize);

    if segment_count > 1 {
        let more_sizes_raw = try!(input_stream.read_exact((4 * (segment_count & !1)) as usize));
        for ii in 0..(segment_count as usize - 1) {
            let size = unsafe {
                let p : *const WireValue<u32> =
                    ::std::mem::transmute(more_sizes_raw.get_unchecked(ii * 4));
                (*p).get()
            };
            more_sizes.push(size);
            total_words += size;
        }
    }

    //# Don't accept a message which the receiver couldn't possibly
    //# traverse without hitting the traversal limit. Without this
    //# check, a malicious client could transmit a very large
    //# segment size to make the receiver allocate excessive space
    //# and possibly crash.
    if ! (total_words as u64 <= options.traversal_limit_in_words)  {
        return invalid_input("Message is too large. To increase the limit on the \
                              receiving end, see capnp::ReaderOptions.");
    }

    let mut owned_space : Vec<Word> = Word::allocate_zeroed_vec(total_words as usize);
    let buf_len = total_words as usize * BYTES_PER_WORD;

    unsafe {
        let ptr : *mut u8 = ::std::mem::transmute(owned_space.as_mut_slice().as_mut_ptr());
        let buf = ::std::slice::from_raw_parts_mut::<u8>(ptr, buf_len);
        try!(io::read_at_least(input_stream, buf, buf_len));
    }

    // TODO(maybe someday) lazy reading like in capnp-c++?

    let mut segment_slices : Vec<(usize, usize)> = vec!((0, segment0_size as usize));

    let arena = {
        let segment0 : &[Word] = &owned_space[0 .. segment0_size as usize];
        let mut segments : Vec<&[Word]> = vec!(segment0);

        if segment_count > 1 {
            let mut offset = segment0_size;

            for ii in 0..(segment_count as usize - 1) {
                segments.push(&owned_space[offset as usize ..
                                           (offset + more_sizes.as_slice()[ii]) as usize]);
                segment_slices.push((offset as usize,
                                     (offset + more_sizes.as_slice()[ii]) as usize));
                offset += more_sizes.as_slice()[ii];
            }
        }
        arena::ReaderArena::new(segments.as_slice(), options)
    };

    Ok(OwnedSpaceMessageReader {
        segment_slices : segment_slices,
        owned_space : owned_space,
        arena : arena,
        options : options,
    })
}


pub fn write_message<T : ::std::old_io::Writer, U : MessageBuilder>(
    output_stream : &mut T,
    message : &U) -> ::std::old_io::IoResult<()> {

    try!(message.get_segments_for_output(
        |segments| {

            let table_size : usize = (segments.len() + 2) & (!1);

            let mut table : Vec<WireValue<u32>> = Vec::with_capacity(table_size);
            unsafe { table.set_len(table_size) }

            table.as_mut_slice()[0].set((segments.len() - 1) as u32);

            for i in 0..segments.len() {
                table.as_mut_slice()[i + 1].set(segments[i].len() as u32);
            }
            if segments.len() % 2 == 0 {
                // Set padding.
                table.as_mut_slice()[segments.len() + 1].set( 0 );
            }

            unsafe {
                let ptr : *const u8 = ::std::mem::transmute(table.as_ptr());
                let buf = ::std::slice::from_raw_parts::<u8>(ptr, table.len() * 4);
                try!(output_stream.write_all(buf));
            }

            for i in 0..segments.len() {
                unsafe {
                    let ptr : *const u8 = ::std::mem::transmute(segments[i].as_ptr());
                    let buf = ::std::slice::from_raw_parts::<u8>(ptr, segments[i].len() * BYTES_PER_WORD);
                    try!(output_stream.write_all(buf));
                }
            }
            Ok(())
        }));

    output_stream.flush()
}
