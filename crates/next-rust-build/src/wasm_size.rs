//! What each island costs inside the one shared wasm module.
//!
//! [`wasm_islands`](crate::wasm_islands) compiles every island in the
//! project's `islands/` crate into a single `.wasm` module: one instance,
//! one fetch, shared by every island on every page. That is the right
//! tradeoff for the browser, but it hides a build-time question a project
//! author will eventually ask: "I added one more island, why did the shared
//! module grow by 40 KB?" This module answers it by building a call graph
//! out of the module's own bytes and attributing code to whichever islands
//! can actually reach it.
//!
//! The approach, in order:
//! 1. Parse the import section far enough to know how many function indices
//!    the imports occupy (`call` targets are indices into the combined
//!    import+defined function space).
//! 2. Parse the export section for `hydrate_<Island>` functions: one root
//!    per island.
//! 3. Parse the element section (and any `ref.func` instructions) for the
//!    set of functions a table could hold, because...
//! 4. ...walking each function body for `call` and `call_indirect` builds a
//!    call graph, and a `call_indirect` cannot generally be resolved to one
//!    callee at build time. This module treats it as reaching *every*
//!    function that could be in the table: conservative, sometimes
//!    pessimistic, always honest about not knowing which row of the table
//!    gets picked at runtime.
//! 5. Reachability from each island's root, intersected with "how many
//!    islands can reach this", gives exclusive vs. shared bytes.
//!
//! Everything here is read-only and best-effort. A module this walks is one
//! `wasm-opt` already produced for shipping, deep inside a build script:
//! a bug here must never fail the build. Structural sections (import,
//! export, code framing) that fail to parse make the whole report an `Err`
//! — the caller is expected to treat that as "skip the report", same as a
//! missing `wasm-opt`. A single function *body* that this module's opcode
//! table cannot walk (some future SIMD or exception-handling opcode this
//! reader does not know) degrades instead: that function contributes its
//! own byte count but no outgoing edges, as if it called nothing. That can
//! only make the report look *more* shared than reality (a function whose
//! real calls are invisible looks like a dead end reachable from nothing
//! new), never wrong in the direction of hiding cost.
//!
//! What this cannot know, honestly listed:
//! - **`call_indirect` is all-or-nothing.** Every function holding one is
//!   charged for every function any table could hold, even though at
//!   runtime it calls exactly one. On a module with one big vtable (a
//!   `dyn Trait` used by several islands) this makes "exclusive" bytes
//!   look smaller than they are and "shared" larger — the safe direction
//!   for a report meant to explain "why is this bigger than expected", but
//!   not a precise cost model.
//! - **The data section is not attributed to islands at all.** Statics,
//!   string constants, and panic messages live there, not in `code`; this
//!   report only ever divides up code-section bytes. `sections` reports the
//!   data section's total size on its own, unattributed.
//! - **`wasm-opt` inlines.** If it inlines a small helper into two islands'
//!   hydrate paths, that helper's bytes are gone from the module and
//!   duplicated invisibly inside both callers' bodies — they show up as
//!   *exclusive* to each island (correct, that copy really is exclusive to
//!   it) but the two islands look less related than the source code is.
//!   Conversely, if `wasm-opt` outlines a big shared body wasm-opt found
//!   common to only two call sites, it becomes a shared function this
//!   report sees correctly. In short: sizes are honest about the module as
//!   compiled, not about the source it came from.
//! - **A function reachable from zero islands is still "shared".** Per
//!   [`SizeReport::shared`], "shared" is everything that is not exclusive
//!   to exactly one island — that includes the runtime and allocator (only
//!   reachable through non-island exports like `alloc`), and any dead code
//!   `wasm-opt` left behind that nothing in the module can reach.

use std::collections::{HashMap, HashSet};
use std::io::Write;

/// Export prefix marking an island's hydrate function. Duplicated from
/// `wasm_islands.rs` rather than imported: this module is meant to stand
/// alone (see its own doc comment for why).
const PREFIX: &str = "hydrate_";

/// Export kind of a function, both in the export section (kind byte) and
/// the import section (import kind byte): both use `0` for "function".
const KIND_FUNCTION: u8 = 0;

const SEC_CUSTOM: u8 = 0;
const SEC_IMPORT: u8 = 2;
const SEC_EXPORT: u8 = 7;
const SEC_ELEMENT: u8 = 9;
const SEC_CODE: u8 = 10;

const TRUNCATED: &str = "the WebAssembly module is truncated or malformed";

/// What one module's size report produced.
///
/// `total`, `shared` and every `IslandSize` are in bytes. `sections` lists
/// every top-level section the module has, by name, with same-named
/// sections (only possible for repeated custom sections) merged.
#[derive(Debug, Clone)]
pub struct SizeReport {
    /// Size of the whole module, as given to [`report`].
    pub total: usize,
    /// Code-section bytes that are not exclusive to exactly one island:
    /// reached by two or more islands, reached by none of them (the
    /// runtime, the allocator, dead code), or left over as section framing
    /// (the code section's own count and per-function length prefixes,
    /// which do not belong to any one function body). See the module doc
    /// comment for what this cannot know.
    pub shared: usize,
    /// One entry per `hydrate_<Island>` export, sorted by name.
    pub islands: Vec<IslandSize>,
    /// `(section name, bytes)` for every top-level section, in the order
    /// the module lists them. Standard sections are named `type`,
    /// `import`, `function`, `table`, `memory`, `global`, `export`,
    /// `start`, `element`, `code`, `data`, `data_count`; a custom section
    /// is named `custom:<name>` when its name parses, else `custom`.
    pub sections: Vec<(String, usize)>,
}

