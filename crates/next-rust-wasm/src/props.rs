//! The compact binary encoding a `<nr-island data-p="…">` blob is read out of.
//!
//! There is no JSON parser anywhere in this module — that is the entire point
//! of the format. A field name never appears as bytes in the module either:
//! it is folded to a 32-bit key by an FNV-1a hash evaluated at compile time
//! (see [`key`] and [`k!`](crate::k)), so the only things the reader ever looks at are
//! `u32` keys and the value bytes that follow them.
//!
//! The wire shape (this is the frozen contract, not this crate's choice):
//!
//! ```text
//! blob   := u8 version(=1), uvarint count, entry*        (entries sorted by key)
//! entry  := u32le key, value
//! value  := u8 tag, payload
//! tag 0  NULL      (no payload)
//! tag 1  FALSE     (no payload)
//! tag 2  TRUE      (no payload)
//! tag 3  INT       zigzag uvarint (i64)
//! tag 4  FLOAT     8 bytes f64 little-endian
//! tag 5  STR       uvarint len, UTF-8 bytes
//! tag 6  BYTES     uvarint len, bytes
//! tag 7  LIST      uvarint n, value*
//! tag 8  MAP       uvarint n, (u32le key, value)*
//! ```
//!
//! Everything in this file is a *reader*: no allocation, no panics, and no
//! out-of-bounds access, no matter what bytes it is pointed at. A blob that
//! is truncated, corrupt, or actively hostile just makes lookups answer
//! `None` (or, for [`List::get`], `Value::Null`) — the same answer a caller
//! gets for a field that was never there. The host-side encoder that
//! produces well-formed blobs lives in [`crate::encode`], behind the
//! `encode` feature; it never runs in the browser.

/// A field name folded to a 32-bit key. A plain `u32` alias rather than a
/// newtype: every use of it is a comparison or a memory load, so a wrapper
/// type would only add a struct literal for `key()` and a field access at
/// every call site for no benefit once inlined — the alias gets there
/// without asking the optimizer to remove anything.
pub type Key = u32;

/// FNV-1a 32-bit over the name's UTF-8 bytes, offset basis `0x811c_9dc5`,
/// prime `0x0100_0193`. `const fn` so [`k!`](crate::k) can force it to run at compile
/// time: called from a `const` binding, the string literal is consumed by
/// the compiler's constant evaluator and only the resulting `u32` need
/// survive into the compiled module — no field name has to sit in the data
/// section for the browser to download.
pub const fn key(name: &str) -> Key {
    let bytes = name.as_bytes();
    let mut hash: u32 = 0x811c_9dc5;
    let mut i = 0;
    while i < bytes.len() {
        hash ^= bytes[i] as u32;
        hash = hash.wrapping_mul(0x0100_0193);
        i += 1;
    }
    hash
}

/// Hash a field name at compile time. `k!("label")` is the only way callers
/// are expected to produce a [`Key`] — binding it to a `const` inside the
/// macro is what forces the hash into the constant evaluator instead of
/// leaving it as a runtime call over a `&'static str` the compiler might
/// decide to keep around.
#[macro_export]
macro_rules! k {
    ($name:literal) => {{
        const NR_PROPS_KEY: $crate::props::Key = $crate::props::key($name);
        NR_PROPS_KEY
    }};
}

/// One decoded entry. Borrows from the blob it was read out of, so it never
/// outlives the [`Props`] (or [`List`]) it came from.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Value<'a> {
    Null,
    Bool(bool),
    Int(i64),
    Float(f64),
    Str(&'a str),
    Bytes(&'a [u8]),
    List(List<'a>),
    Map(Props<'a>),
}

impl<'a> Value<'a> {
    pub fn is_null(&self) -> bool {
        matches!(self, Value::Null)
    }

    pub fn bool(&self) -> Option<bool> {
        match self {
            Value::Bool(v) => Some(*v),
            _ => None,
        }
    }

    pub fn i64(&self) -> Option<i64> {
        match self {
            Value::Int(v) => Some(*v),
            _ => None,
        }
    }

    /// Also accepts an INT, converted losslessly (short of `i64` magnitudes
    /// past `f64`'s 53-bit mantissa). This is not generosity for its own
    /// sake: JSON has one numeric type, so the `data-props` fallback in
    /// [`crate::encode::JSON_TO_BLOB_JS`] must decide INT vs FLOAT with
    /// `Number.isInteger`, and a `f64` field that happens to hold a whole
    /// number — `0.0`, `-0.0`, `5.0` — comes across the fallback path
    /// tagged INT. Without this, `.f64()` on such a field would silently
    /// see nothing every time that path was taken.
    pub fn f64(&self) -> Option<f64> {
        match self {
            Value::Float(v) => Some(*v),
            Value::Int(v) => Some(*v as f64),
            _ => None,
        }
    }

    pub fn str(&self) -> Option<&'a str> {
        match self {
            Value::Str(v) => Some(*v),
            _ => None,
        }
    }

    pub fn bytes(&self) -> Option<&'a [u8]> {
        match self {
            Value::Bytes(v) => Some(*v),
            _ => None,
        }
    }

    pub fn as_list(&self) -> Option<List<'a>> {
        match self {
            Value::List(v) => Some(*v),
            _ => None,
        }
    }

    pub fn as_map(&self) -> Option<Props<'a>> {
        match self {
            Value::Map(v) => Some(*v),
            _ => None,
        }
    }
}

