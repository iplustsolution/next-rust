//! Async without a runtime: the executor, the timers and the callback table.
//!
//! The browser is single threaded and the module is too, so this is as small
//! as an executor gets. [`spawn`] parks a future in a slab and polls it; a
//! future that is waiting registers a **token** and returns `Pending`; the
//! shim calls [`resume`] with that token when the promise it stands for
//! settles, and the task is polled again. There are no threads, no atomics
//! and no allocation beyond the slab, which is why this costs about a
//! kilobyte of wasm rather than the tens of kilobytes a general executor does.
//!
//! Callbacks work the same way: [`on`] registers a closure and hands the shim
//! a number, and [`on_event`] is what the shim calls back into.

use alloc::boxed::Box;
use alloc::collections::BTreeMap;
use alloc::string::String;
use alloc::vec::Vec;
use core::cell::RefCell;
use core::future::Future;
use core::pin::Pin;
use core::task::{Context, Poll, RawWaker, RawWakerVTable, Waker};

use crate::dom::Node;
use crate::host::{self, op};

/// What a settled promise handed back.
#[derive(Default)]
pub struct Settled {
    /// The handle the promise resolved with (a `Response`, a module, …).
    pub handle: u32,
    /// A second integer: `done` for a stream read, the error flag otherwise.
    pub flag: u32,
    /// The string the promise resolved with, when it resolved with one.
    pub text: Option<String>,
}

impl Settled {
    /// Whether the promise rejected.
    pub fn failed(&self) -> bool {
        self.flag & FAILED != 0
    }

    /// The handle, owned.
    pub fn node(&self) -> Option<Node> {
        Node::new(self.handle)
    }
}

/// `Settled::flag`: the promise rejected rather than resolved.
pub const FAILED: u32 = 1;
/// `Settled::flag`: a stream reader reached the end.
pub const DONE: u32 = 2;

type Task = Pin<Box<dyn Future<Output = ()>>>;
/// One registered event handler; the shim calls it back by number.
type Callback = Box<dyn Fn(Option<Node>)>;

#[derive(Default)]
struct State {
    /// Futures that have been spawned and have not finished.
    tasks: BTreeMap<u32, Task>,
    /// Which task a token belongs to, and what it settled with.
    waiting: BTreeMap<u32, (u32, Option<Settled>)>,
    /// Registered event callbacks.
    callbacks: BTreeMap<u32, Callback>,
    /// Tasks that are ready to be polled again.
    ready: Vec<u32>,
    next: u32,
    /// Depth of `run`, so a callback that spawns does not re-enter the loop.
    running: bool,
}

crate::local! {
    static STATE: RefCell<State> = RefCell::new(State::default());
}

fn next_id() -> u32 {
    STATE.with(|s| {
        let mut s = s.borrow_mut();
        s.next = s.next.wrapping_add(1).max(1);
        s.next
    })
}

// A waker that does nothing: a task is woken by `resume`, never by itself.
static VTABLE: RawWakerVTable =
    RawWakerVTable::new(|_| RawWaker::new(core::ptr::null(), &VTABLE), |_| {}, |_| {}, |_| {});

fn waker() -> Waker {
    // SAFETY: the vtable's four functions are all no-ops on a null pointer.
    unsafe { Waker::from_raw(RawWaker::new(core::ptr::null(), &VTABLE)) }
}

/// Start a future. It runs until its first `await` before this returns, which
/// is what the JavaScript it replaces does too.
pub fn spawn(future: impl Future<Output = ()> + 'static) {
    let id = next_id();
    STATE.with(|s| {
        s.borrow_mut().tasks.insert(id, Box::pin(future));
        s.borrow_mut().ready.push(id);
    });
    run();
}

/// Poll every task that is ready, until none is.
fn run() {
    if STATE.with(|s| core::mem::replace(&mut s.borrow_mut().running, true)) {
        return;
    }
    while let Some(id) = STATE.with(|s| s.borrow_mut().ready.pop()) {
        let Some(mut task) = STATE.with(|s| s.borrow_mut().tasks.remove(&id)) else { continue };
        let waker = waker();
        let mut cx = Context::from_waker(&waker);
        if task.as_mut().poll(&mut cx).is_pending() {
            STATE.with(|s| s.borrow_mut().tasks.insert(id, task));
        }
    }
    STATE.with(|s| s.borrow_mut().running = false);
}

/// A token the shim settles. Awaiting it parks the task; the value is what
/// [`resume`] was called with.
pub struct Pending {
    token: u32,
    registered: bool,
}

/// Take a token to hand to a host call that settles asynchronously.
pub fn pending() -> Pending {
    Pending { token: next_id(), registered: false }
}

impl Pending {
    /// The number to pass to the host call.
    pub fn token(&self) -> u32 {
        self.token
    }
}

impl Future for Pending {
    type Output = Settled;

