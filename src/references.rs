//! Exact source identity values. These name objects; they do not resolve or authorize them.
use serde::{Deserialize, Deserializer, Serialize, de};
use std::fmt;
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ReferenceError;
impl ReferenceError {
    pub fn code(self) -> &'static str {
        "E_REFERENCE_VALUE"
    }
}
impl fmt::Display for ReferenceError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.write_str("Reference identities require nonempty strings of at most 512 UTF-8 bytes")
    }
}
impl std::error::Error for ReferenceError {}
fn valid(s: &str) -> bool {
    !s.is_empty() && s.len() <= 512
}
// Check borrowed decoded strings before retaining a copy. JSON escape scratch
// buffers are bounded by the caller's outer input envelope (8 MiB in the SDK).
struct Identity(String);
impl<'de> Deserialize<'de> for Identity {
    fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        struct Visitor;
        impl de::Visitor<'_> for Visitor {
            type Value = Identity;
            fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
                f.write_str("nonempty bounded reference identity")
            }
            fn visit_str<E: de::Error>(self, value: &str) -> Result<Identity, E> {
                if !valid(value) {
                    return Err(E::custom(ReferenceError));
                }
                Ok(Identity(value.to_owned()))
            }
        }
        d.deserialize_str(Visitor)
    }
}
macro_rules! checked {
    ($name:ident,$dto:ident,[$($field:ident),+])=>{
        #[derive(Clone,Debug,PartialEq,Eq,Serialize)]
        #[serde(transparent)]
        pub struct $name(weave_contract::$dto);
        impl $name {
            pub fn new(value:weave_contract::$dto)->Result<Self,ReferenceError> {if $(valid(&value.$field))&&+ {Ok(Self(value))} else {Err(ReferenceError)}}
            pub fn reference(&self)->&weave_contract::$dto {&self.0}
            pub(crate) fn size(&self)->usize {256 $(+ self.0.$field.len()*6)+}
        }
        impl<'de> Deserialize<'de> for $name {
            fn deserialize<D:Deserializer<'de>>(d:D)->Result<Self,D::Error> {
                #[derive(Deserialize)]
                #[serde(deny_unknown_fields)]
                struct Wire { $($field: Identity),+ }
                let wire=Wire::deserialize(d)?;
                Ok(Self(weave_contract::$dto {$($field:wire.$field.0),+}))
            }
        }
    }
}
checked!(NodeReference, NodeRef, [graph_id, revision, node_id]);
checked!(EdgeReference, StructuralRef, [graph_id, revision, edge_id]);
checked!(
    AssertionReference,
    AssertionRef,
    [graph_id, revision, assertion_id]
);
checked!(SnapshotReference, GraphRef, [graph_id, revision]);
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(
    tag = "kind",
    content = "reference",
    rename_all = "snake_case",
    deny_unknown_fields
)]
pub enum ObjectReference {
    Node(NodeReference),
    Edge(EdgeReference),
    Assertion(AssertionReference),
    Snapshot(SnapshotReference),
}
impl ObjectReference {
    pub(crate) fn size(&self) -> usize {
        128 + match self {
            Self::Node(r) => r.size(),
            Self::Edge(r) => r.size(),
            Self::Assertion(r) => r.size(),
            Self::Snapshot(r) => r.size(),
        }
    }
}
