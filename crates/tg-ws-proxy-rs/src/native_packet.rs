//! Native MTProto transport framing used by Telegram's HTTP endpoint.
use crate::crypto::ProtoTag;
use bytes::Bytes;

pub const MAX_PACKET: usize = 4 * 1024 * 1024;
pub const MAX_PADDING: usize = 256;

pub fn header_len(tag: ProtoTag, first: u8) -> usize {
    if tag == ProtoTag::Abridged && first & 0x7f != 0x7f {
        1
    } else {
        4
    }
}

pub fn packet_len(tag: ProtoTag, header: &[u8]) -> Result<usize, String> {
    if header.is_empty() {
        return Err("empty native header".into());
    }
    let length = if tag == ProtoTag::Abridged {
        if header[0] & 0x7f == 0x7f {
            if header.len() != 4 {
                return Err("incomplete abridged header".into());
            }
            u32::from_le_bytes([header[1], header[2], header[3], 0]) as usize * 4
        } else {
            (header[0] & 0x7f) as usize * 4
        }
    } else {
        let bytes: [u8; 4] = header.try_into().map_err(|_| "incomplete native header")?;
        (u32::from_le_bytes(bytes) & 0x7fffffff) as usize
    };
    let extra = if tag == ProtoTag::PaddedIntermediate {
        MAX_PADDING
    } else {
        0
    };
    if !(24..=MAX_PACKET + extra).contains(&length) {
        return Err("native packet exceeds supported size".into());
    }
    Ok(length)
}

pub fn unpad(tag: ProtoTag, mut body: Vec<u8>) -> Result<Bytes, String> {
    if tag == ProtoTag::PaddedIntermediate {
        if body.len() < 24 {
            return Err("short padded packet".into());
        }
        let plain = body[..8] == [0; 8];
        let length = if plain {
            20usize
                .checked_add(u32::from_le_bytes(body[16..20].try_into().unwrap()) as usize)
                .ok_or("native length overflow")?
        } else {
            24 + (body.len() - 24) / 16 * 16
        };
        let padding = body
            .len()
            .checked_sub(length)
            .ok_or("invalid native padding")?;
        if length < 24 || padding > if plain { MAX_PADDING } else { 15 } {
            return Err("invalid native padding".into());
        }
        body.truncate(length);
    }
    if body.len() > MAX_PACKET || !body.len().is_multiple_of(4) {
        return Err("invalid native alignment".into());
    }
    Ok(Bytes::from(body))
}

/// Transport errors are four-byte signed bodies and use the same framing.
pub fn frame(tag: ProtoTag, body: &[u8]) -> Result<Vec<u8>, String> {
    let (mut result, padding) = frame_header(tag, body.len())?;
    result.reserve(body.len() + padding);
    result.extend_from_slice(body);
    result.extend((0..padding).map(|_| rand::random::<u8>()));
    Ok(result)
}

/// Header and padding length for streaming a reply without copying its body.
pub fn frame_header(tag: ProtoTag, length: usize) -> Result<(Vec<u8>, usize), String> {
    if length == 0 || length > MAX_PACKET || !length.is_multiple_of(4) {
        return Err("invalid HTTP MTProto response".into());
    }
    let padding = if tag == ProtoTag::PaddedIntermediate {
        (rand::random::<u8>() % 4) as usize
    } else {
        0
    };
    let mut result = Vec::with_capacity(4);
    if tag == ProtoTag::Abridged {
        let words = length / 4;
        if words < 127 {
            result.push(words as u8);
        } else {
            result.push(0x7f);
            result.extend_from_slice(&(words as u32).to_le_bytes()[..3]);
        }
    } else {
        result.extend_from_slice(&((length + padding) as u32).to_le_bytes());
    }
    Ok((result, padding))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn native_framing_roundtrip_and_boundaries() {
        for tag in [
            ProtoTag::Abridged,
            ProtoTag::Intermediate,
            ProtoTag::PaddedIntermediate,
        ] {
            for size in [24, 508, 512, MAX_PACKET] {
                let mut body = vec![1; size];
                // Padded encrypted packets have a 24-byte prefix then AES blocks.
                if tag == ProtoTag::PaddedIntermediate {
                    body.resize(24 + (size - 24) / 16 * 16, 1);
                }
                let wire = frame(tag, &body).unwrap();
                let n = header_len(tag, wire[0]);
                assert_eq!(packet_len(tag, &wire[..n]).unwrap(), wire.len() - n);
                assert_eq!(&unpad(tag, wire[n..].to_vec()).unwrap()[..], body);
            }
        }
    }
    #[test]
    fn permits_256_bytes_padding_for_plain_auth_packets() {
        let mut body = vec![0; 280];
        body[16..20].copy_from_slice(&4u32.to_le_bytes());
        assert_eq!(unpad(ProtoTag::PaddedIntermediate, body).unwrap().len(), 24);
        assert!(packet_len(ProtoTag::Intermediate, &u32::MAX.to_le_bytes()).is_err());
        assert!(unpad(ProtoTag::PaddedIntermediate, vec![0; 24]).is_err());
    }
}
