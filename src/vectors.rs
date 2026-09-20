//! Checked source vectors over the exact native Space/Coordinates descriptor profile.
use serde::{Deserialize, Deserializer, Serialize, Serializer, de, ser::SerializeStruct};
use std::{cell::Cell, fmt};
pub use weave_spaces::{Geometry, Metric, Space, Unit, VectorRole};

pub const MAX_DIMENSIONS: usize = 4096;
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct VectorError(pub &'static str);
impl fmt::Display for VectorError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.0)
    }
}
impl std::error::Error for VectorError {}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct VectorDescriptor {
    space: Space,
    role: VectorRole,
}
impl VectorDescriptor {
    pub fn new(space: Space, role: VectorRole) -> Result<Self, VectorError> {
        space
            .validate()
            .map_err(|_| VectorError("E_VECTOR_DESCRIPTOR"))?;
        let mut strings = vec![space.id.as_str(), space.revision.as_str()];
        match &space.geometry {
            Geometry::Physical3d { frame, .. } => strings.push(frame),
            Geometry::Embedding {
                encoder,
                preprocessing,
                ..
            } => {
                strings.push(encoder);
                strings.push(preprocessing);
            }
        }
        if strings.iter().any(|s| s.len() > 512) {
            return Err(VectorError("E_VECTOR_DESCRIPTOR"));
        }
        if !matches!(
            (&space.geometry, role),
            (
                Geometry::Physical3d { .. },
                VectorRole::Position | VectorRole::Direction
            ) | (Geometry::Embedding { .. }, VectorRole::Embedding)
        ) {
            return Err(VectorError("E_VECTOR_DESCRIPTOR"));
        }
        Ok(Self { space, role })
    }
    pub fn space(&self) -> &Space {
        &self.space
    }
    pub fn role(&self) -> VectorRole {
        self.role
    }
    pub fn dimensions(&self) -> usize {
        match self.space.geometry {
            Geometry::Physical3d { .. } => 3,
            Geometry::Embedding { dimensions, .. } => dimensions,
        }
    }
    pub(crate) fn size(&self) -> usize {
        256 + self.space.id.len()
            + self.space.revision.len()
            + match &self.space.geometry {
                Geometry::Physical3d { frame, .. } => frame.len(),
                Geometry::Embedding {
                    encoder,
                    preprocessing,
                    ..
                } => encoder.len() + preprocessing.len(),
            }
    }
}
#[derive(Clone, Debug, PartialEq)]
pub struct CheckedVector {
    descriptor: VectorDescriptor,
    values: Vec<f64>,
}
// Constructors and strict deserialization admit only finite components and normalize
// signed zero. No mutable access or unchecked conversion can introduce NaN.
impl Eq for CheckedVector {}
impl CheckedVector {
    pub fn new(descriptor: VectorDescriptor, mut values: Vec<f64>) -> Result<Self, VectorError> {
        if values.len() != descriptor.dimensions() {
            return Err(VectorError("E_VECTOR_DIMENSION"));
        }
        if values.iter().any(|v| !v.is_finite()) {
            return Err(VectorError("E_VECTOR_VALUE"));
        }
        for value in &mut values {
            if *value == 0.0 {
                *value = 0.0;
            }
        }
        // The source profile has already prechecked count/finite/descriptor bounds.
        // Calling the canonical pure validator needs no Evidence or host authority.
        let coordinates = weave_spaces::Coordinates {
            space: descriptor.space.clone(),
            role: descriptor.role,
            values,
        };
        coordinates
            .validate()
            .map_err(|_| VectorError("E_VECTOR_VALUE"))?;
        Ok(Self {
            descriptor,
            values: coordinates.values,
        })
    }
    pub fn descriptor(&self) -> &VectorDescriptor {
        &self.descriptor
    }
    pub fn values(&self) -> &[f64] {
        &self.values
    }
    pub(crate) fn size(&self) -> usize {
        self.descriptor.size() + self.values.len() * 32 + 64
    }
}
impl Serialize for CheckedVector {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let mut s = serializer.serialize_struct("CheckedVector", 3)?;
        s.serialize_field("space", &self.descriptor.space)?;
        s.serialize_field("role", &self.descriptor.role)?;
        s.serialize_field("values", &self.values)?;
        s.end()
    }
}

