//! Host-side tests for the props wire format: the reader in `src/props.rs`
//! against the encoder in `src/encode.rs`, run on the native target with
//! `--features encode`. The whole file is gated on that feature so a plain
//! `cargo test -p next-rust-wasm` (no features) still compiles — it just
//! runs none of this.
#![cfg(feature = "encode")]

use next_rust_wasm::encode::{self, EncodeError, Value as EValue};
use next_rust_wasm::k;
use next_rust_wasm::props::{Props, Value, key};

#[test]
fn k_matches_key() {
    assert_eq!(k!("label"), key("label"));
    assert_eq!(k!(""), key(""));
    assert_eq!(
        k!("a rather longer field name, still just a name"),
        key("a rather longer field name, still just a name")
    );
}

#[test]
fn round_trip_every_tag() {
    let fields = [
        ("a_null", EValue::Null),
        ("b_false", EValue::Bool(false)),
        ("c_true", EValue::Bool(true)),
        ("d_int", EValue::Int(-42)),
        ("e_float", EValue::Float(3.5)),
        ("f_str", EValue::Str("hello")),
        ("g_bytes", EValue::Bytes(&[0, 1, 2, 255])),
        ("h_list", EValue::List(vec![EValue::Int(1), EValue::Str("two"), EValue::Bool(true)])),
        ("i_map", EValue::Map(vec![("x", EValue::Int(7)), ("y", EValue::Str("z"))])),
    ];
    let blob = encode::encode(&fields).expect("no collisions among these names");
    let props = Props::new(&blob);

    assert!(props.has(key("a_null")));
    assert_eq!(props.i64(key("a_null")), None, "NULL is not an INT");
    assert_eq!(props.bool(key("b_false")), Some(false));
    assert_eq!(props.bool(key("c_true")), Some(true));
    assert_eq!(props.i64(key("d_int")), Some(-42));
    assert_eq!(props.f64(key("d_int")), Some(-42.0), "f64() also accepts an INT-tagged value");
    assert_eq!(props.f64(key("e_float")), Some(3.5));
    assert_eq!(props.i64(key("e_float")), None, "i64() does not accept a FLOAT-tagged value");
    assert_eq!(props.str(key("f_str")), Some("hello"));
    assert_eq!(props.bytes(key("g_bytes")), Some(&[0u8, 1, 2, 255][..]));

    let list = props.list(key("h_list")).expect("h_list is a LIST");
    assert_eq!(list.len(), 3);
    assert_eq!(list.get(0).i64(), Some(1));
    assert_eq!(list.get(1).str(), Some("two"));
    assert_eq!(list.get(2).bool(), Some(true));
    assert!(matches!(list.get(3), Value::Null), "past the end reads as null");
    let collected: Vec<Value<'_>> = list.iter().collect();
    assert_eq!(collected.len(), 3);

    let map = props.map(key("i_map")).expect("i_map is a MAP");
    assert_eq!(map.i64(key("x")), Some(7));
    assert_eq!(map.str(key("y")), Some("z"));
    assert!(!map.has(key("not_there")));

    // A key absent from the blob entirely.
    assert!(!props.has(key("nope")));
    assert_eq!(props.str(key("nope")), None);
}

#[test]
fn nested_list_of_maps_and_map_of_lists() {
    let fields = [
        (
            "rows",
            EValue::List(vec![
                EValue::Map(vec![("id", EValue::Int(1)), ("label", EValue::Str("one"))]),
                EValue::Map(vec![("id", EValue::Int(2)), ("label", EValue::Str("two"))]),
            ]),
        ),
        ("groups", EValue::Map(vec![("nums", EValue::List(vec![EValue::Int(1), EValue::Int(2), EValue::Int(3)]))])),
    ];
    let blob = encode::encode(&fields).unwrap();
    let props = Props::new(&blob);

    let rows = props.list(key("rows")).unwrap();
    assert_eq!(rows.len(), 2);
    let row0 = rows.get(0).as_map().expect("row is a MAP");
    assert_eq!(row0.i64(key("id")), Some(1));
    assert_eq!(row0.str(key("label")), Some("one"));
    let row1 = rows.get(1).as_map().unwrap();
    assert_eq!(row1.i64(key("id")), Some(2));

    let groups = props.map(key("groups")).unwrap();
    let nums = groups.list(key("nums")).unwrap();
    assert_eq!(nums.len(), 3);
    assert_eq!(nums.get(2).i64(), Some(3));
}

#[test]
fn unicode_strings_round_trip() {
    let s = "héllo wörld \u{1F389}"; // multi-byte UTF-8 and a 4-byte emoji
    let fields = [("greeting", EValue::Str(s))];
    let blob = encode::encode(&fields).unwrap();
    let props = Props::new(&blob);
    assert_eq!(props.str(key("greeting")), Some(s));
}

