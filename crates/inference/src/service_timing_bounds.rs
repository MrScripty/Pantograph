//! Bounded borrowed preflight before optional identity serialization. No cloning
//! of caller values, unbounded string scans, or recursion beyond the depth cap.
use serde::ser::{self, Serialize};

const MAX_BYTES: usize = 64 * 1024;
const MAX_NODES: usize = 2048;
const MAX_DEPTH: usize = 32;

pub(super) fn digest(value: &impl Serialize) -> Option<String> {
    value
        .serialize(&mut Budget {
            bytes: 0,
            nodes: 0,
            depth: 0,
        })
        .ok()?;
    // Preflight bounds raw bytes, nodes and depth. Escape expansion is at most
    // six bytes per raw byte; structural output is bounded by the node budget.
    let bytes = serde_json::to_vec(value).ok()?;
    Some(blake3::hash(&bytes).to_hex().to_string())
}

#[derive(Debug)]
struct Limit;
impl std::fmt::Display for Limit {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("timing identity budget exceeded")
    }
}
impl std::error::Error for Limit {}
impl ser::Error for Limit {
    fn custom<T: std::fmt::Display>(_: T) -> Self {
        Self
    }
}
struct Budget {
    bytes: usize,
    nodes: usize,
    depth: usize,
}
impl Budget {
    fn charge(&mut self, bytes: usize) -> Result<(), Limit> {
        self.bytes = self.bytes.checked_add(bytes).ok_or(Limit)?;
        self.nodes += 1;
        if self.bytes > MAX_BYTES || self.nodes > MAX_NODES {
            return Err(Limit);
        }
        Ok(())
    }
    fn compound(&mut self, length: Option<usize>) -> Result<Compound<'_>, Limit> {
        self.charge(0)?;
        if length.is_some_and(|length| length > MAX_NODES - self.nodes) || self.depth == MAX_DEPTH {
            return Err(Limit);
        }
        self.depth += 1;
        Ok(Compound(self))
    }
}
struct Compound<'a>(&'a mut Budget);
impl Drop for Compound<'_> {
    fn drop(&mut self) {
        self.0.depth -= 1;
    }
}

