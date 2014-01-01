use std;
use capnp;
use zmq;


pub fn slice_cast<'a, T, V>(s : &'a [T]) -> &'a [V] {
    unsafe {
        std::cast::transmute(
            std::unstable::raw::Slice {data : s.unsafe_ref(0),
                                       len : s.len() * std::mem::size_of::<T>() / std::mem::size_of::<V>()  })
    }
}


pub fn frames_to_segments<'a>(frames : &'a [zmq::Message] ) -> ~ [&'a [capnp::common::Word]] {

    let mut result : ~ [&'a [capnp::common::Word]] = box [];

    for frame in frames.iter() {
        unsafe {
            let slice = frame.with_bytes(|v|
                    std::unstable::raw::Slice { data : v.unsafe_ref(0),
                                                len : v.len() / 8 } );
            result.push(std::cast::transmute(slice));
        }
    }

    return result;
}
