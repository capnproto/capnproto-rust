use core::convert::TryInto;

use alloc::string::ToString;

use crate::message::ReaderOptions;
use crate::message::ReaderSegments;
use crate::private::units::BYTES_PER_WORD;
use crate::Error;
use crate::Result;

use super::SEGMENTS_COUNT_LIMIT;

const U32_LEN_IN_BYTES: usize = core::mem::size_of::<u32>();

/// Segments read from a buffer, useful for when you have the message in a buffer and don't want the
/// extra copy of `read_message`.
///
/// `NoAllocSliceSegments` is similar to [`crate::serialize::SliceSegments`] but optimized for
/// low memory embedded environment. It does not do heap allocations and consumes only one fat
/// pointer worth of memory.
///
/// # Performance considerations
///
/// Due to lack of heap allocations, `NoAllocSliceSegments` does not cache segments offset and
/// length and has to parse message header every time `NoAllocSliceSegments::get_segment` is called.
/// The parsing has O(N) complexity where N is total number of segments in the message.
/// `NoAllocSliceSegments` has optimization for single segment messages: if message has only one
/// segment, it will be parsed only once during creation and no parsing will be required on `get_segment` calls
pub enum NoAllocSliceSegments<'b> {
    SingleSegment { segment: &'b [u8] },
    MultipleSegments { message: &'b [u8] },
}

impl<'b> NoAllocSliceSegments<'b> {
    /// Reads a serialized message (including a segment table) from a buffer and takes ownership, without copying.
    /// The slice is allowed to extend beyond the end of the message. On success, updates `slice` to point
    /// to the remaining bytes beyond the end of the message.
    ///
    /// ALIGNMENT: If the "unaligned" feature is enabled, then there are no alignment requirements on `buffer`.
    /// Otherwise, `buffer` must be 8-byte aligned (attempts to read the message will trigger errors).
    pub fn try_new(slice: &mut &'b [u8], options: ReaderOptions) -> Result<Self> {
        let mut remaining = *slice;

        verify_alignment(remaining.as_ptr())?;

        let segments_count = u32_to_segments_count(read_u32_le(&mut remaining)?)?;

        if segments_count >= SEGMENTS_COUNT_LIMIT {
            return Err(Error::failed(format!(
                "Too many segments: {}",
                segments_count
            )));
        }

        let mut total_segments_length_bytes = 0_usize;

        for _ in 0..segments_count {
            let segment_length_in_bytes =
                u32_to_segment_length_bytes(read_u32_le(&mut remaining)?)?;

            total_segments_length_bytes = total_segments_length_bytes
                .checked_add(segment_length_in_bytes)
                .ok_or_else(|| {
                    Error::failed(
                        "Message is too large; it's size cannot be represented in usize."
                            .to_string(),
                    )
                })?;
        }

        // Don't accept a message which the receiver couldn't possibly traverse without hitting the
        // traversal limit. Without this check, a malicious client could transmit a very large segment
        // size to make the receiver allocate excessive space and possibly crash.
        if let Some(limit) = options.traversal_limit_in_words {
            let total_segments_length_words = total_segments_length_bytes / 8;
            if total_segments_length_words > limit {
                return Err(Error::failed(format!(
                    "Message has {} words, which is too large. To increase the limit on the \
                             receiving end, see capnp::message::ReaderOptions.",
                    total_segments_length_words
                )));
            }
        }

        // If number of segments is even, header length will not be aligned by 8, we need to consume
        // padding from the remainder of the message
        if segments_count % 2 == 0 {
            let _padding = read_u32_le(&mut remaining)?;
        }

        let expected_data_offset = calculate_data_offset(segments_count).ok_or_else(|| {
            Error::failed(
                "Message is too large; it's size cannot be represented in usize".to_string(),
            )
        })?;

        let consumed_bytes = slice.len() - remaining.len();

        assert_eq!(
            expected_data_offset, consumed_bytes,
            "Expected header size and actual header size must match, otherwise we have a bug in this code"
        );

        // If data section of the message is smaller than calculated total segments length, the message
        // is malformed. It looks like it's ok to have extra bytes in the end, according to
        // of `SliceSegments` implementation.
        if remaining.len() < total_segments_length_bytes {
            return Err(Error::failed(format!(
                "Message ends prematurely. Header claimed {} words, but message only has {} words.",
                total_segments_length_bytes / BYTES_PER_WORD,
                remaining.len() / BYTES_PER_WORD
            )));
        }

        let message_length = expected_data_offset + total_segments_length_bytes;
        let message = &slice[..message_length];

        *slice = &slice[message_length..];

        if segments_count == 1 {
            Ok(Self::SingleSegment {
                segment: &message[expected_data_offset..],
            })
        } else {
            Ok(Self::MultipleSegments { message })
        }
    }
}

impl<'b> ReaderSegments for NoAllocSliceSegments<'b> {
    fn get_segment(&self, idx: u32) -> Option<&[u8]> {
        // panic safety: we are doing a lot of `unwrap` here. We assume that underlying message slice
        // holds valid capnp message - we already verified slice in NoAllocSliceSegments::try_new,
        // so these unwraps are not expected to panic unless we have bug in the code.

        let idx: usize = idx.try_into().unwrap();

        match self {
            NoAllocSliceSegments::SingleSegment { segment } => {
                if idx == 0 {
                    Some(segment)
                } else {
                    None
                }
            }
            NoAllocSliceSegments::MultipleSegments { message } => {
                let mut buf = *message;

                let segments_count = u32_to_segments_count(read_u32_le(&mut buf).unwrap()).unwrap();

                if idx >= segments_count {
                    return None;
                }

                let mut segment_offset = calculate_data_offset(segments_count).unwrap();

                for _ in 0..idx {
                    segment_offset = segment_offset
                        .checked_add(
                            u32_to_segment_length_bytes(read_u32_le(&mut buf).unwrap()).unwrap(),
                        )
                        .unwrap();
                }

                let segment_length =
                    u32_to_segment_length_bytes(read_u32_le(&mut buf).unwrap()).unwrap();

                Some(&message[segment_offset..(segment_offset + segment_length)])
            }
        }
    }

    fn len(&self) -> usize {
        // panic safety: we are doing a lot of `unwrap` here. We assume that underlying message slice
        // holds valid capnp message - we already verified slice in NoAllocSliceSegments::try_new

        match self {
            NoAllocSliceSegments::SingleSegment { .. } => 1,
            NoAllocSliceSegments::MultipleSegments { mut message } => {
                u32_to_segments_count(read_u32_le(&mut message).unwrap()).unwrap()
            }
        }
    }
}

/// Verifies whether pointer meets alignment requirements
///
/// If crate is compiled with "unaligned" feature, then this function does nothing since
/// there are no alignment requirements in this mode.
///
/// If crate was not compiled with "unaligned" feature, it will verify that pointer is aligned
/// by WORD boundary.
fn verify_alignment(ptr: *const u8) -> Result<()> {
    if cfg!(feature = "unaligned") {
        return Ok(());
    }

    let ptr = ptr as usize;
    if ptr % BYTES_PER_WORD == 0 {
        Ok(())
    } else {
        Err(Error::failed(
            "Message was not aligned by 8 bytes boundary. \
            Either ensure that message is properly aligned or compile \
            `capnp` crate with \"unaligned\" feature enabled."
                .to_string(),
        ))
    }
}

/// Reads u32 little endian value from the front of the slice and truncates processed bytes
/// Returns Error if there are not enough bytes to read u32
fn read_u32_le(slice: &mut &[u8]) -> Result<u32> {
    if slice.len() < U32_LEN_IN_BYTES {
        return Err(Error::disconnected("Message ended too soon".to_string()));
    }

    // Panic safety: we just confirmed that `slice` has at least `U32_LEN_IN_BYTES` so nothing
    // here should panic
    let u32_buf: [u8; U32_LEN_IN_BYTES] = slice[..U32_LEN_IN_BYTES].try_into().unwrap();
    *slice = &slice[U32_LEN_IN_BYTES..];

    Ok(u32::from_le_bytes(u32_buf))
}

/// Converts 32 bit value which represents encoded segments count in header to usize segment count
fn u32_to_segments_count(val: u32) -> Result<usize> {
    // This conversion can fail on 8 or 16 bit machines.
    let result: Option<usize> = val.try_into().ok();

    // According to encoding schema, segments count is encoded as (count - 1), where 0 means one
    // segment, 1 - two segments and so on, so we need to add +1 to value read from the stream.
    // We need to do +1 to value read from the stream.
    let result = result.and_then(|v: usize| v.checked_add(1));

    result.ok_or_else(|| {
        Error::failed(
            "Cannot represent 4 byte length as `usize`. This may indicate that you are \
            running on 8 or 16 bit platform or message is too large."
                .to_string(),
        )
    })
}

/// Converts 32 bit vlaue which represents encoded segment length to usize segment length in bytes
fn u32_to_segment_length_bytes(val: u32) -> Result<usize> {
    // This convertion can fail on 8 or 16 bit machines.
    let length_in_words: Option<usize> = val.try_into().ok();

    let length_in_bytes = length_in_words.and_then(|l| l.checked_mul(BYTES_PER_WORD));

    length_in_bytes.ok_or_else(|| {
        Error::failed(
            "Cannot represent 4 byte segment length as usize. This may indicate that \
            you are running on 8 or 16 bit platform or segment is too large"
                .to_string(),
        )
    })
}

/// Calculates expected offset of the message data (beginning of first segment)
/// in the capnp message.
/// Message data comes right after message header and potential padding
///
/// Returns None if it's impossible to calculate offset without arithmentic overflow of usize or
/// if segments count is invalid
fn calculate_data_offset(segments_count: usize) -> Option<usize> {
    // Message data goes right after message header.
    // Message header has following format:
    //
    // Segment count (u32)
    // Segments length (u32 per each segment)
    // Padding to align header size by 8 bytes (it will be either 0 bytes or 4 bytes)

    // It should be impossible to have properly encoded message with 0 segments
    if segments_count == 0 {
        return None;
    }

    let mut data_offset = 0_usize;

    {
        // 4 bytes encoded segments count
        let segments_count_len = U32_LEN_IN_BYTES;
        data_offset = data_offset.checked_add(segments_count_len)?;
    }

    {
        // 4 bytes per each segment
        let segments_lengt_len = segments_count.checked_mul(U32_LEN_IN_BYTES)?;
        data_offset = data_offset.checked_add(segments_lengt_len)?;
    }

    // Message data must be aligned by 8 bytes. If there was even number of segments, then
    // header size will not be aligned by 8, in this case we have to add 4 byte padding to make
    // data offset aligned by 8.
    let padding_len = match data_offset % BYTES_PER_WORD {
        0 => 0,
        4 => 4,
        _ => unreachable!(
            "Mis-alignment by anything other than 4 should be impossible, this is a bug"
        ),
    };

    data_offset = data_offset.checked_add(padding_len)?;

    // It's a sanity check to ensure that message offset has correct alignment
    assert_eq!(
        data_offset % BYTES_PER_WORD,
        0,
        "data_offset after adding panic must be aligned by 8. \
            If it's not, it's a bug"
    );

    Some(data_offset)
}

#[cfg(test)]
mod tests {
    use quickcheck::{quickcheck, TestResult};

    use crate::{
        message::{ReaderOptions, ReaderSegments},
        serialize::{self, no_alloc_slice_segments::calculate_data_offset},
        OutputSegments,
        Word,
    };

    use super::{
        read_u32_le, u32_to_segment_length_bytes, u32_to_segments_count, verify_alignment,
        NoAllocSliceSegments,
    };

    use alloc::vec::Vec;

    #[cfg(feature = "unaligned")]
    #[test]
    fn test_verify_alignment_unaligned_mode() {
        // To run this test do
        // `% cargo test --features unaligned`

        // no alignment requirements in "unaligned" mode
        for ptr in 1024..(1024 + 8) {
            verify_alignment(ptr as *const u8).unwrap();
        }
    }

    #[cfg(not(feature = "unaligned"))]
    #[test]
    fn test_verify_alignment() {
        verify_alignment(1024 as *const u8).unwrap();
        for ptr in 1025..(1024 + 8) {
            verify_alignment(ptr as *const u8).unwrap_err();
        }
    }

    #[test]
    fn test_read_u32_le() {
        let buffer = [0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08];
        let mut buffer_remaining = &buffer[..];

        assert_eq!(read_u32_le(&mut buffer_remaining).unwrap(), 0x04030201);
        assert_eq!(buffer_remaining, &buffer[4..]);
    }

    #[test]
    fn test_read_u32_le_truncated() {
        let buffer = [0x01, 0x02, 0x03];
        let mut buffer_remaining = &buffer[..];

        read_u32_le(&mut buffer_remaining).unwrap_err();
        assert_eq!(buffer_remaining, &buffer[..]);
    }

    #[test]
    fn test_u32_to_segments_count() {
        assert_eq!(u32_to_segments_count(0).unwrap(), 1);
        assert_eq!(u32_to_segments_count(10).unwrap(), 11);
        // There is no way to reproduce "negative" case on 64 bit machine
    }

    #[test]
    fn test_u32_to_segment_length_bytes() {
        assert_eq!(u32_to_segment_length_bytes(0).unwrap(), 0);
        assert_eq!(u32_to_segment_length_bytes(123).unwrap(), 123 * 8);
    }

    #[test]
    fn test_calculate_data_offset_no_padding() {
        assert_eq!(calculate_data_offset(0), None);

        assert_eq!(calculate_data_offset(1), Some(8));

        assert_eq!(calculate_data_offset(2), Some(16));
        assert_eq!(calculate_data_offset(3), Some(16));

        assert_eq!(calculate_data_offset(100), Some(408));
        assert_eq!(calculate_data_offset(101), Some(408));
    }

    quickcheck! {
        fn test_no_alloc_buffer_segments_single_segment_optimization(
            segment_0 : Vec<Word>) -> TestResult
        {
            let words = &segment_0[..];
            let bytes = Word::words_to_bytes(words);
            let output_segments = OutputSegments::SingleSegment([bytes]);
            let mut msg = vec![];

            serialize::write_message_segments(&mut msg, &output_segments).unwrap();

            let no_alloc_segments =
                NoAllocSliceSegments::try_new(&mut msg.as_slice(), ReaderOptions::new()).unwrap();

            assert!(matches!(
                no_alloc_segments,
                NoAllocSliceSegments::SingleSegment { segment } if segment == bytes
            ));

            assert_eq!(no_alloc_segments.len(), 1);
            assert_eq!(no_alloc_segments.get_segment(0), Some(bytes));
            assert_eq!(no_alloc_segments.get_segment(1), None);
            TestResult::from_bool(true)
        }

        fn test_no_alloc_buffer_segments_multiple_segments(segments_vec: Vec<Vec<Word>>) -> TestResult {
            if segments_vec.is_empty() { return TestResult::discard() };

            let segments: Vec<_> = segments_vec.iter().map(|s|
                                                           Word::words_to_bytes(s.as_slice())).collect();

            let output_segments = OutputSegments::MultiSegment(segments.clone());

            let mut msg = vec![];

            serialize::write_message_segments(&mut msg, &output_segments).unwrap();

            let no_alloc_segments =
                NoAllocSliceSegments::try_new(&mut msg.as_slice(), ReaderOptions::new()).unwrap();

            assert_eq!(no_alloc_segments.len(), segments.len());
            for (i, segment) in segments.iter().enumerate() {
                assert_eq!(no_alloc_segments.get_segment(i as u32), Some(*segment));
            }

            assert_eq!(
                no_alloc_segments.get_segment(no_alloc_segments.len() as u32),
                None
            );
            TestResult::from_bool(true)
        }
    }

    #[test]
    fn test_no_alloc_buffer_segments_message_postfix() {
        let output_segments = OutputSegments::SingleSegment([&[1, 2, 3, 4, 5, 6, 7, 8]]);
        let mut buf = vec![];
        serialize::write_message_segments(&mut buf, &output_segments).unwrap();
        buf.extend_from_slice(&[11, 12, 13, 14, 15, 16]);

        let remaining = &mut buf.as_slice();
        NoAllocSliceSegments::try_new(remaining, ReaderOptions::new()).unwrap();

        // Confirm that slice pointer was advanced to data past first message
        assert_eq!(*remaining, &[11, 12, 13, 14, 15, 16]);
    }

    #[test]
    fn test_no_alloc_buffer_segments_message_invalid() {
        let mut buf = vec![];

        buf.extend([0, 2, 0, 0].iter().cloned()); // 513 segments
        buf.extend([0; 513 * 8].iter().cloned());
        assert!(NoAllocSliceSegments::try_new(&mut &buf[..], ReaderOptions::new()).is_err());
        buf.clear();

        buf.extend([0, 0, 0, 0].iter().cloned()); // 1 segments
        assert!(NoAllocSliceSegments::try_new(&mut &buf[..], ReaderOptions::new()).is_err());
        buf.clear();

        buf.extend([0, 0, 0, 0].iter().cloned()); // 1 segments
        buf.extend([0; 3].iter().cloned());
        assert!(NoAllocSliceSegments::try_new(&mut &buf[..], ReaderOptions::new()).is_err());
        buf.clear();

        buf.extend([255, 255, 255, 255].iter().cloned()); // 0 segments
        assert!(NoAllocSliceSegments::try_new(&mut &buf[..], ReaderOptions::new()).is_err());
        buf.clear();
    }

    quickcheck! {
        fn test_no_alloc_buffer_segments_message_truncated(segments_vec: Vec<Vec<Word>>) -> TestResult {
            if segments_vec.is_empty() { return TestResult::discard() }

            let segments: Vec<_> = segments_vec.iter()
                .map(|s| Word::words_to_bytes(s.as_slice())).collect();

            let output_segments = OutputSegments::MultiSegment(segments.clone());

            let mut msg = vec![];

            serialize::write_message_segments(&mut msg, &output_segments).unwrap();

            // Lop off the final element.
            msg.pop().unwrap();

            let no_alloc_segments =
                NoAllocSliceSegments::try_new(&mut msg.as_slice(), ReaderOptions::new());

            assert!(no_alloc_segments.is_err());
            TestResult::from_bool(true)
        }

        fn test_no_alloc_buffer_segments_message_options_limit(
            segments_vec: Vec<Vec<Word>>) -> TestResult
        {
            let mut word_count = 0;
            let segments: Vec<_> = segments_vec.iter()
                .map(|s| {
                    let ws = Word::words_to_bytes(s.as_slice());
                    word_count += s.len();
                    ws
                }).collect();
            if word_count == 0 { return TestResult::discard() };

            let output_segments = OutputSegments::MultiSegment(segments.clone());

            let mut msg = vec![];

            serialize::write_message_segments(&mut msg, &output_segments).unwrap();

            let mut options = ReaderOptions::new();
            options.traversal_limit_in_words(Some(word_count));

            let _no_alloc_segments =
                NoAllocSliceSegments::try_new(&mut msg.as_slice(), options).unwrap();

            let mut options = ReaderOptions::new();
            options.traversal_limit_in_words(Some(word_count - 1));

            let no_alloc_segments = NoAllocSliceSegments::try_new(&mut msg.as_slice(), options);

            assert!(no_alloc_segments.is_err());
            TestResult::from_bool(true)
        }

        fn test_no_alloc_buffer_segments_bad_alignment(segment_0: Vec<Word>) -> TestResult {
            if segment_0.is_empty() { return TestResult::discard(); }
            let output_segments = OutputSegments::SingleSegment([Word::words_to_bytes(&segment_0)]);

            let mut msg = vec![];

            serialize::write_message_segments(&mut msg, &output_segments).unwrap();
            // mis-align buffer by 1 byte
            msg.insert(0_usize, 0_u8);

            let no_alloc_segments = NoAllocSliceSegments::try_new(&mut &msg[1..], ReaderOptions::new());

            if cfg!(feature = "unaligned") {
                // If we build with "unaligned" feature, alignment requirements should not be enforced
                no_alloc_segments.unwrap();
            } else {
                assert!(no_alloc_segments.is_err());
            }
            TestResult::from_bool(true)
        }
    }
}