/// One island's share of the module.
#[derive(Debug, Clone)]
pub struct IslandSize {
    /// The island's name, without the `hydrate_` prefix.
    pub name: String,
    /// Function-body bytes reachable from this island's `hydrate_<Island>`
    /// export and from no other island's.
    pub exclusive: usize,
    /// Function-body bytes reachable from this island's `hydrate_<Island>`
    /// export at all, including bytes also reachable from other islands.
    /// Always `>= exclusive`.
    pub reachable: usize,
}

/// Build a size report for a wasm module, or `Err` when the module cannot
/// be understood at all (not wasm, truncated, an unsupported version, or a
/// structural section — import/export/code framing — that will not parse).
/// An `Err` here means "skip the report", not "fail the build": every
/// caller is expected to treat it that way, same as a missing `wasm-opt`.
pub fn report(wasm: &[u8]) -> Result<SizeReport, String> {
    let parsed = parse_sections(wasm)?;
    let code_total = parsed.sections.iter().find(|(name, _)| name == "code").map_or(0, |(_, n)| *n);

    // Every defined function's body length, and (best effort) its outgoing
    // calls. A body the opcode walker cannot finish contributes its length
    // but no edges: see the module doc comment.
    let mut body_len: HashMap<u32, usize> = HashMap::new();
    let mut indirect_targets: HashSet<u32> = parsed.element_targets;
    let mut infos: HashMap<u32, BodyInfo> = HashMap::new();
    for (index, content) in &parsed.bodies {
        body_len.insert(*index, content.len());
        if let Some(info) = scan_function(content) {
            indirect_targets.extend(info.ref_funcs.iter().copied());
            infos.insert(*index, info);
        }
    }
    let mut edges: HashMap<u32, Vec<u32>> = HashMap::new();
    for (index, info) in &infos {
        let mut targets = info.calls.clone();
        if info.has_indirect {
            targets.extend(indirect_targets.iter().copied());
        }
        edges.insert(*index, targets);
    }

    let mut island_roots: Vec<(String, u32)> = parsed
        .exports
        .iter()
        .filter(|(name, kind, _)| *kind == KIND_FUNCTION && name.len() > PREFIX.len() && name.starts_with(PREFIX))
        .map(|(name, _, index)| (name[PREFIX.len()..].to_owned(), *index))
        .collect();
    island_roots.sort_by(|a, b| a.0.cmp(&b.0));
    island_roots.dedup_by(|a, b| a.0 == b.0);

    let reachable_sets: Vec<(String, HashSet<u32>)> =
        island_roots.iter().map(|(name, root)| (name.clone(), reachable_from(*root, &edges))).collect();

    // For each defined function, which single island (if any) reaches it.
    // `None` once a second, different island reaches the same function.
    let mut owner: HashMap<u32, Option<usize>> = HashMap::new();
    for (i, (_, set)) in reachable_sets.iter().enumerate() {
        for &f in set {
            if !body_len.contains_key(&f) {
                continue; // an imported function: no body bytes to attribute.
            }
            owner
                .entry(f)
                .and_modify(|o| {
                    if *o != Some(i) {
                        *o = None
                    }
                })
                .or_insert(Some(i));
        }
    }

    let mut islands: Vec<IslandSize> = reachable_sets
        .iter()
        .enumerate()
        .map(|(i, (name, set))| {
            let reachable: usize = set.iter().filter_map(|f| body_len.get(f)).sum();
            let exclusive: usize =
                set.iter().filter(|f| owner.get(f).copied() == Some(Some(i))).filter_map(|f| body_len.get(f)).sum();
            IslandSize { name: name.clone(), exclusive, reachable }
        })
        .collect();
    islands.sort_by(|a, b| a.name.cmp(&b.name));

    let exclusive_sum: usize = islands.iter().map(|i| i.exclusive).sum();
    let shared = code_total.saturating_sub(exclusive_sum);

    Ok(SizeReport { total: wasm.len(), shared, islands, sections: parsed.sections })
}

/// Brotli-compressed length of `bytes` at the highest quality, or `None` if
/// compression fails. `next-rust-build` already depends on the `brotli`
/// crate in-process (see `codegen.rs`, which precompresses `assets/` and
/// `client/` output the same way) so this compresses in memory rather than
/// shelling out to a `brotli` binary — no `PATH` lookup, nothing to be
/// missing in the environment a build script runs in.
pub fn brotli_len(bytes: &[u8]) -> Option<usize> {
    let mut out = Vec::with_capacity(bytes.len() / 2);
    {
        let mut writer = brotli::CompressorWriter::new(&mut out, 4096, 11, 22);
        if writer.write_all(bytes).is_err() {
            return None;
        }
    }
    Some(out.len())
}

/// Depth-first reachability from `root` over `edges`, including `root`
/// itself. Terminates on any graph (cycles included) because it never
/// revisits a function once seen.
fn reachable_from(root: u32, edges: &HashMap<u32, Vec<u32>>) -> HashSet<u32> {
    let mut seen = HashSet::new();
    let mut stack = vec![root];
    seen.insert(root);
    while let Some(f) = stack.pop() {
        if let Some(callees) = edges.get(&f) {
            for &callee in callees {
                if seen.insert(callee) {
                    stack.push(callee);
                }
            }
        }
    }
    seen
}

