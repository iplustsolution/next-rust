//! Deserialize route [`Params`] into typed structs.
//!
//! Single values deserialize into strings, numbers and booleans (parsed from
//! text); catch-all values deserialize into sequences.

use next_rust_router::{ParamValue, Params};
use serde::de::{self, DeserializeOwned, IntoDeserializer, MapAccess, SeqAccess, Visitor};

#[derive(Debug)]
pub struct DeError(String);

impl std::fmt::Display for DeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

impl std::error::Error for DeError {}

impl de::Error for DeError {
    fn custom<T: std::fmt::Display>(msg: T) -> Self {
        DeError(msg.to_string())
    }
}

pub fn from_params<T: DeserializeOwned>(params: &Params) -> Result<T, DeError> {
    T::deserialize(ParamsDe { params })
}

struct ParamsDe<'a> {
    params: &'a Params,
}

impl<'de> de::Deserializer<'de> for ParamsDe<'_> {
    type Error = DeError;

    fn deserialize_any<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, DeError> {
        let entries: Vec<(String, ParamValue)> = self.params.iter().map(|(k, v)| (k.to_owned(), v.clone())).collect();
        visitor.visit_map(ParamsMap { iter: entries.into_iter(), value: None })
    }

    fn deserialize_newtype_struct<V: Visitor<'de>>(self, _name: &'static str, visitor: V) -> Result<V::Value, DeError> {
        visitor.visit_newtype_struct(self)
    }

    serde::forward_to_deserialize_any! {
        bool i8 i16 i32 i64 i128 u8 u16 u32 u64 u128 f32 f64 char str string
        bytes byte_buf option unit unit_struct seq tuple
        tuple_struct map struct enum identifier ignored_any
    }
}

struct ParamsMap {
    iter: std::vec::IntoIter<(String, ParamValue)>,
    value: Option<ParamValue>,
}

impl<'de> MapAccess<'de> for ParamsMap {
    type Error = DeError;

    fn next_key_seed<K: de::DeserializeSeed<'de>>(&mut self, seed: K) -> Result<Option<K::Value>, DeError> {
        match self.iter.next() {
            Some((k, v)) => {
                self.value = Some(v);
                seed.deserialize(k.into_deserializer()).map(Some)
            }
            None => Ok(None),
        }
    }

    fn next_value_seed<V: de::DeserializeSeed<'de>>(&mut self, seed: V) -> Result<V::Value, DeError> {
        match self.value.take() {
            Some(v) => seed.deserialize(ValueDe(v)),
            None => Err(DeError("value without key".into())),
        }
    }
}

struct ValueDe(ParamValue);

macro_rules! parse_num {
    ($($method:ident => $visit:ident : $t:ty),*) => {$(
        fn $method<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, DeError> {
            match self.0 {
                ParamValue::One(s) => visitor.$visit(s.parse::<$t>().map_err(|e| DeError(format!("`{s}`: {e}")))?),
                ParamValue::Many(_) => Err(DeError("expected a single value".into())),
            }
        }
    )*};
}

impl<'de> de::Deserializer<'de> for ValueDe {
    type Error = DeError;

    fn deserialize_any<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, DeError> {
        match self.0 {
            ParamValue::One(s) => visitor.visit_string(s),
            ParamValue::Many(v) => visitor.visit_seq(Seq(v.into_iter())),
        }
    }

    parse_num! {
        deserialize_bool => visit_bool: bool,
        deserialize_i8 => visit_i8: i8, deserialize_i16 => visit_i16: i16,
        deserialize_i32 => visit_i32: i32, deserialize_i64 => visit_i64: i64,
        deserialize_u8 => visit_u8: u8, deserialize_u16 => visit_u16: u16,
        deserialize_u32 => visit_u32: u32, deserialize_u64 => visit_u64: u64,
        deserialize_f32 => visit_f32: f32, deserialize_f64 => visit_f64: f64
    }

    fn deserialize_option<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, DeError> {
        visitor.visit_some(self)
    }

    fn deserialize_seq<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, DeError> {
        match self.0 {
            ParamValue::One(s) => visitor.visit_seq(Seq(vec![s].into_iter())),
            ParamValue::Many(v) => visitor.visit_seq(Seq(v.into_iter())),
        }
    }

    fn deserialize_newtype_struct<V: Visitor<'de>>(self, _: &'static str, visitor: V) -> Result<V::Value, DeError> {
        visitor.visit_newtype_struct(self)
    }

    fn deserialize_enum<V: Visitor<'de>>(
        self,
        _: &'static str,
        _: &'static [&'static str],
        visitor: V,
    ) -> Result<V::Value, DeError> {
        match self.0 {
            ParamValue::One(s) => visitor.visit_enum(s.into_deserializer()),
            ParamValue::Many(_) => Err(DeError("expected a single value".into())),
        }
    }

    serde::forward_to_deserialize_any! {
        i128 u128 char str string bytes byte_buf unit unit_struct tuple
        tuple_struct map struct identifier ignored_any
    }
}

struct Seq(std::vec::IntoIter<String>);

impl<'de> SeqAccess<'de> for Seq {
    type Error = DeError;

    fn next_element_seed<T: de::DeserializeSeed<'de>>(&mut self, seed: T) -> Result<Option<T::Value>, DeError> {
        match self.0.next() {
            Some(s) => seed.deserialize(ValueDe(ParamValue::One(s))).map(Some),
            None => Ok(None),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde::Deserialize;

    #[derive(Deserialize, Debug, PartialEq)]
    struct P {
        id: u64,
        slug: String,
        rest: Vec<String>,
        flag: Option<bool>,
    }

    #[test]
    fn typed_params() {
        let params = Params::new().with("id", "42").with("slug", "hello").with("rest", ["a", "b"]).with("flag", "true");
        let p: P = from_params(&params).unwrap();
        assert_eq!(p, P { id: 42, slug: "hello".into(), rest: vec!["a".into(), "b".into()], flag: Some(true) });
        let bad = Params::new().with("id", "abc").with("slug", "x").with("rest", ["a"]);
        assert!(from_params::<P>(&bad).is_err());
        let map: std::collections::BTreeMap<String, String> = from_params(&Params::new().with("a", "1")).unwrap();
        assert_eq!(map["a"], "1");
    }
}
