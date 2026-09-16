use serde::{Deserialize, Serialize};

/// Value of a route parameter.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(untagged)]
pub enum ParamValue {
    /// `[id]`
    One(String),
    /// `[...slug]` and `[[...slug]]`
    Many(Vec<String>),
}

/// Route parameters captured while matching, in pattern order.
#[derive(Debug, Clone, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(
    from = "std::collections::BTreeMap<String, ParamValue>",
    into = "std::collections::BTreeMap<String, ParamValue>"
)]
pub struct Params(Vec<(String, ParamValue)>);

impl Params {
    pub fn new() -> Self {
        Self(Vec::new())
    }

    /// Builder helper: `Params::new().with("slug", "hello")`.
    pub fn with(mut self, name: impl Into<String>, value: impl Into<ParamValue>) -> Self {
        self.insert(name, value);
        self
    }

    pub fn insert(&mut self, name: impl Into<String>, value: impl Into<ParamValue>) {
        let name = name.into();
        let value = value.into();
        if let Some(slot) = self.0.iter_mut().find(|(n, _)| *n == name) {
            slot.1 = value;
        } else {
            self.0.push((name, value));
        }
    }

    pub(crate) fn pop(&mut self) {
        self.0.pop();
    }

    /// Single-segment value. For catch-all params this returns the segments
    /// joined by `/`… use [`Params::get_all`] to obtain them individually.
    pub fn get(&self, name: &str) -> Option<&str> {
        self.0.iter().find(|(n, _)| n == name).and_then(|(_, v)| match v {
            ParamValue::One(s) => Some(s.as_str()),
            ParamValue::Many(_) => None,
        })
    }

    /// Catch-all segments (`[...slug]`). A single `[id]` value is returned as
    /// a one element slice.
    pub fn get_all(&self, name: &str) -> Option<&[String]> {
        self.0.iter().find(|(n, _)| n == name).map(|(_, v)| match v {
            ParamValue::One(s) => std::slice::from_ref(s),
            ParamValue::Many(v) => v.as_slice(),
        })
    }

    pub fn value(&self, name: &str) -> Option<&ParamValue> {
        self.0.iter().find(|(n, _)| n == name).map(|(_, v)| v)
    }

    pub fn iter(&self) -> impl Iterator<Item = (&str, &ParamValue)> {
        self.0.iter().map(|(n, v)| (n.as_str(), v))
    }

    pub fn len(&self) -> usize {
        self.0.len()
    }

    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }
}

impl From<&str> for ParamValue {
    fn from(s: &str) -> Self {
        ParamValue::One(s.to_owned())
    }
}

impl From<String> for ParamValue {
    fn from(s: String) -> Self {
        ParamValue::One(s)
    }
}

impl From<Vec<String>> for ParamValue {
    fn from(v: Vec<String>) -> Self {
        ParamValue::Many(v)
    }
}

impl From<Vec<&str>> for ParamValue {
    fn from(v: Vec<&str>) -> Self {
        ParamValue::Many(v.into_iter().map(str::to_owned).collect())
    }
}

impl<const N: usize> From<[&str; N]> for ParamValue {
    fn from(v: [&str; N]) -> Self {
        ParamValue::Many(v.iter().map(|s| (*s).to_owned()).collect())
    }
}

impl From<std::collections::BTreeMap<String, ParamValue>> for Params {
    fn from(map: std::collections::BTreeMap<String, ParamValue>) -> Self {
        Params(map.into_iter().collect())
    }
}

impl From<Params> for std::collections::BTreeMap<String, ParamValue> {
    fn from(p: Params) -> Self {
        p.0.into_iter().collect()
    }
}

impl<K: Into<String>, V: Into<ParamValue>> FromIterator<(K, V)> for Params {
    fn from_iter<I: IntoIterator<Item = (K, V)>>(iter: I) -> Self {
        let mut p = Params::new();
        for (k, v) in iter {
            p.insert(k, v);
        }
        p
    }
}