/// A LIST value: `uvarint n` followed by `n` values, none of which are
/// necessarily the same width. That rules out true random access — [`get`]
/// walks from the front and is `O(i)` — but each element still only needs
/// decoding once it is actually asked for.
///
/// [`get`]: List::get
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct List<'a> {
    data: &'a [u8],
    count: u32,
}

impl<'a> List<'a> {
    pub fn len(&self) -> u32 {
        self.count
    }

    pub fn is_empty(&self) -> bool {
        self.count == 0
    }

    /// The `i`th element, or `Value::Null` for an index at or past `len()`
    /// and for a blob so truncated the element can't be decoded — the same
    /// "absent reads as null" rule the rest of this module follows.
    pub fn get(&self, i: u32) -> Value<'a> {
        if i >= self.count {
            return Value::Null;
        }
        let mut r = Reader::new(self.data);
        for _ in 0..i {
            if skip_value(&mut r, 0).is_none() {
                return Value::Null;
            }
        }
        decode_value(&mut r, 0).unwrap_or(Value::Null)
    }

    pub fn iter(&self) -> ListIter<'a> {
        ListIter { r: Reader::new(self.data), remaining: self.count }
    }
}

impl<'a> IntoIterator for List<'a> {
    type Item = Value<'a>;
    type IntoIter = ListIter<'a>;

    fn into_iter(self) -> ListIter<'a> {
        self.iter()
    }
}

/// Forward iterator over a [`List`]. Stops (rather than panics) the moment
/// the blob runs out from under it.
pub struct ListIter<'a> {
    r: Reader<'a>,
    remaining: u32,
}

impl<'a> Iterator for ListIter<'a> {
    type Item = Value<'a>;

    fn next(&mut self) -> Option<Value<'a>> {
        if self.remaining == 0 {
            return None;
        }
        self.remaining -= 1;
        decode_value(&mut self.r, 0)
    }
}

/// A read-only view over a props blob, or over one MAP value nested inside
/// one. Two entries with the same key never both exist in a blob the encoder
/// produced (see `encode::EncodeError` in `crate::encode`), so lookup can
/// stop at the first key at or past the target.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Props<'a> {
    /// Bytes starting at the first entry's key, running to the end of this
    /// map's entries (the whole blob, for the top level; the sub-slice a
    /// nested MAP's `n` covers, for one read out of a list or another map).
    entries: &'a [u8],
    count: u32,
}

impl<'a> Props<'a> {
    /// An empty view: `has` is always false, every typed getter is `None`.
    pub const EMPTY: Self = Self { entries: &[], count: 0 };

    /// Parse the header (`version`, `count`) and hand back a view over the
    /// entries that follow. A blob with the wrong version, a header that
    /// does not fit, or a `count` too large to be a `u32` is treated as
    /// [`Props::EMPTY`] rather than rejected some other way — there is no
    /// `Result` in this API for a caller to have to handle.
    pub fn new(data: &'a [u8]) -> Self {
        let mut r = Reader::new(data);
        if r.u8() != Some(1) {
            return Self::EMPTY;
        }
        let count = match r.uvarint() {
            Some(c) if c <= u32::MAX as u64 => c as u32,
            _ => return Self::EMPTY,
        };
        Self { entries: data.get(r.pos..).unwrap_or(&[]), count }
    }

    /// Build a view from the `(ptr, len)` pair the shim hands the module in
    /// `hydrate_<Name>` (§4 of the ABI). `len == 0` and a null `ptr` both
    /// mean "no props" and produce [`Props::EMPTY`] without dereferencing
    /// anything.
    ///
    /// # Safety
    /// `ptr` must be valid for reads of `len` bytes for the lifetime `'a`
    /// the caller is claiming, and that memory must not be mutated while
    /// this view (or anything borrowed from it) is alive. This is the one
    /// place in the module that has to trust the host; every method on the
    /// resulting `Props` is safe and panic-free from here on.
    pub unsafe fn from_raw(ptr: *const u8, len: usize) -> Self {
        if ptr.is_null() || len == 0 {
            return Self::EMPTY;
        }
        // SAFETY: forwarded to the caller's own safety obligation above.
        let data = unsafe { core::slice::from_raw_parts(ptr, len) };
        Self::new(data)
    }

