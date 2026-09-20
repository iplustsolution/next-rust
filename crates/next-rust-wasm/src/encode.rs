//! The host-side counterpart to `crate::props`: turns a tree of values into
//! the same blob the reader decodes, and base64-encodes it for a
//! `data-p="…"` attribute.
//!
//! This module is `std` and only ever runs on the server that renders pages
//! — never in the browser — which is why it lives behind the `encode`
//! feature instead of always being part of the `no_std` build every island
//! pays for. It duplicates the tiny varint/zigzag arithmetic from
//! `crate::props` rather than sharing it, on purpose: the reader is not
//! allowed to grow a code path that only the write side needs.

#![cfg(feature = "encode")]

extern crate std;

use std::fmt;
use std::string::String;
use std::vec::Vec;

use crate::props::key;

/// A value to encode, mirroring the tag table in `crate::props`. Borrowed
/// strings and bytes, since a tree built for one `encode` call only needs to
/// live for that call.
#[derive(Debug, Clone, PartialEq)]
pub enum Value<'a> {
    Null,
    Bool(bool),
    Int(i64),
    Float(f64),
    Str(&'a str),
    Bytes(&'a [u8]),
    List(Vec<Value<'a>>),
    Map(Vec<(&'a str, Value<'a>)>),
}

/// A build-time failure: two fields hashed to the same key, or the same
/// field name was given twice. Both are bugs in the caller, never something
/// to encode and ship — there is no lossy fallback here.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EncodeError {
    message: String,
}

impl EncodeError {
    fn new(message: String) -> Self {
        Self { message }
    }

    pub fn message(&self) -> &str {
        &self.message
    }
}

impl fmt::Display for EncodeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.message)
    }
}

impl std::error::Error for EncodeError {}

