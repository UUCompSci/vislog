use serde::{
    de::{self, Visitor},
    Deserialize, Deserializer, Serialize,
};
use thiserror::Error;

#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub struct Guid {
    inner: [u8; 16],
}

impl std::fmt::Debug for Guid {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // let mut n: u128 = 0;
        // for (i, byte) in self.inner.iter().enumerate() {
        //     let byte = *byte as u128;
        //     n |= byte << (8 * (15 - i as u128))
        // }

        let hex_chars = self
            .inner
            .iter()
            .map(|byte| {
                let first_half = byte >> 4;
                let second_half = byte & 0b00001111;

                [format!("{:X}", first_half), format!("{:X}", second_half)]
            })
            .flatten()
            .fold(String::new(), |mut acc, c| {
                acc.push_str(&c);
                acc
            });

        write!(
            f,
            "{}-{}-{}-{}-{}",
            &hex_chars[..8],
            &hex_chars[8..12],
            &hex_chars[12..16],
            &hex_chars[16..20],
            &hex_chars[20..]
        )
    }
}

impl std::fmt::Display for Guid {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{self:?}")
    }
}

#[derive(Debug, Error, PartialEq, Eq)]
pub enum GUIDParsingError {
    #[error("String provided is too short")]
    TooShort,

    #[error("String provided is too long")]
    TooLong,

    #[error("String contains invalid characters")]
    InvalidCharacter,
}

impl TryFrom<&str> for Guid {
    type Error = GUIDParsingError;

    fn try_from(s: &str) -> Result<Self, Self::Error> {
        if s.len() < 32 {
            return Err(GUIDParsingError::TooShort);
        }

        // The additional 4 chars is to account for the possible '-' characters
        if s.len() > 36 {
            return Err(GUIDParsingError::TooLong);
        }

        let mut chars = s.chars();

        let mut inner = [0u8; 16];

        for i in 0..16 {
            let mut byte = 0u8;
            let mut byte_index = 0;
            while byte_index < 2 {
                if let Some(c) = chars.next() {
                    match c {
                        '-' => continue,
                        _ => {
                            if let Some(n) = hex_to_num(c) {
                                // Result of `byte_index ^ 1` is either 1 or 0 and determines
                                // whether the current `n` gets shifted 4 bits to the left.
                                // `byte_index` should be 1 when `byte_index` == 0 and 0 when
                                // `byte_index` == 1
                                byte |= n << (4 * (byte_index ^ 1));
                                byte_index += 1;
                            } else {
                                return Err(GUIDParsingError::InvalidCharacter);
                            }
                        }
                    }
                } else {
                    return Err(GUIDParsingError::TooShort);
                }
            }

            inner[i] = byte;
        }

        Ok(Self { inner })
    }
}

impl Serialize for Guid {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(self.to_string().as_str())
    }
}

// TODO: Implement deserialization for byte arrays and u128 integers
impl<'de> Deserialize<'de> for Guid {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        struct GuidVisitor;

        impl<'de> Visitor<'de> for GuidVisitor {
            type Value = Guid;

            fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
                write!(formatter, "a string representing a Guid/Uuid")
            }

            fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
            where
                E: serde::de::Error,
            {
                Guid::try_from(v).map_err(|e| de::Error::custom(e))
            }
        }

        deserializer.deserialize_any(GuidVisitor)
    }
}

const ASCII_NUMS_START: u32 = 48;
const ASCII_UPPER_ALPHA_START: u32 = 65;
const ASCII_LOWER_ALPHA_START: u32 = 97;

fn hex_to_num(c: char) -> Option<u8> {
    if c as u32 > 127 {
        return None;
    }

    let n = match c {
        '0'..='9' => c as u32 - ASCII_NUMS_START,
        'a'..='f' => c as u32 - ASCII_LOWER_ALPHA_START + 10,
        'A'..='F' => c as u32 - ASCII_UPPER_ALPHA_START + 10,
        _ => return None,
    };

    Some(n as u8)
}

pub(crate) fn deserialize_guid_with_curly_braces<'de, D>(deserializer: D) -> Result<Guid, D::Error>
where
    D: Deserializer<'de>,
{
    let mut s: &str = Deserialize::deserialize(deserializer)?;

    // Ommit the curly braces in the source when parsing
    s = &s[1..s.len() - 1];

    Guid::try_from(s).map_err(serde::de::Error::custom)
}

