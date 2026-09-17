use std::{collections::BTreeSet, io};

pub(super) fn parse(value: &str, word_bits: u32) -> io::Result<BTreeSet<u16>> {
    let invalid = || {
        io::Error::new(
            io::ErrorKind::InvalidData,
            "invalid kernel capability bitmap",
        )
    };
    if !matches!(word_bits, 32 | 64) {
        return Err(invalid());
    }
    let words: Vec<_> = value.split_whitespace().collect();
    if words.is_empty() || words.len() > 65536 / word_bits as usize {
        return Err(invalid());
    }
    let mut codes = BTreeSet::new();
    for (index, word) in words.into_iter().rev().enumerate() {
        if !word.chars().all(|c| c.is_ascii_hexdigit()) {
            return Err(invalid());
        }
        let bits = u64::from_str_radix(word, 16)
            .map_err(|error| io::Error::new(io::ErrorKind::InvalidData, error))?;
        if word_bits == 32 && bits > u64::from(u32::MAX) {
            return Err(invalid());
        }
        for bit in 0..word_bits {
            if bits & (1_u64 << bit) != 0 {
                codes.insert(
                    u16::try_from(index * word_bits as usize + bit as usize)
                        .map_err(|error| io::Error::new(io::ErrorKind::InvalidData, error))?,
                );
            }
        }
    }
    Ok(codes)
}

#[cfg(test)]
mod tests {
    use super::parse;

    #[test]
    fn sparse_high_codes_preserve_native_numbering() {
        let mut words = ["0"; 12];
        words[0] = "1"; // BTN_TRIGGER_HAPPY1 (0x2c0), on a 64-bit kernel.
        words[7] = "100000000"; // BTN_JOYSTICK (0x120).
        let bits = parse(&words.join(" "), 64).expect("valid bitmap");
        assert_eq!(bits.into_iter().collect::<Vec<_>>(), [0x120, 0x2c0]);
        assert_eq!(parse("1 0", 32).expect("32-bit words"), [32].into());
    }

    #[test]
    fn empty_malformed_and_overflow_are_errors_not_empty_capabilities() {
        for value in ["", " ", "gg", "-1", "10000000000000000"] {
            assert!(parse(value, 64).is_err(), "{value:?}");
        }
        parse("100000000", 32).expect_err("32-bit overflow");
        parse("1", 8).expect_err("unsupported word width");
        parse(&vec!["0"; 1025].join(" "), 64).expect_err("oversized bitmap");
        assert!(parse("0\n", 64).expect("empty set is valid").is_empty());
    }
}