    /// Linear scan rather than binary search, deliberately: entries are
    /// variable-width (a STR or LIST payload has no fixed size), so there is
    /// no way to jump to the middle entry without an offset index built by
    /// an earlier full pass — which would cost an allocation this reader
    /// does not make. A single forward scan gets the same "stop once a
    /// key at or past the target is seen" early-out with none of that
    /// bookkeeping, so it is both the cheaper and the smaller of the two.
    fn find(&self, target: Key) -> Option<Value<'a>> {
        let mut r = Reader::new(self.entries);
        for _ in 0..self.count {
            let k = r.u32le()?;
            if k == target {
                return decode_value(&mut r, 0);
            }
            if k > target {
                return None;
            }
            skip_value(&mut r, 0)?;
        }
        None
    }

    /// The value at a key, whatever its type.
    ///
    /// The typed accessors below are what schema-shaped props want. This is
    /// for the case they cannot serve: a value whose *shape* varies — an
    /// answer that is a string for one question type and a list for another —
    /// where the caller has to look before it can choose.
    pub fn value(&self, k: Key) -> Option<Value<'a>> {
        self.find(k)
    }

    pub fn has(&self, k: Key) -> bool {
        self.find(k).is_some()
    }

    pub fn i64(&self, k: Key) -> Option<i64> {
        self.find(k)?.i64()
    }

    pub fn f64(&self, k: Key) -> Option<f64> {
        self.find(k)?.f64()
    }

    pub fn bool(&self, k: Key) -> Option<bool> {
        self.find(k)?.bool()
    }

    pub fn str(&self, k: Key) -> Option<&'a str> {
        self.find(k)?.str()
    }

    pub fn bytes(&self, k: Key) -> Option<&'a [u8]> {
        self.find(k)?.bytes()
    }

    pub fn list(&self, k: Key) -> Option<List<'a>> {
        self.find(k)?.as_list()
    }

    pub fn map(&self, k: Key) -> Option<Props<'a>> {
        self.find(k)?.as_map()
    }
}

/// How deep LIST/MAP nesting is followed before a blob is treated as
/// malformed. Not part of the frozen wire format — the ABI does not bound
/// nesting — but recursion depth here is attacker-controlled input on a
/// call stack with no guard page to speak of on `wasm32-unknown-unknown`,
/// so *something* has to cap it. 64 is far past any nesting real props ever
/// use and cheap to check.
const MAX_DEPTH: u8 = 64;

/// A byte cursor with bounds-checked, panic-free reads. Every method returns
/// `None` instead of indexing past the end, and `pos` only ever advances
/// past bytes that were actually there.
struct Reader<'a> {
    data: &'a [u8],
    pos: usize,
}

impl<'a> Reader<'a> {
    fn new(data: &'a [u8]) -> Self {
        Self { data, pos: 0 }
    }

    fn u8(&mut self) -> Option<u8> {
        let b = *self.data.get(self.pos)?;
        self.pos += 1;
        Some(b)
    }

    fn bytes(&mut self, n: usize) -> Option<&'a [u8]> {
        let end = self.pos.checked_add(n)?;
        let s = self.data.get(self.pos..end)?;
        self.pos = end;
        Some(s)
    }

    fn u32le(&mut self) -> Option<u32> {
        let b = self.bytes(4)?;
        Some(u32::from_le_bytes([b[0], b[1], b[2], b[3]]))
    }

    fn f64le(&mut self) -> Option<f64> {
        let b = self.bytes(8)?;
        let mut arr = [0u8; 8];
        arr.copy_from_slice(b);
        Some(f64::from_le_bytes(arr))
    }

    /// LEB128, capped at the 10 bytes a 64-bit value can ever need. The
    /// cap is what keeps a hostile "continuation bit forever" blob from
    /// spinning instead of returning `None`.
    fn uvarint(&mut self) -> Option<u64> {
        let mut result: u64 = 0;
        for i in 0..10u32 {
            let byte = self.u8()?;
            let low7 = (byte & 0x7f) as u64;
            if i == 9 && low7 > 1 {
                // The 10th byte can only contribute the top bit of a u64;
                // anything else means the value does not fit in 64 bits.
                return None;
            }
            result |= low7 << (i * 7);
            if byte & 0x80 == 0 {
                return Some(result);
            }
        }
        None
    }

    fn zigzag_i64(&mut self) -> Option<i64> {
        let u = self.uvarint()?;
        Some(((u >> 1) as i64) ^ -((u & 1) as i64))
    }
}