/// Encode `fields` as a top-level blob: version byte, then the same
/// count-and-entries shape a nested MAP uses.
pub fn encode(fields: &[(&str, Value<'_>)]) -> Result<Vec<u8>, EncodeError> {
    let mut out = std::vec![1u8];
    write_entries(&mut out, fields)?;
    Ok(out)
}

/// [`encode`], then standard (padded) base64 — what goes directly into
/// `data-p="…"`.
pub fn encode_base64(fields: &[(&str, Value<'_>)]) -> Result<String, EncodeError> {
    Ok(base64_encode(&encode(fields)?))
}

/// Sort by key, and refuse to produce a blob where that sort does not give a
/// strict ordering — a collision or a repeated name would otherwise
/// silently shadow one field with another the moment `Props::find`'s
/// early-out (`k > target` stops the scan) is reached.
fn sorted_entries<'a, 'f>(
    fields: &'f [(&'a str, Value<'a>)],
) -> Result<Vec<(u32, &'a str, &'f Value<'a>)>, EncodeError> {
    let mut entries: Vec<(u32, &str, &Value<'_>)> = fields.iter().map(|(n, v)| (key(n), *n, v)).collect();
    entries.sort_by_key(|(k, _, _)| *k);
    for w in entries.windows(2) {
        if w[0].0 == w[1].0 {
            let message = if w[0].1 == w[1].1 {
                std::format!("duplicate props field \"{}\"", w[0].1)
            } else {
                std::format!("props key collision: \"{}\" and \"{}\" both hash to 0x{:08x}", w[0].1, w[1].1, w[0].0)
            };
            return Err(EncodeError::new(message));
        }
    }
    Ok(entries)
}

fn write_entries(out: &mut Vec<u8>, fields: &[(&str, Value<'_>)]) -> Result<(), EncodeError> {
    let entries = sorted_entries(fields)?;
    write_uvarint(out, entries.len() as u64);
    for (k, _, v) in entries {
        out.extend_from_slice(&k.to_le_bytes());
        write_value(out, v)?;
    }
    Ok(())
}

fn write_value(out: &mut Vec<u8>, v: &Value<'_>) -> Result<(), EncodeError> {
    match v {
        Value::Null => out.push(0),
        Value::Bool(false) => out.push(1),
        Value::Bool(true) => out.push(2),
        Value::Int(n) => {
            out.push(3);
            write_uvarint(out, zigzag_encode(*n));
        }
        Value::Float(f) => {
            out.push(4);
            out.extend_from_slice(&f.to_le_bytes());
        }
        Value::Str(s) => {
            out.push(5);
            write_uvarint(out, s.len() as u64);
            out.extend_from_slice(s.as_bytes());
        }
        Value::Bytes(b) => {
            out.push(6);
            write_uvarint(out, b.len() as u64);
            out.extend_from_slice(b);
        }
        Value::List(items) => {
            out.push(7);
            write_uvarint(out, items.len() as u64);
            for item in items {
                write_value(out, item)?;
            }
        }
        Value::Map(fields) => {
            out.push(8);
            write_entries(out, fields)?;
        }
    }
    Ok(())
}

fn write_uvarint(out: &mut Vec<u8>, mut v: u64) {
    loop {
        let mut byte = (v & 0x7f) as u8;
        v >>= 7;
        if v != 0 {
            byte |= 0x80;
        }
        out.push(byte);
        if v == 0 {
            break;
        }
    }
}

/// The same mapping `Reader::zigzag_i64` in `crate::props` inverts:
/// non-negative `n` doubles, negative `n` doubles and subtracts one. Correct
/// at the `i64::MIN`/`i64::MAX` edges because it is done with the wrapping
/// shift-and-xor form, not a `checked`/`saturating` one that would clamp
/// them.
fn zigzag_encode(n: i64) -> u64 {
    ((n << 1) ^ (n >> 63)) as u64
}

const B64: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";

/// Standard, padded base64 — no external crate for something this small on
/// the write side of a crate that otherwise has zero dependencies.
fn base64_encode(data: &[u8]) -> String {
    let mut out = String::with_capacity(data.len().div_ceil(3) * 4);
    for chunk in data.chunks(3) {
        let b0 = chunk[0] as u32;
        let b1 = *chunk.get(1).unwrap_or(&0) as u32;
        let b2 = *chunk.get(2).unwrap_or(&0) as u32;
        let n = (b0 << 16) | (b1 << 8) | b2;
        out.push(B64[(n >> 18 & 0x3f) as usize] as char);
        out.push(B64[(n >> 12 & 0x3f) as usize] as char);
        out.push(if chunk.len() > 1 { B64[(n >> 6 & 0x3f) as usize] as char } else { '=' });
        out.push(if chunk.len() > 2 { B64[(n & 0x3f) as usize] as char } else { '=' });
    }
    out
}

/// The shim's fallback path (§6: "the shim encodes the parsed `data-props`
/// JSON object into the same format"), and the implementation behind the
/// `JSON_DECODE` opcode: a parsed JSON value in, the identical blob
/// `encode` would have produced out, as a `Uint8Array`.
///
/// Pasted verbatim into the shim, so it is written to survive minification
/// (no `Function.name`, no class, nothing but closures over plain data) and
/// to need nothing beyond `TextEncoder` and `DataView`, both already
/// available wherever the shim runs.
///
/// One JSON-shaped gap is inherent, not a bug: JSON has one numeric type, so
/// this treats `Number.isInteger(v)` as tag 3 (INT) and anything else as
/// tag 4 (FLOAT) — the same rule the Rust encoder's caller has to apply by
/// hand when building a `Value` tree from parsed JSON. BYTES (tag 6) is
/// unreachable from JSON and is not attempted.
pub const JSON_TO_BLOB_JS: &str = r#"function nrEncodeProps(obj) {
  var out = [];
  function fnv1a(name) {
    var h = 0x811c9dc5, b = new TextEncoder().encode(name);
    for (var i = 0; i < b.length; i++) { h ^= b[i]; h = Math.imul(h, 0x01000193); }
    return h >>> 0;
  }
  function varint(n) {
    for (;;) { var b = n & 0x7f; n = Math.floor(n / 128); if (n) b |= 0x80; out.push(b); if (!n) return; }
  }
  function bytesOf(s) { var b = new TextEncoder().encode(s); varint(b.length); for (var i = 0; i < b.length; i++) out.push(b[i]); }
  function value(v) {
    if (v === null || v === undefined) { out.push(0); return; }
    if (typeof v === "boolean") { out.push(v ? 2 : 1); return; }
    if (typeof v === "number") {
      if (Number.isInteger(v)) { out.push(3); varint(v >= 0 ? v * 2 : -v * 2 - 1); return; }
      out.push(4);
      var buf = new ArrayBuffer(8);
      new DataView(buf).setFloat64(0, v, true);
      var u8 = new Uint8Array(buf);
      for (var i = 0; i < 8; i++) out.push(u8[i]);
      return;
    }
    if (typeof v === "string") { out.push(5); bytesOf(v); return; }
    if (Array.isArray(v)) { out.push(7); varint(v.length); for (var i = 0; i < v.length; i++) value(v[i]); return; }
    entries(v, false);
  }
  function entries(o, top) {
    var keys = Object.keys(o).map(function (n) { return { k: fnv1a(n), n: n, v: o[n] }; });
    keys.sort(function (a, b) { return a.k - b.k; });
    if (!top) out.push(8);
    varint(keys.length);
    for (var i = 0; i < keys.length; i++) {
      var k = keys[i].k;
      out.push(k & 0xff, (k >>> 8) & 0xff, (k >>> 16) & 0xff, (k >>> 24) & 0xff);
      value(keys[i].v);
    }
  }
  out.push(1);
  entries(obj, true);
  return new Uint8Array(out);
}
"#;
