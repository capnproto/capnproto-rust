// We don't want to pull in base64 crate just for this. So hand-rolling a
// base64 codec.
pub mod base64 {
    const BASE64_CHARS: &[u8; 64] =
        b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";

    pub fn encode(data: &[u8]) -> String {
        let mut encoded = String::with_capacity(data.len().div_ceil(3) * 4);
        for chunk in data.chunks(3) {
            #[allow(clippy::get_first)]
            let b0 = chunk.get(0).copied().unwrap_or(0);
            let b1 = chunk.get(1).copied().unwrap_or(0);
            let b2 = chunk.get(2).copied().unwrap_or(0);
            let n = ((b0 as u32) << 16) | ((b1 as u32) << 8) | (b2 as u32);
            let c0 = BASE64_CHARS[((n >> 18) & 0x3F) as usize];
            let c1 = BASE64_CHARS[((n >> 12) & 0x3F) as usize];
            let c2 = if chunk.len() > 1 {
                BASE64_CHARS[((n >> 6) & 0x3F) as usize]
            } else {
                b'='
            };
            let c3 = if chunk.len() > 2 {
                BASE64_CHARS[(n & 0x3F) as usize]
            } else {
                b'='
            };
            encoded.push(c0 as char);
            encoded.push(c1 as char);
            encoded.push(c2 as char);
            encoded.push(c3 as char);
        }
        encoded
    }

    pub fn decode(data: &str) -> capnp::Result<Vec<u8>> {
        let bytes = data.as_bytes();
        if !bytes.len().is_multiple_of(4) {
            return Err(capnp::Error::failed(
                "Base64 string length must be a multiple of 4".into(),
            ));
        }
        let mut decoded = Vec::with_capacity(bytes.len() / 4 * 3);
        for chunk in bytes.chunks(4) {
            let mut n: u32 = 0;
            let mut padding = 0;
            for &c in chunk {
                n <<= 6;
                match c {
                    b'A'..=b'Z' => n |= (c - b'A') as u32,
                    b'a'..=b'z' => n |= (c - b'a' + 26) as u32,
                    b'0'..=b'9' => n |= (c - b'0' + 52) as u32,
                    b'+' => n |= 62,
                    b'/' => n |= 63,
                    b'=' => {
                        n |= 0;
                        padding += 1;
                    }
                    _ => {
                        return Err(capnp::Error::failed(format!(
                            "Invalid base64 character: {}",
                            c as char
                        )));
                    }
                }
            }
            decoded.push(((n >> 16) & 0xFF) as u8);
            if padding < 2 {
                decoded.push(((n >> 8) & 0xFF) as u8);
            }
            if padding < 1 {
                decoded.push((n & 0xFF) as u8);
            }
        }
        Ok(decoded)
    }
}

// We don't want to pull in hex crate just for this. So hand-rolling a
// hex codec.
pub mod hex {
    const HEX_CHARS: &[u8; 16] = b"0123456789abcdef";
    fn hex_char_to_value(c: u8) -> capnp::Result<u8> {
        match c {
            b'0'..=b'9' => Ok(c - b'0'),
            b'a'..=b'f' => Ok(c - b'a' + 10),
            b'A'..=b'F' => Ok(c - b'A' + 10),
            _ => Err(capnp::Error::failed(format!(
                "Invalid hex character: {}",
                c as char
            ))),
        }
    }

    pub fn encode(data: &[u8]) -> String {
        let mut encoded = String::with_capacity(data.len() * 2);
        for &byte in data {
            let high = HEX_CHARS[(byte >> 4) as usize];
            let low = HEX_CHARS[(byte & 0x0F) as usize];
            encoded.push(high as char);
            encoded.push(low as char);
        }
        encoded
    }

    pub fn decode(data: &str) -> capnp::Result<Vec<u8>> {
        if !data.len().is_multiple_of(2) {
            return Err(capnp::Error::failed(
                "Hex string must have even length".into(),
            ));
        }
        let mut decoded = Vec::with_capacity(data.len() / 2);
        let bytes = data.as_bytes();
        for i in (0..data.len()).step_by(2) {
            let high = hex_char_to_value(bytes[i])?;
            let low = hex_char_to_value(bytes[i + 1])?;
            decoded.push((high << 4) | low);
        }
        Ok(decoded)
    }
}