/// What one function body's `call`, `call_indirect` and `ref.func`
/// instructions add to the call graph.
struct BodyInfo {
    calls: Vec<u32>,
    has_indirect: bool,
    ref_funcs: Vec<u32>,
}

/// Everything [`parse_sections`] pulls out of the module in one pass.
struct Parsed<'a> {
    sections: Vec<(String, usize)>,
    exports: Vec<(String, u8, u32)>,
    element_targets: HashSet<u32>,
    /// `(function index, body content)` for every function the code section
    /// defines, content being its locals declarations plus its expression,
    /// exactly as long as the code entry declared.
    bodies: Vec<(u32, &'a [u8])>,
}

/// Walk the module's top-level sections once, collecting section sizes and
/// the few sections this report needs to look inside. Assumes sections
/// appear in the module's canonical order (imports before code), which is
/// a hard requirement of the format and something `wasm-opt` preserves; a
/// module that violates it is already invalid wasm, and at worst this
/// mis-numbers functions rather than panicking.
fn parse_sections(wasm: &[u8]) -> Result<Parsed<'_>, String> {
    let mut reader = Reader::new(wasm);
    let header = reader.take(8).ok_or("the WebAssembly module is truncated: no header")?;
    if &header[..4] != b"\0asm" {
        return Err("not a WebAssembly module: the magic bytes are missing".to_owned());
    }
    if header[4..] != [1, 0, 0, 0] {
        return Err(format!("unsupported WebAssembly version {:?}", &header[4..]));
    }

    let mut sections: Vec<(String, usize)> = Vec::new();
    let mut num_import_funcs: u32 = 0;
    let mut exports: Vec<(String, u8, u32)> = Vec::new();
    let mut element_targets: HashSet<u32> = HashSet::new();
    let mut bodies: Vec<(u32, &[u8])> = Vec::new();

    while !reader.done() {
        let id = reader.byte().ok_or(TRUNCATED)?;
        let len = reader.usize().ok_or(TRUNCATED)?;
        let body = reader.take(len).ok_or(TRUNCATED)?;
        add_section(&mut sections, section_name(id, body), body.len());
        match id {
            SEC_IMPORT => num_import_funcs = count_import_funcs(body)?,
            SEC_EXPORT => exports = parse_exports(body)?,
            // Best-effort: an element section this reader cannot follow
            // just means no indirect targets are known, which can only
            // under-state `call_indirect`'s reach (see the module doc
            // comment on `call_indirect` being all-or-nothing already).
            SEC_ELEMENT => element_targets = parse_element_targets(body).unwrap_or_default(),
            SEC_CODE => bodies = parse_code(body, num_import_funcs)?,
            _ => {}
        }
    }
    Ok(Parsed { sections, exports, element_targets, bodies })
}

fn add_section(sections: &mut Vec<(String, usize)>, name: String, len: usize) {
    match sections.iter_mut().find(|(existing, _)| *existing == name) {
        Some(entry) => entry.1 += len,
        None => sections.push((name, len)),
    }
}

fn section_name(id: u8, body: &[u8]) -> String {
    match id {
        SEC_CUSTOM => custom_section_name(body),
        1 => "type".to_owned(),
        SEC_IMPORT => "import".to_owned(),
        3 => "function".to_owned(),
        4 => "table".to_owned(),
        5 => "memory".to_owned(),
        6 => "global".to_owned(),
        SEC_EXPORT => "export".to_owned(),
        8 => "start".to_owned(),
        SEC_ELEMENT => "element".to_owned(),
        SEC_CODE => "code".to_owned(),
        11 => "data".to_owned(),
        12 => "data_count".to_owned(),
        other => format!("section-{other}"),
    }
}

fn custom_section_name(body: &[u8]) -> String {
    let mut reader = Reader::new(body);
    let name = reader.usize().and_then(|len| reader.take(len)).and_then(|bytes| std::str::from_utf8(bytes).ok());
    match name {
        Some(name) if !name.is_empty() => format!("custom:{name}"),
        _ => "custom".to_owned(),
    }
}

/// How many function indices the import section occupies. Function indices
/// used by `call` count imported functions first, in import-section order,
/// before any function the module itself defines.
fn count_import_funcs(body: &[u8]) -> Result<u32, String> {
    let mut reader = Reader::new(body);
    let count = reader.usize().ok_or(TRUNCATED)?;
    let mut num_funcs: u32 = 0;
    for _ in 0..count {
        skip_name(&mut reader)?; // module
        skip_name(&mut reader)?; // field
        match reader.byte().ok_or(TRUNCATED)? {
            KIND_FUNCTION => {
                reader.leb128().ok_or(TRUNCATED)?; // typeidx
                num_funcs = num_funcs.checked_add(1).ok_or(TRUNCATED)?;
            }
            1 => {
                reader.byte().ok_or(TRUNCATED)?; // reftype
                skip_limits(&mut reader)?;
            }
            2 => skip_limits(&mut reader)?,
            3 => {
                reader.byte().ok_or(TRUNCATED)?; // valtype
                reader.byte().ok_or(TRUNCATED)?; // mutability
            }
            _ => return Err(format!("{TRUNCATED} (an unrecognised import kind)")),
        }
    }
    Ok(num_funcs)
}

fn skip_name(reader: &mut Reader) -> Result<(), String> {
    let len = reader.usize().ok_or(TRUNCATED)?;
    reader.take(len).ok_or(TRUNCATED)?;
    Ok(())
}