#[test]
fn zigzag_and_varint_edges() {
    let cases: &[(&str, i64)] = &[("zero", 0), ("one", 1), ("minus_one", -1), ("min", i64::MIN), ("max", i64::MAX)];
    let fields: Vec<(&str, EValue<'_>)> = cases.iter().map(|(n, v)| (*n, EValue::Int(*v))).collect();
    let blob = encode::encode(&fields).unwrap();
    let props = Props::new(&blob);
    for (name, expected) in cases {
        assert_eq!(props.i64(key(name)), Some(*expected), "field {name}");
    }
}

#[test]
fn empty_blob_reads_as_no_props() {
    let props = Props::new(&[]);
    assert!(!props.has(key("anything")));
    assert_eq!(props.i64(key("anything")), None);

    // Well-formed but zero entries: version 1, count 0.
    let props = Props::new(&[1, 0]);
    assert!(!props.has(key("anything")));

    let empty = encode::encode(&[]).unwrap();
    let props = Props::new(&empty);
    assert!(!props.has(key("anything")));
}

/// Every truncation of a well-formed blob must decode as "field not found",
/// never panic and never read past what is actually there. This is the
/// hard requirement on the reader: it has to be total over a hostile input,
/// not just correct over a well-formed one.
#[test]
fn every_truncation_of_a_good_blob_is_safe() {
    let fields = [
        ("a", EValue::Int(-1234567890123)),
        ("b", EValue::Float(1.5e10)),
        ("c", EValue::Str("a longer string so its length varint takes more than one byte, maybe")),
        ("d", EValue::Bytes(&[1, 2, 3, 4, 5, 6, 7, 8, 9, 10])),
        ("e", EValue::List(vec![EValue::Int(1), EValue::List(vec![EValue::Str("nested"), EValue::Bool(true)])])),
        ("f", EValue::Map(vec![("inner", EValue::Map(vec![("deep", EValue::Int(99))]))])),
    ];
    let blob = encode::encode(&fields).unwrap();

    for len in 0..=blob.len() {
        let prefix = &blob[..len];
        let props = Props::new(prefix);
        for name in ["a", "b", "c", "d", "e", "f", "missing"] {
            let k = key(name);
            let _ = props.has(k);
            let _ = props.i64(k);
            let _ = props.f64(k);
            let _ = props.bool(k);
            let _ = props.str(k);
            let _ = props.bytes(k);
            if let Some(list) = props.list(k) {
                for i in 0..=list.len() {
                    let _ = list.get(i);
                }
                let _: Vec<Value<'_>> = list.iter().collect();
            }
            if let Some(map) = props.map(k) {
                let _ = map.has(key("deep"));
                let _ = map.i64(key("deep"));
            }
        }
    }
}

#[test]
fn key_collision_is_rejected_and_names_both_fields() {
    // "jZ3" and "Wraa" both hash to the same 32-bit FNV-1a key under this
    // crate's exact hash (basis 0x811c9dc5, prime 0x01000193) — found by
    // brute-force search, not hand-picked to look alike.
    assert_eq!(key("jZ3"), key("Wraa"), "fixture assumption: these must collide");

    let fields = [("jZ3", EValue::Int(1)), ("Wraa", EValue::Int(2))];
    let err = encode::encode(&fields).expect_err("colliding keys must be a build-time error");
    assert!(err.message().contains("jZ3"), "message names the first field: {}", err.message());
    assert!(err.message().contains("Wraa"), "message names the second field: {}", err.message());
}

#[test]
fn duplicate_field_name_is_also_rejected() {
    let fields = [("dup", EValue::Int(1)), ("dup", EValue::Int(2))];
    let err: EncodeError = encode::encode(&fields).expect_err("the same field twice must be an error");
    assert!(err.message().contains("dup"));
}

#[test]
fn nested_map_collisions_are_caught_too() {
    let fields = [("outer", EValue::Map(vec![("jZ3", EValue::Int(1)), ("Wraa", EValue::Int(2))]))];
    assert!(encode::encode(&fields).is_err(), "a collision inside a nested MAP must fail the whole encode");
}

next_rust_wasm::props! {
    pub struct Plan<'a> {
        count: i64 = 0,
        label: &'a str = "",
        open: bool = false,
        price: f64 = 0.0,
    }
}

#[test]
fn props_macro_reads_fields_with_defaults_for_missing_ones() {
    let fields = [("count", EValue::Int(3)), ("label", EValue::Str("Pro")), ("open", EValue::Bool(true))];
    let blob = encode::encode(&fields).unwrap();
    let props = Props::new(&blob);
    let plan = Plan::read(&props);
    assert_eq!(plan.count, 3);
    assert_eq!(plan.label, "Pro");
    assert!(plan.open);
    assert_eq!(plan.price, 0.0, "absent field falls back to its default");
}

/// Props are keyed by the server component's argument names, so a component
/// taking a single struct nests its fields one level down. That shape has
/// already cost a debugging session — a field plainly present in the HTML read
/// as `None` — so it gets a worked example here rather than only a doc line.
#[test]
fn a_single_argument_component_nests_its_fields_under_the_argument_name() {
    use next_rust_wasm::encode::{Value, encode};
    use next_rust_wasm::{Props, k};

    let blob = encode(&[(
        "props",
        Value::Map(vec![("seconds_remaining", Value::Int(300)), ("title", Value::Str("Paper 1"))]),
    )])
    .unwrap();
    let props = Props::new(&blob);

    // The trap: the field is there, but not where a reader expects it.
    assert_eq!(props.i64(k!("seconds_remaining")), None);

    // Both ways through.
    let inner = props.map(k!("props")).unwrap();
    assert_eq!(inner.i64(k!("seconds_remaining")), Some(300));
    assert_eq!(inner.str(k!("title")), Some("Paper 1"));
}
