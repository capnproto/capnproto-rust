use std::io::{Read};

enum Header {

    Incomplete {
        num_segments: Option<usize>,
        segment_slices: Vec<(usize, usize)>,
        partial_read: 
    }
    Complete {
        total_words: usize,
        segment_slices: Vec<(usize, usize)>
    }
}

struct MessageReader<R> where R: Read {
    read: R,

    frame: Option<(Option<usize>, Vec<(usize, usize)> 



}