    fn poll(mut self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<Settled> {
        let token = self.token;
        let settled = STATE.with(|s| {
            let mut s = s.borrow_mut();
            match s.waiting.get_mut(&token) {
                Some((_, value @ Some(_))) => value.take(),
                _ => None,
            }
        });
        if let Some(settled) = settled {
            STATE.with(|s| s.borrow_mut().waiting.remove(&token));
            return Poll::Ready(settled);
        }
        if !self.registered {
            self.registered = true;
            // The task being polled is the one the token belongs to; `run`
            // removed it from the slab, so remember the token against the id
            // the executor will put back.
            STATE.with(|s| s.borrow_mut().waiting.insert(token, (0, None)));
        }
        Poll::Pending
    }
}

impl Drop for Pending {
    fn drop(&mut self) {
        STATE.with(|s| {
            s.borrow_mut().waiting.remove(&self.token);
        });
    }
}

/// Register an event callback and return the number the shim calls back with.
pub fn on(handler: impl Fn(Option<Node>) + 'static) -> u32 {
    let id = next_id();
    STATE.with(|s| s.borrow_mut().callbacks.insert(id, Box::new(handler)));
    id
}

/// Forget a callback the runtime will not need again.
pub fn forget(id: u32) {
    STATE.with(|s| {
        s.borrow_mut().callbacks.remove(&id);
    });
}

/// `setTimeout`, as a handle that cancels when it is dropped.
///
/// It owns its callback as well as the browser's timer id. A timer that fires
/// forgets its callback, and one that is cancelled or dropped forgets it too:
/// without that, a repeating timer like the loading bar's 200 ms trickle would
/// add an entry to the callback table five times a second for the life of the
/// page.
pub struct Timer {
    id: u32,
    callback: u32,
}

impl Timer {
    /// Run `handler` in `ms` milliseconds.
    pub fn after(ms: f64, handler: impl Fn() + 'static) -> Self {
        let callback = next_id();
        let once = callback;
        STATE.with(|s| {
            s.borrow_mut().callbacks.insert(
                callback,
                Box::new(move |_| {
                    handler();
                    forget(once);
                }),
            )
        });
        Self { id: host::call(op::SET_TIMEOUT, ms.max(0.0) as u32, callback, 0, 0, 0), callback }
    }

    /// Stop it, as `clearTimeout` does. Dropping the handle does this too.
    pub fn cancel(&self) {
        if self.id != 0 {
            host::call(op::CLEAR_TIMEOUT, self.id, 0, 0, 0, 0);
        }
        forget(self.callback);
    }
}

impl Drop for Timer {
    fn drop(&mut self) {
        self.cancel();
    }
}

/// Wait `ms` milliseconds: `new Promise(r => setTimeout(r, ms))`.
pub async fn pause(ms: f64) {
    let slot = pending();
    let token = slot.token();
    let cb = on(move |_| resume(token, 0, 0, host::NULL_STR));
    host::call(op::SET_TIMEOUT, ms.max(0.0) as u32, cb, 0, 0, 0);
    slot.await;
    forget(cb);
}

// ---- what the shim calls -------------------------------------------------------

/// One event: the shim passes the callback's number and a handle for the
/// event object, which is `0` for a timer.
///
/// # Safety
/// Exported for the shim only.
#[unsafe(no_mangle)]
pub extern "C" fn on_event(id: u32, event: u32) {
    let handler = STATE.with(|s| s.borrow().callbacks.contains_key(&id));
    if handler {
        // The closure is taken out so a handler that removes itself, or that
        // spawns work which does, cannot invalidate the borrow.
        let taken = STATE.with(|s| s.borrow_mut().callbacks.remove(&id));
        if let Some(handler) = taken {
            handler(Node::new(event));
            STATE.with(|s| {
                let mut s = s.borrow_mut();
                // A handler that removed itself stays removed.
                s.callbacks.entry(id).or_insert(handler);
            });
        }
    }
    run();
}

/// A promise the module is waiting on settled.
///
/// # Safety
/// Exported for the shim only. `text` is [`host::NULL_STR`] when the promise
/// settled without a string.
#[unsafe(no_mangle)]
pub extern "C" fn resume(token: u32, handle: u32, flag: u32, text_len: u32) {
    let text = if text_len == host::NULL_STR {
        None
    } else {
        let mut bytes = alloc::vec![0u8; text_len as usize];
        host::call(op::RESULT_TAKE, bytes.as_mut_ptr() as u32, text_len, 0, 0, 0);
        Some(String::from_utf8(bytes).unwrap_or_default())
    };
    let known = STATE.with(|s| {
        let mut s = s.borrow_mut();
        match s.waiting.get_mut(&token) {
            Some(entry) => {
                entry.1 = Some(Settled { handle, flag, text });
                true
            }
            None => false,
        }
    });
    if known {
        // Every task is polled: the executor does not track which token
        // belongs to which task, and there are never more than a handful.
        STATE.with(|s| {
            let mut s = s.borrow_mut();
            let ids: Vec<u32> = s.tasks.keys().copied().collect();
            s.ready.extend(ids);
        });
    }
    run();
}