macro_rules! scalar {
    ($($method:ident($ty:ty)),*) => { $(
        fn $method(self, _: $ty) -> Result<(), Limit> { self.charge(0) }
    )* };
}
impl<'a> ser::Serializer for &'a mut Budget {
    type Ok = ();
    type Error = Limit;
    type SerializeSeq = Compound<'a>;
    type SerializeTuple = Compound<'a>;
    type SerializeTupleStruct = Compound<'a>;
    type SerializeTupleVariant = Compound<'a>;
    type SerializeMap = Compound<'a>;
    type SerializeStruct = Compound<'a>;
    type SerializeStructVariant = Compound<'a>;
    scalar!(
        serialize_bool(bool),
        serialize_i8(i8),
        serialize_i16(i16),
        serialize_i32(i32),
        serialize_i64(i64),
        serialize_u8(u8),
        serialize_u16(u16),
        serialize_u32(u32),
        serialize_u64(u64),
        serialize_f32(f32),
        serialize_f64(f64),
        serialize_char(char)
    );
    fn serialize_str(self, value: &str) -> Result<(), Limit> {
        self.charge(value.len())
    }
    fn serialize_bytes(self, value: &[u8]) -> Result<(), Limit> {
        self.charge(value.len())
    }
    fn serialize_none(self) -> Result<(), Limit> {
        self.charge(0)
    }
    fn serialize_some<T: ?Sized + Serialize>(self, value: &T) -> Result<(), Limit> {
        self.charge(0)?;
        value.serialize(self)
    }
    fn serialize_unit(self) -> Result<(), Limit> {
        self.charge(0)
    }
    fn serialize_unit_struct(self, name: &'static str) -> Result<(), Limit> {
        self.charge(name.len())
    }
    fn serialize_unit_variant(
        self,
        name: &'static str,
        _: u32,
        variant: &'static str,
    ) -> Result<(), Limit> {
        self.charge(name.len() + variant.len())
    }
    fn serialize_newtype_struct<T: ?Sized + Serialize>(
        self,
        name: &'static str,
        value: &T,
    ) -> Result<(), Limit> {
        self.charge(name.len())?;
        value.serialize(self)
    }
    fn serialize_newtype_variant<T: ?Sized + Serialize>(
        self,
        name: &'static str,
        _: u32,
        variant: &'static str,
        value: &T,
    ) -> Result<(), Limit> {
        self.charge(name.len() + variant.len())?;
        value.serialize(self)
    }
    fn serialize_seq(self, len: Option<usize>) -> Result<Compound<'a>, Limit> {
        self.compound(len)
    }
    fn serialize_tuple(self, len: usize) -> Result<Compound<'a>, Limit> {
        self.compound(Some(len))
    }
    fn serialize_tuple_struct(self, _: &'static str, len: usize) -> Result<Compound<'a>, Limit> {
        self.compound(Some(len))
    }
    fn serialize_tuple_variant(
        self,
        _: &'static str,
        _: u32,
        _: &'static str,
        len: usize,
    ) -> Result<Compound<'a>, Limit> {
        self.compound(Some(len))
    }
    fn serialize_map(self, len: Option<usize>) -> Result<Compound<'a>, Limit> {
        self.compound(len)
    }
    fn serialize_struct(self, _: &'static str, len: usize) -> Result<Compound<'a>, Limit> {
        self.compound(Some(len))
    }
    fn serialize_struct_variant(
        self,
        _: &'static str,
        _: u32,
        _: &'static str,
        len: usize,
    ) -> Result<Compound<'a>, Limit> {
        self.compound(Some(len))
    }
    // Refuse a potentially allocating default Display conversion.
    fn collect_str<T: ?Sized + std::fmt::Display>(self, _: &T) -> Result<(), Limit> {
        Err(Limit)
    }
}
macro_rules! sequence {
    ($trait:ident, $method:ident) => {
        impl ser::$trait for Compound<'_> {
            type Ok = ();
            type Error = Limit;
            fn $method<T: ?Sized + Serialize>(&mut self, value: &T) -> Result<(), Limit> {
                value.serialize(&mut *self.0)
            }
            fn end(self) -> Result<(), Limit> {
                Ok(())
            }
        }
    };
}
sequence!(SerializeSeq, serialize_element);
sequence!(SerializeTuple, serialize_element);
sequence!(SerializeTupleStruct, serialize_field);
sequence!(SerializeTupleVariant, serialize_field);
impl ser::SerializeMap for Compound<'_> {
    type Ok = ();
    type Error = Limit;
    fn serialize_key<T: ?Sized + Serialize>(&mut self, key: &T) -> Result<(), Limit> {
        key.serialize(&mut *self.0)
    }
    fn serialize_value<T: ?Sized + Serialize>(&mut self, value: &T) -> Result<(), Limit> {
        value.serialize(&mut *self.0)
    }
    fn end(self) -> Result<(), Limit> {
        Ok(())
    }
}
macro_rules! fields {
    ($trait:ident) => {
        impl ser::$trait for Compound<'_> {
            type Ok = ();
            type Error = Limit;
            fn serialize_field<T: ?Sized + Serialize>(
                &mut self,
                key: &'static str,
                value: &T,
            ) -> Result<(), Limit> {
                self.0.charge(key.len())?;
                value.serialize(&mut *self.0)
            }
            fn end(self) -> Result<(), Limit> {
                Ok(())
            }
        }
    };
}
fields!(SerializeStruct);
fields!(SerializeStructVariant);

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn refuses_bytes_nodes_and_depth_without_truncating_identity() {
        assert!(digest(&"x".repeat(MAX_BYTES + 1)).is_none());
        assert!(digest(&vec![0; MAX_NODES + 1]).is_none());
        let mut deep = serde_json::Value::Null;
        for _ in 0..=MAX_DEPTH {
            deep = serde_json::json!([deep]);
        }
        assert!(digest(&deep).is_none());
        let value = serde_json::json!({"a": [0, " exact \n"]});
        assert_eq!(
            digest(&value),
            Some(
                blake3::hash(&serde_json::to_vec(&value).unwrap())
                    .to_hex()
                    .to_string()
            )
        );
    }
}
