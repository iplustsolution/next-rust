//! The executor islands await on.
//!
//! Everything asynchronous a browser does ends in a callback, and the shim
//! turns every one of those callbacks into a single call to [`on_resume`].
//! This module is the other half of that: a table of spawned futures, a table
//! of outstanding tokens, and a ready queue joining the two. There is no
//! reactor, no timer wheel and no thread here, because the browser already is
//! the reactor — the only thing missing was somewhere to put a future while
//! the event loop gets on with it.
//!
//! Wasm islands run on the browser's single thread, which is what makes the
//! `static mut` tables below sound: there is no second thread to race with and
//! no way to re-enter Rust except through an export, and every export lands in
//! `drain`, which is not re-entrant. Each access goes through a raw pointer
//! rather than a reference, because a reference to a `static mut` is exactly
//! what the `static_mut_refs` lint forbids.
//!
//! Off the browser every host call answers zero, so nothing ever resumes and
//! an awaited future simply stays pending forever. That is deliberate: a test
//! binary that links this must never hang the process or trap, and a future
//! that is never woken does neither.

use alloc::boxed::Box;
use alloc::vec::Vec;
use core::future::Future;
use core::pin::Pin;
use core::task::{Context, Poll, RawWaker, RawWakerVTable, Waker};

use crate::host::{call1, op};

/// The resume carried a value.
pub const TAG_VALUE: u8 = 0;
/// The resume carried an error.
pub const TAG_ERROR: u8 = 1;
/// The stream behind this token ended; no further resume will arrive for it.
pub const TAG_END: u8 = 2;

/// One resume, decoded out of the `[u8 tag][u16le status][payload…]` blob the
/// shim hands to [`on_resume`].
///
/// A blob too short to hold the header is not an error: a missing tag reads as
/// [`TAG_VALUE`] and a missing status as zero, so a truncated or empty payload
/// resolves the future with nothing rather than trapping inside the island.
pub struct Msg {
    pub tag: u8,
    pub status: u16,
    pub body: Vec<u8>,
}

/// Decode a resume blob. Kept separate from [`on_resume`] so the payload
/// parsers can be driven from a test without a browser or a raw pointer.
pub(crate) fn decode(bytes: &[u8]) -> Msg {
    let status = match (bytes.get(1), bytes.get(2)) {
        (Some(low), Some(high)) => u16::from_le_bytes([*low, *high]),
        _ => 0,
    };
    Msg { tag: bytes.first().copied().unwrap_or(TAG_VALUE), status, body: bytes.get(3..).unwrap_or(&[]).to_vec() }
}

// --- the task table ----------------------------------------------------------

type Boxed = Pin<Box<dyn Future<Output = ()>>>;

/// One spawned future.
///
/// `fut` is empty while the future is being polled — it is moved out for the
/// duration and put back afterwards — which is what makes a resume that
/// arrives *during* a poll safe: the resume only appends to a queue and to the
/// ready list, and never touches the future itself. `live` is separate from
/// `fut.is_some()` for the same reason, so a slot whose future is out on loan
/// is not handed to the next [`spawn`].
struct Slot {
    fut: Option<Boxed>,
    live: bool,
}

static mut TASKS: Vec<Slot> = Vec::new();
static mut READY: Vec<u32> = Vec::new();
static mut DRAINING: bool = false;

#[allow(clippy::deref_addrof)]
fn tasks() -> &'static mut Vec<Slot> {
    // SAFETY: single-threaded, and no caller holds this across a poll.
    unsafe { &mut *(&raw mut TASKS) }
}

#[allow(clippy::deref_addrof)]
fn ready() -> &'static mut Vec<u32> {
    // SAFETY: as above.
    unsafe { &mut *(&raw mut READY) }
}

#[allow(clippy::deref_addrof)]
fn draining() -> bool {
    // SAFETY: as above.
    unsafe { *(&raw const DRAINING) }
}

#[allow(clippy::deref_addrof)]
fn set_draining(on: bool) {
    // SAFETY: as above.
    unsafe { *(&raw mut DRAINING) = on }
}

/// Run a future to completion, off to the side of whatever called this.
///
/// The future is polled once before this returns, so the synchronous prefix of
/// an `async` block — the part before its first `await` — runs immediately and
/// in order, which is what island code expects when it disables a button and
/// then awaits an action. It is kept only if that first poll left it pending.
///
/// Spawning from inside a poll is allowed: the new task joins the ready queue
/// and the drain loop already running picks it up before it returns, rather
/// than being polled recursively underneath the task that spawned it.
pub fn spawn(fut: impl Future<Output = ()> + 'static) {
    let boxed: Boxed = Box::pin(fut);
    let table = tasks();
    let index = match table.iter().position(|slot| !slot.live && slot.fut.is_none()) {
        Some(index) => index,
        None => {
            table.push(Slot { fut: None, live: false });
            table.len() - 1
        }
    };
    table[index] = Slot { fut: Some(boxed), live: true };
    wake(index as u32);
}

