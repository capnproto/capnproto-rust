use std;
use capnp;
use zmq;


fn slice_cast<'a, T, V>(s : &'a [T]) -> &'a [V] {
    unsafe {
        std::mem::transmute(
            std::raw::Slice {data : s.as_ptr(),
                             len : s.len() * std::mem::size_of::<T>() / std::mem::size_of::<V>()  })
    }
}


pub fn frames_to_segments<'a>(frames : &'a [zmq::Message] ) -> Vec<&'a [capnp::common::Word]> {

    let mut result : Vec<&'a [capnp::common::Word]> = Vec::new();

    for frame in frames.iter() {
        unsafe {
            let slice = frame.with_bytes(|v|
                    std::raw::Slice { data : v.as_ptr(),
                                      len : v.len() / 8 });

            // TODO check whether bytes are aligned on a word boundary.
            // If not, copy them into a new buffer. Who will own that buffer?

            result.push(std::mem::transmute(slice));
        }
    }

    return result;
}

pub fn recv(socket : &mut zmq::Socket) -> Result<Vec<zmq::Message>, zmq::Error> {
    let mut frames = Vec::new();
    loop {
        match socket.recv_msg(0) {
            Ok(m) => frames.push(m),
            Err(e) => return Err(e)
        }
        match socket.get_rcvmore() {
            Ok(true) => (),
            Ok(false) => return Ok(frames),
            Err(e) => return Err(e)
        }
    }
}

pub fn send<U:capnp::message::MessageBuilder>(
    socket : &mut zmq::Socket, message : & U)
                  -> Result<(), zmq::Error>{

    message.get_segments_for_output(|segments| {
            for ii in range(0, segments.len()) {
                let flags = if ii == segments.len() - 1 { 0 } else { zmq::SNDMORE };
                match socket.send(slice_cast(segments[ii]), flags) {
                    Ok(_) => {}
                    Err(_) => {fail!();} // XXX
                }
            }
        });

    Ok(())
}