/// A `limits` record: a flag byte (bit 0 says whether a maximum follows),
/// then one or two LEB128 numbers. Shared with table and memory imports.
fn skip_limits(reader: &mut Reader) -> Result<(), String> {
    let flags = reader.byte().ok_or(TRUNCATED)?;
    reader.leb128().ok_or(TRUNCATED)?; // minimum
    if flags & 1 != 0 {
        reader.leb128().ok_or(TRUNCATED)?; // maximum
    }
    Ok(())
}

fn parse_exports(body: &[u8]) -> Result<Vec<(String, u8, u32)>, String> {
    let mut reader = Reader::new(body);
    let count = reader.usize().ok_or(TRUNCATED)?;
    let mut out = Vec::with_capacity(count.min(1024));
    for _ in 0..count {
        let len = reader.usize().ok_or(TRUNCATED)?;
        let name = reader.take(len).ok_or(TRUNCATED)?;
        let name = std::str::from_utf8(name).map_err(|_| "an export name is not valid UTF-8".to_owned())?;
        let kind = reader.byte().ok_or(TRUNCATED)?;
        let index = reader.usize().ok_or(TRUNCATED)?;
        let index = u32::try_from(index).map_err(|_| TRUNCATED.to_owned())?;
        out.push((name.to_owned(), kind, index));
    }
    Ok(out)
}

fn parse_code(body: &[u8], num_import_funcs: u32) -> Result<Vec<(u32, &[u8])>, String> {
    let mut reader = Reader::new(body);
    let count = reader.usize().ok_or(TRUNCATED)?;
    let mut out = Vec::with_capacity(count.min(4096));
    for i in 0..count {
        let len = reader.usize().ok_or(TRUNCATED)?;
        let content = reader.take(len).ok_or(TRUNCATED)?;
        let i = u32::try_from(i).map_err(|_| TRUNCATED.to_owned())?;
        let index = num_import_funcs.checked_add(i).ok_or(TRUNCATED)?;
        out.push((index, content));
    }
    Ok(out)
}

/// Every function index an element segment could put in a table, plus every
/// function index referenced by a `ref.func` inside one of its init
/// expressions. Handles all eight element-segment encodings (active,
/// passive and declarative, crossed with the plain-funcidx and
/// typed-expression forms); returns `None` on anything else rather than
/// guessing at layout, and the caller treats that as "no targets known".
fn parse_element_targets(body: &[u8]) -> Option<HashSet<u32>> {
    let mut reader = Reader::new(body);
    let count = reader.usize()?;
    let mut targets = HashSet::new();
    for _ in 0..count {
        match reader.usize()? {
            0 => {
                skip_const_expr(&mut reader)?;
                read_funcidx_vec(&mut reader, &mut targets)?;
            }
            1 | 3 => {
                reader.byte()?; // elemkind
                read_funcidx_vec(&mut reader, &mut targets)?;
            }
            2 => {
                reader.leb128()?; // tableidx
                skip_const_expr(&mut reader)?;
                reader.byte()?; // elemkind
                read_funcidx_vec(&mut reader, &mut targets)?;
            }
            4 => {
                skip_const_expr(&mut reader)?;
                read_expr_vec(&mut reader, &mut targets)?;
            }
            5 | 7 => {
                reader.byte()?; // reftype
                read_expr_vec(&mut reader, &mut targets)?;
            }
            6 => {
                reader.leb128()?; // tableidx
                skip_const_expr(&mut reader)?;
                reader.byte()?; // reftype
                read_expr_vec(&mut reader, &mut targets)?;
            }
            _ => return None,
        }
    }
    Some(targets)
}

fn read_funcidx_vec(reader: &mut Reader, out: &mut HashSet<u32>) -> Option<()> {
    let count = reader.usize()?;
    for _ in 0..count {
        out.insert(u32::try_from(reader.usize()?).ok()?);
    }
    Some(())
}

/// A `vec(expr)`, as flags 4-7 use: each element is itself a tiny constant
/// expression (typically `ref.func $i end` or `ref.null func end`), so a
/// `ref.func` reached while skipping one is a genuine table target.
fn read_expr_vec(reader: &mut Reader, out: &mut HashSet<u32>) -> Option<()> {
    let count = reader.usize()?;
    for _ in 0..count {
        for insn in ExprInsns::new(reader) {
            if let Insn::RefFunc(index) = insn? {
                out.insert(index);
            }
        }
    }
    Some(())
}

/// One constant expression: instructions up to and including its
/// terminating `end` (0x0B). Used for element-segment offsets, where the
/// value itself (almost always `i32.const <n>`) is not needed, only that
/// the right number of bytes gets skipped.
fn skip_const_expr(reader: &mut Reader) -> Option<()> {
    for insn in ExprInsns::new(reader) {
        insn?;
    }
    Some(())
}

/// Iterates one constant expression's instructions, stopping after (and
/// consuming) its terminating `end`. Yields `None` and stops early if an
/// instruction cannot be decoded.
struct ExprInsns<'a, 'b> {
    reader: &'b mut Reader<'a>,
    done: bool,
}

impl<'a, 'b> ExprInsns<'a, 'b> {
    fn new(reader: &'b mut Reader<'a>) -> Self {
        Self { reader, done: false }
    }
}

impl Iterator for ExprInsns<'_, '_> {
    type Item = Option<Insn>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.done {
            return None;
        }
        let start = self.reader.pos;
        match self.reader.byte() {
            Some(0x0B) => {
                self.done = true;
                None
            }
            Some(_) => {
                self.reader.pos = start;
                match decode_one(self.reader) {
                    Some(insn) => Some(Some(insn)),
                    None => {
                        self.done = true;
                        Some(None)
                    }
                }
            }
            None => {
                self.done = true;
                Some(None)
            }
        }
    }
}