// Strict duplicate-aware JSON decoding precedes canonical DTO interpretation.
// Internal-tag serde decoding or Value parsing alone could erase duplicate keys.
struct Strict(serde_json::Value);
struct Excess;
impl<'de> de::DeserializeSeed<'de> for Excess {
    type Value = ();
    fn deserialize<D: Deserializer<'de>>(self, _d: D) -> Result<(), D::Error> {
        Err(de::Error::custom("E_VECTOR_BUDGET"))
    }
}
struct Seed<'a> {
    bytes: &'a Cell<usize>,
    depth: usize,
}
impl Seed<'_> {
    fn charge<E: de::Error>(&self, n: usize) -> Result<(), E> {
        let next = self.bytes.get().saturating_add(n);
        if next > 1_048_576 || self.depth > 8 {
            return Err(E::custom("E_VECTOR_BUDGET"));
        }
        self.bytes.set(next);
        Ok(())
    }
}
impl<'de> de::DeserializeSeed<'de> for Seed<'_> {
    type Value = serde_json::Value;
    fn deserialize<D: Deserializer<'de>>(self, d: D) -> Result<Self::Value, D::Error> {
        self.charge::<D::Error>(32)?;
        d.deserialize_any(self)
    }
}
impl<'de> de::Visitor<'de> for Seed<'_> {
    type Value = serde_json::Value;
    fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.write_str("bounded strict vector JSON")
    }
    fn visit_bool<E: de::Error>(self, v: bool) -> Result<Self::Value, E> {
        Ok(v.into())
    }
    fn visit_i64<E: de::Error>(self, v: i64) -> Result<Self::Value, E> {
        Ok(v.into())
    }
    fn visit_u64<E: de::Error>(self, v: u64) -> Result<Self::Value, E> {
        Ok(v.into())
    }
    fn visit_f64<E: de::Error>(self, v: f64) -> Result<Self::Value, E> {
        serde_json::Number::from_f64(v)
            .map(Into::into)
            .ok_or_else(|| E::custom("E_VECTOR_VALUE"))
    }
    fn visit_str<E: de::Error>(self, v: &str) -> Result<Self::Value, E> {
        self.charge::<E>(v.len())?;
        Ok(v.into())
    }
    fn visit_string<E: de::Error>(self, v: String) -> Result<Self::Value, E> {
        self.charge::<E>(v.len())?;
        Ok(v.into())
    }
    fn visit_unit<E: de::Error>(self) -> Result<Self::Value, E> {
        Ok(serde_json::Value::Null)
    }
    fn visit_seq<A: de::SeqAccess<'de>>(self, mut seq: A) -> Result<Self::Value, A::Error> {
        let mut values = Vec::new();
        loop {
            if values.len() == MAX_DIMENSIONS {
                // Detect sequence end without traversing an excess nested value.
                seq.next_element_seed(Excess)?;
                break;
            }
            let Some(v) = seq.next_element_seed(Seed {
                bytes: self.bytes,
                depth: self.depth + 1,
            })?
            else {
                break;
            };
            values.push(v);
        }
        Ok(serde_json::Value::Array(values))
    }
    fn visit_map<A: de::MapAccess<'de>>(self, mut map: A) -> Result<Self::Value, A::Error> {
        let mut values = serde_json::Map::new();
        while let Some(key) = map.next_key::<String>()? {
            self.charge::<A::Error>(key.len() + 32)?;
            if values.len() >= 16 || values.contains_key(&key) {
                return Err(de::Error::custom(
                    "duplicate or excessive vector descriptor fields",
                ));
            }
            let value = map.next_value_seed(Seed {
                bytes: self.bytes,
                depth: self.depth + 1,
            })?;
            values.insert(key, value);
        }
        Ok(serde_json::Value::Object(values))
    }
}
impl<'de> Deserialize<'de> for Strict {
    fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        use de::DeserializeSeed;
        Seed {
            bytes: &Cell::new(0),
            depth: 0,
        }
        .deserialize(d)
        .map(Self)
    }
}
impl<'de> Deserialize<'de> for VectorDescriptor {
    fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        #[derive(Deserialize)]
        #[serde(deny_unknown_fields)]
        struct Raw {
            space: Space,
            role: VectorRole,
        }
        let raw: Raw =
            serde_json::from_value(Strict::deserialize(d)?.0).map_err(de::Error::custom)?;
        Self::new(raw.space, raw.role).map_err(de::Error::custom)
    }
}
impl<'de> Deserialize<'de> for CheckedVector {
    fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        #[derive(Deserialize)]
        #[serde(deny_unknown_fields)]
        struct Raw {
            space: Space,
            role: VectorRole,
            values: Vec<f64>,
        }
        let raw: Raw =
            serde_json::from_value(Strict::deserialize(d)?.0).map_err(de::Error::custom)?;
        let descriptor = VectorDescriptor::new(raw.space, raw.role).map_err(de::Error::custom)?;
        Self::new(descriptor, raw.values).map_err(de::Error::custom)
    }
}
