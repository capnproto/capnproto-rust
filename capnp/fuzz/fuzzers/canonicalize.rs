#![no_main]

use capnp::{message, serialize};
use libfuzzer_sys::fuzz_target;

fn try_go(mut data: &[u8]) -> ::capnp::Result<()> {
    let orig_data = data;
    let message = serialize::read_message(&mut data, Default::default())?;
    assert!(orig_data.len() > data.len());
    let bytes_consumed = orig_data.len() - data.len();
    let maybe_is_canonical = message.is_canonical();
    let canonical_words = message.canonicalize()?;

    if let Ok(true) = maybe_is_canonical {
        assert_eq!(
            &orig_data[8..bytes_consumed],
            capnp::Word::words_to_bytes(&canonical_words[..])
        );
    }

    let segments = &[capnp::Word::words_to_bytes(&canonical_words[..])];
    let segment_array = message::SegmentArray::new(segments);
    let canonical_message = message::Reader::new(segment_array, Default::default());
    assert!(canonical_message.is_canonical()?);

    let canonical2_words = canonical_message.canonicalize()?;
    assert_eq!(canonical_words, canonical2_words);
    Ok(())
}

fuzz_target!(|data: &[u8]| {
    let _ = try_go(data);
});
