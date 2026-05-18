pub(crate) fn decode_base64(input: &str) -> Result<Vec<u8>, String> {
    let encoded = input
        .split_once(',')
        .filter(|(prefix, _)| prefix.starts_with("data:") && prefix.ends_with(";base64"))
        .map(|(_, body)| body)
        .unwrap_or(input);
    let clean = encoded
        .bytes()
        .filter(|byte| !byte.is_ascii_whitespace())
        .collect::<Vec<_>>();
    if clean.len() % 4 == 1 {
        return Err("invalid base64 length".to_string());
    }

    let mut output = Vec::with_capacity(clean.len().saturating_mul(3) / 4);
    for chunk in clean.chunks(4) {
        let mut values = [0_u8; 4];
        let mut padding = 0;
        for (index, byte) in chunk.iter().copied().enumerate() {
            if byte == b'=' {
                padding += 1;
                values[index] = 0;
            } else {
                values[index] = decode_base64_byte(byte)?;
            }
        }
        if chunk.len() < 4 {
            padding += 4 - chunk.len();
        }
        let triple = ((values[0] as u32) << 18)
            | ((values[1] as u32) << 12)
            | ((values[2] as u32) << 6)
            | values[3] as u32;
        output.push(((triple >> 16) & 0xff) as u8);
        if padding < 2 {
            output.push(((triple >> 8) & 0xff) as u8);
        }
        if padding == 0 {
            output.push((triple & 0xff) as u8);
        }
    }
    Ok(output)
}

fn decode_base64_byte(byte: u8) -> Result<u8, String> {
    match byte {
        b'A'..=b'Z' => Ok(byte - b'A'),
        b'a'..=b'z' => Ok(byte - b'a' + 26),
        b'0'..=b'9' => Ok(byte - b'0' + 52),
        b'+' => Ok(62),
        b'/' => Ok(63),
        _ => Err(format!("invalid base64 byte 0x{byte:02x}")),
    }
}

#[cfg(test)]
mod tests {
    use super::decode_base64;

    #[test]
    fn decode_base64_accepts_plain_and_data_url_inputs() {
        assert_eq!(decode_base64("aGVsbG8=").expect("plain base64"), b"hello");
        assert_eq!(
            decode_base64("data:image/png;base64,aGVsbG8=").expect("data url base64"),
            b"hello"
        );
    }

    #[test]
    fn decode_base64_rejects_invalid_shape() {
        assert!(decode_base64("abcde").is_err());
        assert!(decode_base64("****").is_err());
    }
}
