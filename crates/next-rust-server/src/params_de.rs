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

    /// xorshift64*: seeded, so a failure is reproducible.
    struct Rng(u64);

    impl Rng {
        fn next(&mut self) -> u64 {
            self.0 ^= self.0 >> 12;
            self.0 ^= self.0 << 25;
            self.0 ^= self.0 >> 27;
            self.0.wrapping_mul(0x2545_F491_4F6C_DD1D)
        }

        fn below(&mut self, n: usize) -> usize {
            (self.next() % n as u64) as usize
        }

        fn pick<'a>(&mut self, from: &[&'a str]) -> &'a str {
            from[self.below(from.len())]
        }
    }

    const VALUES: &[&str] = &[
        "",
        "0",
        "-0",
        "+1",
        "1",
        "-1",
        "42",
        "99999999999999999999",
        "-99999999999999999999",
        "18446744073709551616",
        "2147483648",
        "-2147483649",
        "1e400",
        "-1e400",
        "NaN",
        "nan",
        "inf",
        "-inf",
        "infinity",
        "0x10",
        "1_000",
        " 1",
        "1 ",
        "1.5",
        ".5",
        "true",
        "TRUE",
        "false",
        "yes",
        "on",
        "One",
        "Two",
        "three",
        "null",
        "\u{0}",
        "\u{feff}",
        "héllo",
        "a/b",
        "..",
        "%2e",
        "<script>",
        "id",
        "slug",
    ];

    #[derive(Deserialize, Debug, PartialEq)]
    #[serde(rename_all = "snake_case")]
    enum Kind {
        One,
        Two,
    }

    #[derive(Deserialize, Debug)]
    #[allow(dead_code)]
    struct Wide {
        id: Option<i32>,
        n: Option<u8>,
        big: Option<i64>,
        f: Option<f64>,
        flag: Option<bool>,
        kind: Option<Kind>,
        rest: Option<Vec<String>>,
        nested: Option<Vec<Vec<String>>>,
        text: Option<String>,
    }

    /// Numbers that do not fit are an error, never a panic and never a
    /// wrapped value. `-0`, `+1` and `1e400` are the ones worth naming.
    #[test]
    fn numbers_out_of_range_are_refused_rather_than_wrapped() {
        let one = |name: &str, v: &str| Params::new().with(name, v);
        #[derive(Deserialize, Debug)]
        struct I(i32);
        #[derive(Deserialize, Debug)]
        struct F(f64);
        for bad in ["99999999999999999999", "2147483648", "-2147483649", "1e400", "NaN", " 1", "1_000", "0x10", ""] {
            let got: Result<std::collections::BTreeMap<String, I>, _> = from_params(&one("x", bad));
            assert!(got.is_err(), "i32 accepted {bad:?}: {got:?}");
        }
        let ok =
            |v: &str| from_params::<std::collections::BTreeMap<String, I>>(&one("x", v)).map(|m| m["x"].0).expect(v);
        assert_eq!(ok("-0"), 0);
        assert_eq!(ok("+1"), 1);
        assert_eq!(ok("2147483647"), i32::MAX);
        // `f64` follows Rust's parser: `1e400` is infinity and `NaN` is NaN,
        // both finite-looking to a page that does not check. Worth knowing.
        let f = |v: &str| from_params::<std::collections::BTreeMap<String, F>>(&one("x", v)).map(|m| m["x"].0);
        assert_eq!(f("1e400").unwrap(), f64::INFINITY);
        assert!(f("NaN").unwrap().is_nan());
        assert!(f("nan").unwrap().is_nan());
        assert!(f("1e999999999999999999").unwrap().is_infinite());
        assert!(f("0x10").is_err());
    }

    /// Generated parameters against every shape a page might ask for: an
    /// error or a value, never a panic, and always quickly.
    #[test]
    fn generated_params_never_panic_whatever_the_target_type() {
        let mut rng = Rng(0xDE5E_5EED_0001);
        let names = ["id", "n", "big", "f", "flag", "kind", "rest", "nested", "text", "", "0", "a b"];
        let start = std::time::Instant::now();
        let mut wide_ok = 0usize;
        for _ in 0..8_000 {
            let mut params = Params::new();
            for _ in 0..rng.below(6) {
                let name = names[rng.below(names.len())];
                if rng.next().is_multiple_of(3) {
                    let many: Vec<String> = (0..rng.below(4)).map(|_| rng.pick(VALUES).to_owned()).collect();
                    params.insert(name, many);
                } else {
                    params.insert(name, rng.pick(VALUES));
                }
            }
            wide_ok += usize::from(from_params::<Wide>(&params).is_ok());
            let _ = from_params::<std::collections::BTreeMap<String, String>>(&params);
            let _ = from_params::<std::collections::BTreeMap<String, Vec<String>>>(&params);
            let _ = from_params::<std::collections::BTreeMap<String, serde_json::Value>>(&params);
            let _ = from_params::<Vec<(String, String)>>(&params);
            let _ = from_params::<()>(&params);
        }
        assert!(wide_ok > 100, "the generator produces parameters a page can use ({wide_ok})");
        assert!(start.elapsed() < std::time::Duration::from_secs(20), "took {:?}", start.elapsed());
    }

    /// Shapes that would be a problem if they were not bounded: a very long
    /// value, very many parameters, and a name that looks like Rust.
    #[test]
    fn size_is_bounded_by_the_input() {
        let big = "x".repeat(400_000);
        let params = Params::new().with("text", big.as_str()).with("rest", vec![big.as_str(), big.as_str()]);
        let start = std::time::Instant::now();
        let got: Wide = from_params(&params).expect("deserialises");
        assert_eq!(got.text.as_deref().map(str::len), Some(big.len()));
        let mut many = Params::new();
        for i in 0..5_000 {
            many.insert(format!("p{i}"), "1");
        }
        let map: std::collections::BTreeMap<String, u8> = from_params(&many).expect("deserialises");
        assert_eq!(map.len(), 5_000);
        assert!(start.elapsed() < std::time::Duration::from_secs(20), "took {:?}", start.elapsed());
        // A name that collides with Rust syntax is data, not code.
        for name in ["self", "Self", "crate", "super", "fn", "impl", "unsafe", "r#type", "std::process::exit"] {
            let got: std::collections::BTreeMap<String, String> = from_params(&Params::new().with(name, "v")).unwrap();
            assert_eq!(got[name], "v");
        }
    }

    /// A catch-all is a sequence, a single value is not — and asking for the
    /// wrong one is an error rather than a silent join or split.
    #[test]
    fn single_values_and_catch_alls_do_not_swap_places() {
        #[derive(Deserialize, Debug, PartialEq)]
        struct S {
            v: Vec<String>,
        }
        #[derive(Deserialize, Debug, PartialEq)]
        struct O {
            v: u32,
        }
        #[derive(Deserialize, Debug, PartialEq)]
        struct E {
            v: Kind,
        }
        // A single value read as a sequence is a one element sequence.
        assert_eq!(from_params::<S>(&Params::new().with("v", "a/b")).unwrap(), S { v: vec!["a/b".into()] });
        // A sequence read as a number or an enum is an error, not a join.
        assert!(from_params::<O>(&Params::new().with("v", vec!["1", "2"])).is_err());
        assert!(from_params::<E>(&Params::new().with("v", vec!["one"])).is_err());
        assert_eq!(from_params::<E>(&Params::new().with("v", "one")).unwrap(), E { v: Kind::One });
        assert!(from_params::<E>(&Params::new().with("v", "One")).is_err(), "variants are not renamed twice");
        assert!(from_params::<E>(&Params::new().with("v", "three")).is_err());
        // An empty catch-all is an empty sequence, not a missing field.
        let empty: Vec<String> = Vec::new();
        assert_eq!(from_params::<S>(&Params::new().with("v", empty)).unwrap(), S { v: Vec::new() });
    }
}