#[cfg(test)]
mod test {
    use uuid::uuid;

    use super::*;

    #[test]
    fn hex_to_num_ascii_nums() {
        assert_eq!(hex_to_num('0'), Some(0));
        assert_eq!(hex_to_num('5'), Some(5));
        assert_eq!(hex_to_num('9'), Some(9));
    }

    #[test]
    fn hex_to_num_ascii_lower() {
        assert_eq!(hex_to_num('a'), Some(10));
        assert_eq!(hex_to_num('d'), Some(13));
        assert_eq!(hex_to_num('f'), Some(15));
    }

    #[test]
    fn hex_to_num_ascii_upper() {
        assert_eq!(hex_to_num('A'), Some(10));
        assert_eq!(hex_to_num('D'), Some(13));
        assert_eq!(hex_to_num('F'), Some(15));
    }

    #[test]
    fn hex_to_num_invalid_chars() {
        // All ascii chars that are printable
        // NOTE: This is by no means a comprehensive test. This is only used to show that the
        // function `hex_to_num` rejects invalid `char`s
        let invalid_char_iter = ('!'..='/')
            .chain(':'..='@')
            .chain('['..='`')
            .chain('{'..='~');

        invalid_char_iter.for_each(|c| assert_eq!(hex_to_num(c), None));
    }

    #[test]
    fn parse_guid_from_str_with_hyphens() {
        let s = "C7AD875E-1344-4D9B-A883-32E748890908";
        let guid = Guid::try_from(s).expect("Failed to parse GUID");

        let expected = Guid {
            inner: [
                0xC7, 0xAD, 0x87, 0x5E, 0x13, 0x44, 0x4D, 0x9B, 0xA8, 0x83, 0x32, 0xE7, 0x48, 0x89,
                0x09, 0x08,
            ],
        };

        assert_eq!(guid, expected);
    }

    #[test]
    fn parse_guid_from_str_without_hyphens() {
        let s = "C7AD875E13444D9BA88332E748890908";
        let guid = Guid::try_from(s).expect("Failed to parse GUID");

        let expected = Guid {
            inner: [
                0xC7, 0xAD, 0x87, 0x5E, 0x13, 0x44, 0x4D, 0x9B, 0xA8, 0x83, 0x32, 0xE7, 0x48, 0x89,
                0x09, 0x08,
            ],
        };

        assert_eq!(guid, expected);
    }

    #[test]
    fn error_when_parse_guid_from_str_when_too_long() {
        let s = "C7AD875E-1344-4D9B-A883-32E748890908-123321123";

        assert_eq!(Guid::try_from(s), Err(GUIDParsingError::TooLong));
    }

    #[test]
    fn error_when_parse_guid_from_str_when_too_short() {
        let s = "C7AD875E-1344-4D9B-A883";

        assert_eq!(Guid::try_from(s), Err(GUIDParsingError::TooShort));
    }

    #[test]
    fn error_when_parse_guid_from_str_with_invalid_char() {
        let s = "+7AD875E-1344-4D9B-A883-32E748890908";

        assert_eq!(Guid::try_from(s), Err(GUIDParsingError::InvalidCharacter));
    }

    #[test]
    fn parse_guid_then_back_to_str() {
        let s = "C7AD875E-1344-4D9B-A883-32E748890908";
        let guid = Guid::try_from(s).expect("Failed to parse GUID");

        assert_eq!(guid.to_string(), s);
    }

    #[test]
    fn parse_and_display_guid_starting_with_0() {
        let s = "08DD69D3-9F67-4A81-A5AA-5738B6A79D2B";
        let guid = Guid::try_from(s).unwrap();

        assert_eq!(guid.to_string(), s);
    }

    #[test]
    fn equivalent_results_with_uuid() {
        let uuid = uuid!("08DD69D3-9F67-4A81-A5AA-5738B6A79D2B");

        let guid = Guid::try_from("08DD69D3-9F67-4A81-A5AA-5738B6A79D2B").unwrap();

        assert_eq!(uuid.to_string().to_uppercase(), guid.to_string());
    }
}