/// Strip a function body's local-declarations prefix and walk its
/// expression for `call`, `call_indirect` and `ref.func`. `None` if the
/// locals prefix itself does not parse (rare: only a body wasm-opt
/// mangled); an unwalkable expression still returns `Some`, just with
/// whatever was found before the first unrecognised opcode discarded (see
/// `scan_body`).
fn scan_function(content: &[u8]) -> Option<BodyInfo> {
    let mut reader = Reader::new(content);
    let local_groups = reader.usize()?;
    for _ in 0..local_groups {
        reader.leb128()?; // how many locals in this group
        reader.byte()?; // their shared value type
    }
    let expr = reader.rest();
    scan_body(expr)
}

/// Walk one function's expression bytes end to end. Returns `None` (instead
/// of a partial result) the moment an opcode cannot be decoded, so a body
/// this reader cannot fully account for never contributes a wrong or
/// half-true set of edges — only nothing, per the module doc comment.
fn scan_body(expr: &[u8]) -> Option<BodyInfo> {
    let mut reader = Reader::new(expr);
    let mut info = BodyInfo { calls: Vec::new(), has_indirect: false, ref_funcs: Vec::new() };
    while !reader.done() {
        match decode_one(&mut reader)? {
            Insn::Call(index) => info.calls.push(index),
            Insn::CallIndirect => info.has_indirect = true,
            Insn::RefFunc(index) => info.ref_funcs.push(index),
            Insn::Other => {}
        }
    }
    Some(info)
}

/// The one piece of information this report needs out of an instruction:
/// whether it is a direct call, an indirect one, or a function reference —
/// everything else is `Other` once its immediate is safely skipped.
enum Insn {
    Call(u32),
    CallIndirect,
    RefFunc(u32),
    Other,
}

/// Decode one instruction's opcode and immediate, positioned at the opcode
/// byte. Returns `None` on any opcode this table does not recognise
/// (exception-handling, SIMD, atomics, GC — proposals a hand-written,
/// `panic = "abort"`, `wasm32-unknown-unknown` islands crate has no reason
/// to emit) rather than guessing at an immediate shape it does not know,
/// per the module doc comment on graceful degradation.
fn decode_one(reader: &mut Reader) -> Option<Insn> {
    match reader.byte()? {
        0x00 | 0x01 | 0x05 | 0x0B | 0x0F | 0x1A | 0x1B | 0xD1 => Some(Insn::Other),
        // block, loop, if: one blocktype, itself a LEB128-shaped value
        // (a plain byte or a signed 33-bit type index) — decoding the
        // value is unnecessary, only its byte length matters here.
        0x02..=0x04 => {
            reader.leb128()?;
            Some(Insn::Other)
        }
        // br, br_if
        0x0C | 0x0D => {
            reader.leb128()?;
            Some(Insn::Other)
        }
        // br_table: vec(labelidx) + one default labelidx
        0x0E => {
            let count = reader.usize()?;
            for _ in 0..=count {
                reader.leb128()?;
            }
            Some(Insn::Other)
        }
        // call, return_call (tail calls): funcidx
        0x10 | 0x12 => Some(Insn::Call(u32::try_from(reader.usize()?).ok()?)),
        // call_indirect, return_call_indirect: typeidx, tableidx
        0x11 | 0x13 => {
            reader.leb128()?;
            reader.leb128()?;
            Some(Insn::CallIndirect)
        }
        // select t*: vec(valtype), each one raw byte
        0x1C => {
            let count = reader.usize()?;
            reader.take(count)?;
            Some(Insn::Other)
        }
        // local.get/set/tee, global.get/set, table.get/set
        0x20..=0x26 => {
            reader.leb128()?;
            Some(Insn::Other)
        }
        // loads and stores: memarg (align, offset)
        0x28..=0x3E => {
            reader.leb128()?;
            reader.leb128()?;
            Some(Insn::Other)
        }
        // memory.size, memory.grow: one reserved/memidx LEB128
        0x3F | 0x40 => {
            reader.leb128()?;
            Some(Insn::Other)
        }
        // i32.const, i64.const: one LEB128
        0x41 | 0x42 => {
            reader.leb128()?;
            Some(Insn::Other)
        }
        0x43 => {
            reader.take(4)?; // f32.const: 4 raw bytes, not LEB128
            Some(Insn::Other)
        }
        0x44 => {
            reader.take(8)?; // f64.const: 8 raw bytes, not LEB128
            Some(Insn::Other)
        }
        // every comparison, unary, binary, conversion and sign-extension
        // numeric op: no immediate at all.
        0x45..=0xC4 => Some(Insn::Other),
        0xD0 => {
            reader.leb128()?; // ref.null: one heaptype byte, LEB128-shaped
            Some(Insn::Other)
        }
        0xD2 => Some(Insn::RefFunc(u32::try_from(reader.usize()?).ok()?)),
        // the 0xFC "misc" prefix: saturating truncation (no extra
        // immediate) and bulk-memory/table ops (0-2 index LEB128s each).
        0xFC => {
            let extra = match reader.usize()? {
                0..=7 => 0,
                8 | 10 | 12 | 14 => 2,
                9 | 11 | 13 | 15 | 16 | 17 => 1,
                _ => return None,
            };
            for _ in 0..extra {
                reader.leb128()?;
            }
            Some(Insn::Other)
        }
        _ => None,
    }
}