/// Queue a task and, unless one is already running, drain the queue.
fn wake(index: u32) {
    let queue = ready();
    if !queue.contains(&index) {
        queue.push(index);
    }
    drain();
}

/// Poll every ready task until none is left.
///
/// The loop, rather than polling the woken task directly, is what makes the
/// two awkward cases safe. A resume that arrives while a task is being polled
/// — the shim calling `on_resume` from inside a host call this task made —
/// only appends to the queue, and a task that spawns another from inside its
/// own poll does the same; neither re-enters this function, so a future is
/// never polled from inside itself and the tables are never aliased.
fn drain() {
    if draining() {
        return;
    }
    set_draining(true);
    loop {
        let queue = ready();
        if queue.is_empty() {
            break;
        }
        let index = queue.remove(0);
        poll_task(index);
    }
    set_draining(false);
}

fn poll_task(index: u32) {
    let at = index as usize;
    let Some(mut fut) = tasks().get_mut(at).filter(|slot| slot.live).and_then(|slot| slot.fut.take()) else {
        // A stale queue entry for a task that has already finished. A spurious
        // poll is always allowed, and so a spurious nothing is too.
        return;
    };
    // SAFETY: the data pointer is an index, not an address, and the vtable
    // below only ever reads it back as one.
    let waker = unsafe { Waker::from_raw(raw_waker(index)) };
    let done = fut.as_mut().poll(&mut Context::from_waker(&waker)).is_ready();
    // The table is looked up again rather than held across the poll: the poll
    // may have spawned, and a push can move the vector.
    if let Some(slot) = tasks().get_mut(at) {
        if done {
            slot.live = false;
        } else {
            slot.fut = Some(fut);
        }
    }
    // A finished future is dropped here, after the slot is released, so the
    // tokens it still holds abort without the table being borrowed.
}

// --- the waker ---------------------------------------------------------------

/// The waker's data pointer is the task index plus one, so waking allocates
/// nothing and cloning a waker is a copy of two words. Plus one because a
/// null data pointer is a shape worth not having, not because anything here
/// dereferences it — nothing ever does.
static VTABLE: RawWakerVTable = RawWakerVTable::new(clone_waker, wake_raw, wake_raw, drop_waker);

fn raw_waker(index: u32) -> RawWaker {
    RawWaker::new((index as usize + 1) as *const (), &VTABLE)
}

fn clone_waker(data: *const ()) -> RawWaker {
    RawWaker::new(data, &VTABLE)
}

fn wake_raw(data: *const ()) {
    let index = data as usize;
    if index > 0 {
        wake(index as u32 - 1);
    }
}

fn drop_waker(_data: *const ()) {}

// --- tokens ------------------------------------------------------------------

/// How many of a token's bits are its index. The rest are a generation, which
/// is bumped every time a slot is reused so that a resume the shim sends after
/// an abort cannot be delivered to whichever future happened to take the slot
/// next. Rolling over a twelve-bit generation needs four thousand awaits to
/// pass between a token being aborted and the shim resuming it anyway; a
/// wider one is free to add later, since a token is opaque to the shim.
const INDEX_BITS: u32 = 20;
const INDEX_MASK: u32 = (1 << INDEX_BITS) - 1;

struct TokenSlot {
    live: bool,
    /// Set once the value the token was waiting for has arrived, so that
    /// dropping it does not tell the host to cancel work that is already done.
    settled: bool,
    generation: u32,
    waker: Option<Waker>,
    /// Resumes waiting to be taken. A queue rather than a single slot because
    /// a stream resumes repeatedly and two frames can land between two polls;
    /// dropping the first would silently lose an event.
    queue: Vec<Msg>,
}

static mut TOKENS: Vec<TokenSlot> = Vec::new();

#[allow(clippy::deref_addrof)]
fn tokens() -> &'static mut Vec<TokenSlot> {
    // SAFETY: single-threaded, and no borrow outlives the statement that
    // takes it — see the note in [`resume`] about waking with one held.
    unsafe { &mut *(&raw mut TOKENS) }
}

fn slot_of(token: u32) -> Option<&'static mut TokenSlot> {
    let index = (token & INDEX_MASK).checked_sub(1)? as usize;
    let slot = tokens().get_mut(index)?;
    (slot.live && slot.generation == token >> INDEX_BITS).then_some(slot)
}

/// A reservation for one outstanding host call.
///
/// Holding one is what entitles a future to be resumed: the number goes to the
/// host with the operation, and comes back with the value. Dropping it before
/// it settles sends [`op::ABORT`], which is how dropping a future cancels the
/// work behind it — the timer is cleared, the fetch is aborted, the event
/// source is closed. A token that has already produced its value drops
/// silently.
pub struct Token(u32);

