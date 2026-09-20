//! Checked source Bytes. Canonical JSON is lowercase hex, never a numeric array.
use serde::{Deserialize, Deserializer, Serialize, Serializer, de};
use std::fmt;
pub const MAX_BYTES: usize = 256 * 1024;
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Bytes(Vec<u8>);
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BytesError {
    Literal,
    Budget,
}
impl BytesError {
    pub fn code(self) -> &'static str {
        match self {
            Self::Literal => "E_BYTES_LITERAL",
            Self::Budget => "E_BYTES_BUDGET",
        }
    }
}
impl fmt::Display for BytesError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.write_str(match self {
            Self::Literal => "Bytes require even ASCII hexadecimal digits",
            Self::Budget => "Bytes exceed the 256 KiB value bound",
        })
    }
}
impl std::error::Error for BytesError {}
impl Bytes {
    pub fn new(bytes: Vec<u8>) -> Result<Self, BytesError> {
        if bytes.len() > MAX_BYTES {
            Err(BytesError::Budget)
        } else {
            Ok(Self(bytes))
        }
    }
    pub fn from_hex(hex: &str) -> Result<Self, BytesError> {
        if hex.len() > MAX_BYTES * 2 {
            return Err(BytesError::Budget);
        }
        if !hex.len().is_multiple_of(2) || !hex.bytes().all(|b| b.is_ascii_hexdigit()) {
            return Err(BytesError::Literal);
        }
        let mut bytes = Vec::new();
        bytes
            .try_reserve_exact(hex.len() / 2)
            .map_err(|_| BytesError::Budget)?;
        let digit = |b: u8| {
            if b.is_ascii_digit() {
                b - b'0'
            } else {
                b.to_ascii_lowercase() - b'a' + 10
            }
        };
        for &[high, low] in hex.as_bytes().as_chunks::<2>().0 {
            bytes.push(digit(high) * 16 + digit(low));
        }
        Ok(Self(bytes))
    }
    pub fn as_slice(&self) -> &[u8] {
        &self.0
    }
    pub fn len(&self) -> usize {
        self.0.len()
    }
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }
    pub fn to_hex(&self) -> String {
        let mut hex = String::with_capacity(self.len() * 2);
        for b in &self.0 {
            hex.push(b"0123456789abcdef"[(b >> 4) as usize] as char);
            hex.push(b"0123456789abcdef"[(b & 15) as usize] as char);
        }
        hex
    }
    pub fn concat(&self, other: &Self) -> Result<Self, BytesError> {
        let size = self
            .len()
            .checked_add(other.len())
            .filter(|n| *n <= MAX_BYTES)
            .ok_or(BytesError::Budget)?;
        let mut bytes = Vec::new();
        bytes
            .try_reserve_exact(size)
            .map_err(|_| BytesError::Budget)?;
        bytes.extend_from_slice(&self.0);
        bytes.extend_from_slice(&other.0);
        Ok(Self(bytes))
    }
    pub(crate) fn size(&self) -> usize {
        self.len() * 2 + 128
    }
}
impl Serialize for Bytes {
    fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        s.serialize_str(&self.to_hex())
    }
}
impl<'de> Deserialize<'de> for Bytes {
    fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        struct Visitor;
        impl de::Visitor<'_> for Visitor {
            type Value = Bytes;
            fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
                f.write_str("bounded hexadecimal Bytes")
            }
            fn visit_str<E: de::Error>(self, v: &str) -> Result<Bytes, E> {
                Bytes::from_hex(v).map_err(E::custom)
            }
        }
        d.deserialize_str(Visitor)
    }
}
