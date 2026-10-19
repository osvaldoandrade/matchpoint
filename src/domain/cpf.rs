//! Local CPF checks. The stored value is always the original field.

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Validation {
    None,
    Length,
    Checksum,
}

impl Validation {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::None => "none",
            Self::Length => "length",
            Self::Checksum => "checksum",
        }
    }

    pub fn parse(value: &str) -> Option<Self> {
        match value {
            "none" => Some(Self::None),
            "length" => Some(Self::Length),
            "checksum" => Some(Self::Checksum),
            _ => None,
        }
    }
}

/// Returns a stable reason when the field must not be sent.
/// Empty fields are always rejected. Other checks follow `mode`.
pub fn validate_cpf(mode: Validation, raw: &str) -> Option<&'static str> {
    if raw.is_empty() {
        return Some("empty");
    }
    match mode {
        Validation::None => None,
        Validation::Length => digits_11(raw).err(),
        Validation::Checksum => match digits_11(raw) {
            Err(reason) => Some(reason),
            Ok(()) if repeated(raw) => Some("repeated_digits"),
            Ok(()) if !checksum_ok(raw) => Some("bad_checksum"),
            Ok(()) => None,
        },
    }
}

fn digits_11(raw: &str) -> Result<(), &'static str> {
    let b = raw.as_bytes();
    if b.len() == 11 && b.iter().all(|c| c.is_ascii_digit()) {
        Ok(())
    } else {
        Err("not_11_digits")
    }
}

fn repeated(raw: &str) -> bool {
    let b = raw.as_bytes();
    b.iter().all(|c| *c == b[0])
}

fn checksum_ok(raw: &str) -> bool {
    let nums: Vec<u8> = raw.bytes().map(|c| c - b'0').collect();
    digit(&nums[..9]) == nums[9] && digit(&nums[..10]) == nums[10]
}

fn digit(nums: &[u8]) -> u8 {
    let mut sum = 0u32;
    for (i, n) in nums.iter().enumerate() {
        sum += u32::from(*n) * (nums.len() + 1 - i) as u32;
    }
    let rem = sum % 11;
    if rem < 2 {
        0
    } else {
        (11 - rem) as u8
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn preserves_policy_table() {
        let cases = [
            (Validation::None, "", Some("empty")),
            (Validation::None, "abc", None),
            (Validation::None, "00012345678", None),
            (Validation::Length, "00012345678", None),
            (Validation::Length, "00000000000", None),
            (Validation::Length, "123", Some("not_11_digits")),
            (Validation::Length, "529.982.247-25", Some("not_11_digits")),
            (Validation::Checksum, "52998224725", None),
            (Validation::Checksum, "39053344705", None),
            (Validation::Checksum, "00000000000", Some("repeated_digits")),
            (Validation::Checksum, "11111111111", Some("repeated_digits")),
            (Validation::Checksum, "12345678901", Some("bad_checksum")),
        ];
        for (mode, raw, want) in cases {
            assert_eq!(validate_cpf(mode, raw), want, "{mode:?} {raw}");
        }
    }
}