impl Token {
    /// Reserve a slot. The number is never zero, which is the shim's "nothing".
    #[allow(clippy::new_without_default)] // A default that reserves a host slot would be a trap.
    pub fn new() -> Token {
        let table = tokens();
        let index = match table.iter().position(|slot| !slot.live) {
            Some(index) => index,
            None => {
                if table.len() >= INDEX_MASK as usize {
                    // A million live tokens is not a state to recover from,
                    // but it is also not a reason to trap: token zero belongs
                    // to no slot, so the future holding it stays pending.
                    return Token(0);
                }
                table.push(TokenSlot { live: false, settled: false, generation: 0, waker: None, queue: Vec::new() });
                table.len() - 1
            }
        };
        let generation = (table[index].generation + 1) & (u32::MAX >> INDEX_BITS);
        table[index] = TokenSlot { live: true, settled: false, generation, waker: None, queue: Vec::new() };
        Token((generation << INDEX_BITS) | (index as u32 + 1))
    }

    /// The number to hand the host operation.
    pub fn id(&self) -> u32 {
        self.0
    }

    /// The next resume for this token, registering the current task to be
    /// woken when there is not one yet.
    pub fn poll(&self, cx: &mut Context<'_>) -> Poll<Msg> {
        let Some(slot) = slot_of(self.0) else {
            return Poll::Pending;
        };
        if slot.queue.is_empty() {
            slot.waker = Some(cx.waker().clone());
            return Poll::Pending;
        }
        Poll::Ready(slot.queue.remove(0))
    }

    /// Record that the host call is finished, so dropping this does not abort
    /// it. A single-shot future settles the moment its value arrives; a stream
    /// settles when it ends or when it is closed explicitly.
    pub fn settle(&self) {
        if let Some(slot) = slot_of(self.0) {
            slot.settled = true;
        }
    }
}

impl Drop for Token {
    fn drop(&mut self) {
        let Some(slot) = slot_of(self.0) else {
            return;
        };
        let settled = slot.settled;
        slot.live = false;
        slot.settled = false;
        slot.waker = None;
        slot.queue = Vec::new();
        if !settled {
            abort(self.0);
        }
    }
}

#[allow(clippy::deref_addrof)]
fn abort(token: u32) {
    #[cfg(not(target_arch = "wasm32"))]
    {
        // SAFETY: single-threaded. Off the browser the host call does nothing
        // observable, so the count is how a test sees that it happened.
        unsafe { *(&raw mut ABORTS) += 1 }
    }
    call1(op::ABORT, token);
}

/// Hand a resume to whichever future is waiting for it.
///
/// A token that is unknown, already dropped, or from an earlier generation of
/// the same slot is ignored — the shim resolving something twice, or resolving
/// a fetch that was aborted while it was in flight, is a normal race and not
/// an error in the island.
pub fn resume(token: u32, payload: &[u8]) {
    let msg = decode(payload);
    let waker = {
        let Some(slot) = slot_of(token) else {
            return;
        };
        slot.queue.push(msg);
        slot.waker.clone()
    };
    // The borrow above ends before this: waking drains, draining polls, and a
    // poll can allocate another token and move the table out from under it.
    if let Some(waker) = waker {
        waker.wake();
    }
}

/// The export the shim calls when an awaited operation produced a value.
///
/// `ptr`/`len` address a blob the shim allocated through the module's `alloc`
/// export. The bytes are copied out here and the allocation is left to the
/// caller, which is the side that made it.
///
/// The pointer is trusted, exactly as the props blob in `hydrate_<Name>` is:
/// the only caller is the generated export, and the shim's side of the ABI is
/// what guarantees the range is readable.
pub fn on_resume(token: u32, ptr: u32, len: u32) {
    resume(token, payload(ptr, len));
}

#[cfg(target_arch = "wasm32")]
fn payload<'a>(ptr: u32, len: u32) -> &'a [u8] {
    if ptr == 0 || len == 0 {
        return &[];
    }
    // SAFETY: the shim's half of the ABI — the blob is live for the duration
    // of the call, and wasm is single-threaded so nothing mutates it here.
    unsafe { core::slice::from_raw_parts(ptr as usize as *const u8, len as usize) }
}

/// Off the browser a pointer does not fit in a `u32`, so there is nothing this
/// could soundly read. Host-side callers use [`resume`] instead, which takes
/// the bytes directly.
#[cfg(not(target_arch = "wasm32"))]
fn payload<'a>(_ptr: u32, _len: u32) -> &'a [u8] {
    &[]
}

// --- what the host-side tests look at ----------------------------------------

#[cfg(not(target_arch = "wasm32"))]
static mut ABORTS: u32 = 0;

/// How many tokens have been dropped unsettled since the module started.
///
/// The abort itself is invisible off the browser, so this is how a test sees
/// that dropping a future cancelled its host work. Compiled out of the wasm
/// build entirely.
#[cfg(not(target_arch = "wasm32"))]
#[allow(clippy::deref_addrof)]
pub fn aborts() -> u32 {
    // SAFETY: single-threaded.
    unsafe { *(&raw const ABORTS) }
}

/// How many spawned futures have not finished.
#[cfg(not(target_arch = "wasm32"))]
pub fn live_tasks() -> usize {
    tasks().iter().filter(|slot| slot.live).count()
}

/// How many tokens are outstanding.
#[cfg(not(target_arch = "wasm32"))]
pub fn live_tokens() -> usize {
    tokens().iter().filter(|slot| slot.live).count()
}