fn decode_value<'a>(r: &mut Reader<'a>, depth: u8) -> Option<Value<'a>> {
    if depth >= MAX_DEPTH {
        return None;
    }
    match r.u8()? {
        0 => Some(Value::Null),
        1 => Some(Value::Bool(false)),
        2 => Some(Value::Bool(true)),
        3 => Some(Value::Int(r.zigzag_i64()?)),
        4 => Some(Value::Float(r.f64le()?)),
        5 => {
            let len = usize::try_from(r.uvarint()?).ok()?;
            core::str::from_utf8(r.bytes(len)?).ok().map(Value::Str)
        }
        6 => {
            let len = usize::try_from(r.uvarint()?).ok()?;
            Some(Value::Bytes(r.bytes(len)?))
        }
        7 => {
            let n = u32::try_from(r.uvarint()?).ok()?;
            let start = r.pos;
            let mut probe = Reader { data: r.data, pos: start };
            for _ in 0..n {
                skip_value(&mut probe, depth + 1)?;
            }
            let list_bytes = r.data.get(start..probe.pos)?;
            r.pos = probe.pos;
            Some(Value::List(List { data: list_bytes, count: n }))
        }
        8 => {
            let n = u32::try_from(r.uvarint()?).ok()?;
            let start = r.pos;
            let mut probe = Reader { data: r.data, pos: start };
            for _ in 0..n {
                probe.u32le()?;
                skip_value(&mut probe, depth + 1)?;
            }
            let map_bytes = r.data.get(start..probe.pos)?;
            r.pos = probe.pos;
            Some(Value::Map(Props { entries: map_bytes, count: n }))
        }
        _ => None,
    }
}

/// Advance past one value without decoding it, for the entries [`Props::find`]
/// walks over on the way to the target key.
fn skip_value(r: &mut Reader<'_>, depth: u8) -> Option<()> {
    if depth >= MAX_DEPTH {
        return None;
    }
    match r.u8()? {
        0..=2 => Some(()),
        3 => {
            r.zigzag_i64()?;
            Some(())
        }
        4 => {
            r.bytes(8)?;
            Some(())
        }
        5 | 6 => {
            let len = usize::try_from(r.uvarint()?).ok()?;
            r.bytes(len)?;
            Some(())
        }
        7 => {
            let n = r.uvarint()?;
            for _ in 0..n {
                skip_value(r, depth + 1)?;
            }
            Some(())
        }
        8 => {
            let n = r.uvarint()?;
            for _ in 0..n {
                r.u32le()?;
                skip_value(r, depth + 1)?;
            }
            Some(())
        }
        _ => None,
    }
}

/// Implemented for the field types [`props!`](macro@crate::props) supports, so the macro can
/// expand to one `FieldValue::read` call per field instead of matching on
/// the field's type token itself.
pub trait FieldValue<'a>: Sized {
    fn read(props: &Props<'a>, key: Key, default: Self) -> Self;
}

impl<'a> FieldValue<'a> for i64 {
    fn read(props: &Props<'a>, key: Key, default: Self) -> Self {
        props.i64(key).unwrap_or(default)
    }
}

impl<'a> FieldValue<'a> for f64 {
    fn read(props: &Props<'a>, key: Key, default: Self) -> Self {
        props.f64(key).unwrap_or(default)
    }
}

impl<'a> FieldValue<'a> for bool {
    fn read(props: &Props<'a>, key: Key, default: Self) -> Self {
        props.bool(key).unwrap_or(default)
    }
}

impl<'a> FieldValue<'a> for &'a str {
    fn read(props: &Props<'a>, key: Key, default: Self) -> Self {
        props.str(key).unwrap_or(default)
    }
}

/// Typed props without a derive: `props! { pub struct Plan<'a> { count: i64
/// = 0, label: &'a str = "", open: bool = false, price: f64 = 0.0 } }` emits
/// a plain struct plus a `read(&Props<'a>) -> Self` that hashes each field
/// name the same way [`k!`](crate::k) does — at compile time, via a `const` binding —
/// and falls back to the field's default for anything absent, wrong-typed,
/// or past a truncated blob. There is nothing here `Props` itself could not
/// do; this only saves writing out one `k!` lookup per field by hand.
#[macro_export]
macro_rules! props {
    (
        $(#[$meta:meta])*
        $vis:vis struct $name:ident<$lt:lifetime> {
            $( $field:ident : $fty:ty = $default:expr ),* $(,)?
        }
    ) => {
        $(#[$meta])*
        $vis struct $name<$lt> {
            $( pub $field: $fty, )*
        }

        impl<$lt> $name<$lt> {
            pub fn read(props: &$crate::props::Props<$lt>) -> Self {
                Self {
                    $( $field: {
                        const NR_FIELD_KEY: $crate::props::Key = $crate::props::key(stringify!($field));
                        <$fty as $crate::props::FieldValue<$lt>>::read(props, NR_FIELD_KEY, $default)
                    }, )*
                }
            }
        }
    };
}