/// A cursor over a wasm binary. Every read is fallible: a truncated or
/// malformed module returns `None`, never panics. Duplicated from
/// `wasm_islands.rs` (see this module's doc comment on why) rather than
/// shared, plus a `rest()` accessor that file has no need for.
struct Reader<'a> {
    bytes: &'a [u8],
    pos: usize,
}

impl<'a> Reader<'a> {
    fn new(bytes: &'a [u8]) -> Self {
        Self { bytes, pos: 0 }
    }

    fn done(&self) -> bool {
        self.pos >= self.bytes.len()
    }

    fn byte(&mut self) -> Option<u8> {
        let byte = *self.bytes.get(self.pos)?;
        self.pos += 1;
        Some(byte)
    }

    fn take(&mut self, len: usize) -> Option<&'a [u8]> {
        let end = self.pos.checked_add(len)?;
        let slice = self.bytes.get(self.pos..end)?;
        self.pos = end;
        Some(slice)
    }

    /// Everything from the current position to the end.
    fn rest(&self) -> &'a [u8] {
        // `pos` never exceeds `bytes.len()`: `byte()` and `take()` are the
        // only ways to advance it, and both are bounds-checked.
        &self.bytes[self.pos..]
    }

    /// An unsigned LEB128 integer. Over-long encodings are rejected rather
    /// than wrapped around. Also used to skip signed LEB128 immediates
    /// (`i32.const` and friends): decoding is driven by each byte's
    /// continuation bit regardless of how the value is meant to be signed,
    /// so the byte length this consumes is correct either way, even though
    /// the signed value itself is not reconstructed here.
    fn leb128(&mut self) -> Option<u64> {
        let mut value: u64 = 0;
        for shift in 0..10u32 {
            let byte = self.byte()?;
            let bits = u64::from(byte & 0x7f);
            value |= bits.checked_shl(shift * 7).filter(|v| v >> (shift * 7) == bits)?;
            if byte & 0x80 == 0 {
                return Some(value);
            }
        }
        None
    }

    fn usize(&mut self) -> Option<usize> {
        usize::try_from(self.leb128()?).ok()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const EXPORT_SECTION: u8 = SEC_EXPORT;
    const ELEMENT_SECTION: u8 = SEC_ELEMENT;
    const IMPORT_SECTION: u8 = SEC_IMPORT;
    const CODE_SECTION: u8 = SEC_CODE;

    /// Unsigned LEB128, as the wasm binary format writes lengths and counts.
    fn leb(mut value: u64) -> Vec<u8> {
        let mut out = Vec::new();
        loop {
            let byte = (value & 0x7f) as u8;
            value >>= 7;
            if value == 0 {
                out.push(byte);
                return out;
            }
            out.push(byte | 0x80);
        }
    }

    fn name_bytes(name: &str) -> Vec<u8> {
        let mut out = leb(name.len() as u64);
        out.extend(name.as_bytes());
        out
    }

    fn section(id: u8, body: Vec<u8>) -> Vec<u8> {
        let mut out = vec![id];
        out.extend(leb(body.len() as u64));
        out.extend(body);
        out
    }

    /// A minimal valid function body: no locals, calling every function
    /// index in `calls` in order, then `nop`ping `pad` extra times so
    /// otherwise-identical bodies can be told apart by length, then `end`.
    fn body(calls: &[u32], pad: usize, call_indirect: bool) -> Vec<u8> {
        let mut out = vec![0]; // zero local-declaration groups
        for target in calls {
            out.push(0x10); // call
            out.extend(leb(u64::from(*target)));
        }
        if call_indirect {
            out.push(0x11); // call_indirect
            out.extend(leb(0)); // typeidx
            out.extend(leb(0)); // tableidx
        }
        out.extend(std::iter::repeat_n(0x01, pad)); // nop, nop, ...
        out.push(0x0B); // end
        out
    }

    fn code_section(bodies: &[Vec<u8>]) -> Vec<u8> {
        let mut out = leb(bodies.len() as u64);
        for b in bodies {
            out.extend(leb(b.len() as u64));
            out.extend(b.clone());
        }
        out
    }

    fn export_section(entries: &[(&str, u8, u32)]) -> Vec<u8> {
        let mut out = leb(entries.len() as u64);
        for (name, kind, index) in entries {
            out.extend(name_bytes(name));
            out.push(*kind);
            out.extend(leb(u64::from(*index)));
        }
        out
    }

    /// A module with an export section, an optional element section and a
    /// code section — everything `report()` looks at, nothing it does not.
    fn module(exports: &[(&str, u8, u32)], element: Option<Vec<u8>>, bodies: &[Vec<u8>]) -> Vec<u8> {
        let mut wasm = b"\0asm\x01\0\0\0".to_vec();
        wasm.extend(section(EXPORT_SECTION, export_section(exports)));
        if let Some(el) = element {
            wasm.extend(section(ELEMENT_SECTION, el));
        }
        wasm.extend(section(CODE_SECTION, code_section(bodies)));
        wasm
    }

    #[test]
    fn a_helper_shared_by_two_islands_is_shared_not_exclusive() {
        // func0 hydrate_A -> calls 2 (shared) and 3 (a-only)
        // func1 hydrate_B -> calls 2 (shared) and 4 (b-only)
        // func2 shared_helper, func3 a_only, func4 b_only, func5 unreachable
        let f0 = body(&[2, 3], 0, false);
        let f1 = body(&[2, 4], 0, false);
        let f2 = body(&[], 0, false);
        let f3 = body(&[], 1, false);
        let f4 = body(&[], 2, false);
        let f5 = body(&[], 3, false);
        let bodies = [f0.clone(), f1.clone(), f2.clone(), f3.clone(), f4.clone(), f5.clone()];
        let wasm = module(
            &[("hydrate_A", KIND_FUNCTION, 0), ("hydrate_B", KIND_FUNCTION, 1), ("memory", 2, 0)],
            None,
            &bodies,
        );

        let report = report(&wasm).expect("a well-formed module reports");
        assert_eq!(report.total, wasm.len());
        assert_eq!(report.islands.len(), 2);
        let a = &report.islands[0];
        let b = &report.islands[1];
        assert_eq!(a.name, "A");
        assert_eq!(b.name, "B");
        assert_eq!(a.exclusive, f0.len() + f3.len());
        assert_eq!(a.reachable, f0.len() + f2.len() + f3.len());
        assert_eq!(b.exclusive, f1.len() + f4.len());
        assert_eq!(b.reachable, f1.len() + f2.len() + f4.len());
        // The rest of the code section — the shared helper, the
        // unreachable function, and the code section's own framing bytes
        // (its count and each entry's length prefix) — is exactly `shared`.
        let code_total = report.sections.iter().find(|(n, _)| n == "code").unwrap().1;
        assert_eq!(report.shared, code_total - a.exclusive - b.exclusive);
        assert!(report.shared >= f2.len() + f5.len());
    }

    #[test]
    fn call_indirect_reaches_every_function_an_element_segment_lists() {
        // One function import shifts every defined function index by one.
        let mut imports = leb(1);
        imports.extend(name_bytes("e"));
        imports.extend(name_bytes("h"));
        imports.push(KIND_FUNCTION);
        imports.extend(leb(0)); // typeidx
        let mut wasm = b"\0asm\x01\0\0\0".to_vec();
        wasm.extend(section(IMPORT_SECTION, imports));

        // Defined functions: 1 = hydrate_Only (call_indirect), 2..=4 the
        // table's targets. Only one is ever really called at runtime; the
        // report cannot know which, so it charges all three.
        let f1 = body(&[], 0, true);
        let f2 = body(&[], 0, false);
        let f3 = body(&[], 1, false);
        let f4 = body(&[], 2, false);

        // Element segment, flags=0 (active, table 0, offset `i32.const 0`,
        // plain funcidx vec): targets function indices 2, 3, 4.
        let mut element = leb(1); // one segment
        element.extend(leb(0)); // flags
        element.push(0x41); // i32.const
        element.extend(leb(0));
        element.push(0x0B); // end
        element.extend(leb(3)); // 3 targets
        element.extend(leb(2));
        element.extend(leb(3));
        element.extend(leb(4));

        wasm.extend(section(EXPORT_SECTION, export_section(&[("hydrate_Only", KIND_FUNCTION, 1)])));
        wasm.extend(section(ELEMENT_SECTION, element));
        wasm.extend(section(CODE_SECTION, code_section(&[f1.clone(), f2.clone(), f3.clone(), f4.clone()])));

        let report = report(&wasm).expect("a well-formed module reports");
        assert_eq!(report.islands.len(), 1);
        let only = &report.islands[0];
        assert_eq!(only.reachable, f1.len() + f2.len() + f3.len() + f4.len());
        // The only island in the module reaches all of it, so it is all exclusive.
        assert_eq!(only.exclusive, only.reachable);
    }

    #[test]
    fn ref_func_inside_a_typed_element_segment_still_counts_as_a_target() {
        let f0 = body(&[], 0, true); // hydrate_Only
        let f1 = body(&[], 0, false);
        let f2 = body(&[], 1, false);

        // flags=4 (active, table 0, offset expr, then vec(expr) of
        // funcref-typed constant exprs): two `ref.func` entries.
        let mut element = leb(1);
        element.extend(leb(4)); // flags
        element.push(0x41); // i32.const
        element.extend(leb(0));
        element.push(0x0B); // end
        element.extend(leb(2)); // 2 exprs
        element.push(0xD2); // ref.func
        element.extend(leb(1));
        element.push(0x0B); // end
        element.push(0xD2); // ref.func
        element.extend(leb(2));
        element.push(0x0B); // end

        let wasm = module(&[("hydrate_Only", KIND_FUNCTION, 0)], Some(element), &[f0.clone(), f1.clone(), f2.clone()]);
        let report = report(&wasm).expect("a well-formed module reports");
        assert_eq!(report.islands[0].reachable, f0.len() + f1.len() + f2.len());
    }

    #[test]
    fn a_module_with_no_code_section_reports_zero_everywhere() {
        let mut wasm = b"\0asm\x01\0\0\0".to_vec();
        wasm.extend(section(EXPORT_SECTION, export_section(&[("hydrate_A", KIND_FUNCTION, 0)])));
        let report = report(&wasm).expect("a module without code still reports");
        assert_eq!(report.islands.len(), 1);
        assert_eq!(report.islands[0].exclusive, 0);
        assert_eq!(report.islands[0].reachable, 0); // the root itself has no known body length
        assert_eq!(report.shared, 0);
        assert!(report.sections.iter().all(|(n, _)| n != "code"));
    }

    #[test]
    fn a_module_with_no_exports_has_no_islands_and_everything_is_shared() {
        let f0 = body(&[], 4, false);
        let wasm = module(&[], None, std::slice::from_ref(&f0));
        let report = report(&wasm).expect("a module without islands still reports");
        assert!(report.islands.is_empty());
        let code_total = report.sections.iter().find(|(n, _)| n == "code").unwrap().1;
        assert_eq!(report.shared, code_total);
    }

    #[test]
    fn sections_are_named_and_merged_by_name() {
        let mut wasm = b"\0asm\x01\0\0\0".to_vec();
        wasm.extend(section(0, {
            let mut c = name_bytes("producers");
            c.extend([1, 2, 3]);
            c
        }));
        wasm.extend(section(EXPORT_SECTION, export_section(&[])));
        wasm.extend(section(CODE_SECTION, code_section(&[])));
        // A second custom section with the same name: merged, not duplicated.
        wasm.extend(section(0, name_bytes("producers")));
        let report = report(&wasm).unwrap();
        let names: Vec<&str> = report.sections.iter().map(|(n, _)| n.as_str()).collect();
        assert_eq!(names.iter().filter(|n| **n == "custom:producers").count(), 1);
        assert!(names.contains(&"export"));
        assert!(names.contains(&"code"));
    }

    #[test]
    fn broken_modules_are_errors_not_panics() {
        assert!(report(b"<!doctype html>").is_err());
        assert!(report(b"").is_err());
        assert!(report(b"\0asm\x02\0\0\0").is_err());
        assert!(report(b"\0asm\x01\0\0\0").is_ok()); // header alone: no sections, that is fine

        let f0 = body(&[2, 3], 0, false);
        let f1 = body(&[], 0, false);
        let f2 = body(&[], 1, false);
        let wasm = module(&[("hydrate_A", KIND_FUNCTION, 0)], None, &[f0, f1, f2]);
        // Every prefix of a well-formed module either reports or errors —
        // it never panics, and it never fabricates an island that was not
        // actually declared.
        for len in 0..wasm.len() {
            match report(&wasm[..len]) {
                Ok(r) => assert!(r.islands.len() <= 1, "an island materialised out of a truncated module"),
                Err(message) => assert!(!message.is_empty()),
            }
        }
    }

    #[test]
    fn an_unrecognised_opcode_drops_that_bodys_edges_without_failing() {
        // 0xFF is not an opcode this table knows: the body degrades to
        // "attribute nothing" rather than the whole report failing.
        let mut weird = vec![0]; // no locals
        weird.push(0xFF);
        weird.push(0x0B);
        let f0 = weird;
        let f1 = body(&[], 0, false);
        // hydrate_A "calls" nothing usable because its body is unwalkable;
        // it should still report its own size, just with no edges.
        let wasm = module(&[("hydrate_A", KIND_FUNCTION, 0)], None, &[f0.clone(), f1.clone()]);
        let report = report(&wasm).expect("an unwalkable body does not fail the report");
        assert_eq!(report.islands[0].reachable, f0.len());
        assert_eq!(report.islands[0].exclusive, f0.len());
    }

    #[test]
    fn import_kinds_other_than_function_do_not_shift_function_indices() {
        // table, memory and global imports in between two function imports:
        // only the func imports should count toward the function index base.
        let mut imports = leb(5);
        let func_typeidx = leb(0);
        let entries: [(u8, &[u8]); 5] = [
            (KIND_FUNCTION, &func_typeidx),
            (1, &[0x70, 0x00, 0x01]), // table: funcref, limits{flags:0,min:1}
            (2, &[0x01, 0x01, 0x02]), // memory: limits{flags:1,min:1,max:2}
            (3, &[0x7F, 0x00]),       // global: i32, immutable
            (KIND_FUNCTION, &func_typeidx),
        ];
        for (kind, rest) in entries {
            imports.extend(name_bytes("e"));
            imports.extend(name_bytes("x"));
            imports.push(kind);
            imports.extend(rest);
        }
        let mut wasm = b"\0asm\x01\0\0\0".to_vec();
        wasm.extend(section(IMPORT_SECTION, imports));
        // Two function imports precede the defined functions, so hydrate_A is index 2.
        wasm.extend(section(EXPORT_SECTION, export_section(&[("hydrate_A", KIND_FUNCTION, 2)])));
        wasm.extend(section(CODE_SECTION, code_section(&[body(&[], 0, false)])));
        let report = report(&wasm).expect("mixed import kinds parse");
        assert_eq!(report.islands[0].reachable, body(&[], 0, false).len());
    }

    #[test]
    fn brotli_len_shrinks_repetitive_data_and_does_not_panic_on_empty_input() {
        let repetitive = vec![b'a'; 4096];
        let compressed = brotli_len(&repetitive).expect("brotli always succeeds on a Vec sink");
        assert!(compressed < repetitive.len(), "{compressed} should be much smaller than {}", repetitive.len());
        // Empty input still costs a byte or two of brotli framing (the
        // final-block marker); the point of this assertion is just that it
        // does not panic and does not return `None`.
        assert!(brotli_len(&[]).is_some_and(|n| n <= 4));
    }

    #[test]
    fn leb128_reads_what_it_writes() {
        for value in [0u64, 1, 63, 64, 127, 128, 255, 624_485, u32::MAX as u64, u64::MAX] {
            let bytes = leb(value);
            assert_eq!(Reader::new(&bytes).leb128(), Some(value), "{value}");
        }
        assert_eq!(Reader::new(&[0x80; 10]).leb128(), None);
        assert_eq!(Reader::new(&[0xff; 11]).leb128(), None);
    }
}
