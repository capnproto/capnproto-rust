/*
 * Copyright (c) 2013-2014, David Renshaw (dwrenshaw@gmail.com)
 *
 * See the LICENSE file in the capnproto-rust root directory.
 */

use std;
use std::vec::Vec;
use common::*;
use endian::*;
use message::*;
use arena;
use io;


pub struct OwnedSpaceMessageReader {
    options : ReaderOptions,
    arena : Box<arena::ReaderArena>,
    segment_slices : Vec<(uint, uint)>,
    owned_space : Vec<Word>,
}

impl <'a> MessageReader<'a> for OwnedSpaceMessageReader {
    fn get_segment(&self, id : uint) -> &[Word] {
        let (a,b) = self.segment_slices.as_slice()[id];
        self.owned_space.slice(a, b)
    }

    fn arena(&self) -> &arena::ReaderArena { &*self.arena }
    fn mut_arena(&mut self) -> &mut arena::ReaderArena { &mut *self.arena }

    fn get_options(&self) -> &ReaderOptions {
        return &self.options;
    }
}

fn invalid_input<T>(desc : &'static str) -> std::io::IoResult<T> {
    return Err(std::io::IoError{ kind : std::io::InvalidInput,
                                 desc : desc,
                                 detail : None});
}

pub fn new_reader<U : std::io::Reader>(input_stream : &mut U,
                                       options : ReaderOptions)
                                       -> std::io::IoResult<OwnedSpaceMessageReader> {

    let first_word = try!(input_stream.read_exact(8));

    let segment_count : u32 =
        unsafe {let p : *const WireValue<u32> = std::mem::transmute(first_word.as_ptr());
                (*p).get() + 1
    };

    let segment0_size =
        if segment_count == 0 { 0 } else {
        unsafe {let p : *const WireValue<u32> = std::mem::transmute(first_word.as_slice().unsafe_get(4));
                (*p).get()
        }
    };

    let mut total_words = segment0_size;

    if segment_count >= 512 {
        return invalid_input("too many segments");
    }

    let mut more_sizes : Vec<u32> = Vec::with_capacity((segment_count & !1) as uint);

    if segment_count > 1 {
        let more_sizes_raw = try!(input_stream.read_exact((4 * (segment_count & !1)) as uint));
        for ii in range(0, segment_count as uint - 1) {
            let size = unsafe {
                let p : *const WireValue<u32> =
                    std::mem::transmute(more_sizes_raw.as_slice().unsafe_get(ii * 4));
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

    let mut owned_space : Vec<Word> = allocate_zeroed_words(total_words as uint);
    let buf_len = total_words as uint * BYTES_PER_WORD;

    unsafe {
        let ptr : *mut u8 = std::mem::transmute(owned_space.as_mut_slice().as_mut_ptr());
        try!(std::slice::raw::mut_buf_as_slice::<u8,std::io::IoResult<uint>>(ptr, buf_len, |buf| {
                    io::read_at_least(input_stream, buf, buf_len)
                }));
    }

    // TODO(maybe someday) lazy reading like in capnp-c++?

    let mut segment_slices : Vec<(uint, uint)> = vec!((0, segment0_size as uint));

    let arena = {
        let segment0 : &[Word] = owned_space.slice(0, segment0_size as uint);
        let mut segments : Vec<&[Word]> = vec!(segment0);

        if segment_count > 1 {
            let mut offset = segment0_size;

            for ii in range(0, segment_count as uint - 1) {
                segments.push(owned_space.slice(offset as uint,
                                               (offset + more_sizes.as_slice()[ii]) as uint));
                segment_slices.push((offset as uint,
                                     (offset + more_sizes.as_slice()[ii]) as uint));
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


pub fn write_message<'a, T : std::io::Writer, U : MessageBuilder<'a>>(
    output_stream : &mut T,
    message : &U) -> std::io::IoResult<()> {

    try!(message.get_segments_for_output(
        |segments| {

            let table_size : uint = (segments.len() + 2) & (!1);

            let mut table : Vec<WireValue<u32>> = Vec::with_capacity(table_size);
            unsafe { table.set_len(table_size) }

            table.as_mut_slice()[0u].set((segments.len() - 1) as u32);

            for i in range(0, segments.len()) {
                table.as_mut_slice()[i + 1].set(segments[i].len() as u32);
            }
            if segments.len() % 2 == 0 {
                // Set padding.
                table.as_mut_slice()[segments.len() + 1].set( 0 );
            }

            unsafe {
                let ptr : *const u8 = std::mem::transmute(table.as_ptr());
                try!(std::slice::raw::buf_as_slice::<u8,std::io::IoResult<()>>(ptr, table.len() * 4, |buf| {
                        output_stream.write(buf)
                    }));
            }

            for i in range(0, segments.len()) {
                unsafe {
                    let ptr : *const u8 = std::mem::transmute(segments[i].as_ptr());
                    try!(std::slice::raw::buf_as_slice::<u8,std::io::IoResult<()>>(
                        ptr,
                        segments[i].len() * BYTES_PER_WORD,
                        |buf| { output_stream.write(buf) }));
                }
            }
            Ok(())
        }));

    output_stream.flush()
}
