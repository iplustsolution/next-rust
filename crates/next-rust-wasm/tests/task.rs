//! The executor, driven by hand.
//!
//! None of this needs a browser. Off `wasm32` every host call answers zero, so
//! nothing the module sends has an effect and nothing ever resumes by itself —
//! which is exactly the position to test an executor from, because the test
//! *is* the event loop. `task::resume` is the shim, called on whatever token
//! and with whatever bytes the case wants, including bytes no real shim would
//! send.
//!
//! The executor's tables are process-wide statics, sound on the browser
//! because wasm is single-threaded. The test harness is not, so every test
//! takes the same lock and the assertions are deltas rather than absolutes.
//! Nothing here may panic inside a poll: a panic would unwind out of the drain
//! loop and leave it marked as running.

use std::future::Future;
use std::pin::Pin;
use std::sync::{Mutex, MutexGuard};
use std::task::{Context, Poll, Waker};

use next_rust_wasm::net::ActionError;
use next_rust_wasm::task::{self, TAG_ERROR, TAG_VALUE, Token, spawn};

// --- the harness -------------------------------------------------------------

static LOCK: Mutex<()> = Mutex::new(());
static LOG: Mutex<Vec<&'static str>> = Mutex::new(Vec::new());
static IDS: Mutex<Vec<u32>> = Mutex::new(Vec::new());

/// Serialise the tests and start each one with an empty log. A poisoned lock
/// is taken anyway: one failed assertion should not cascade into every other
/// test reporting a poisoning instead of its own result.
fn harness() -> MutexGuard<'static, ()> {
    let guard = LOCK.lock().unwrap_or_else(|poison| poison.into_inner());
    LOG.lock().unwrap_or_else(|poison| poison.into_inner()).clear();
    IDS.lock().unwrap_or_else(|poison| poison.into_inner()).clear();
    guard
}

fn record(what: &'static str) {
    LOG.lock().unwrap_or_else(|poison| poison.into_inner()).push(what);
}

fn log() -> Vec<&'static str> {
    LOG.lock().unwrap_or_else(|poison| poison.into_inner()).clone()
}

/// Reserve a token and publish its number, so the test can resolve by hand
/// what the task is waiting on.
fn reserve() -> Token {
    let token = Token::new();
    IDS.lock().unwrap_or_else(|poison| poison.into_inner()).push(token.id());
    token
}

fn reserved() -> Vec<u32> {
    IDS.lock().unwrap_or_else(|poison| poison.into_inner()).clone()
}

/// What a one-shot host future does: wait for one message, settle, hand back
/// the status so the test can see which resume it was.
struct Wait(Token);

impl Future for Wait {
    type Output = u16;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<u16> {
        match self.0.poll(cx) {
            Poll::Ready(msg) => {
                self.0.settle();
                Poll::Ready(msg.status)
            }
            Poll::Pending => Poll::Pending,
        }
    }
}

/// A future that resumes itself from inside its own poll, which is what the
/// shim does when a host call completes synchronously.
struct ResumesItself {
    token: Token,
    started: bool,
}

impl Future for ResumesItself {
    type Output = u16;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<u16> {
        let it = self.get_mut();
        if let Poll::Ready(msg) = it.token.poll(cx) {
            it.token.settle();
            return Poll::Ready(msg.status);
        }
        if !it.started {
            it.started = true;
            task::resume(it.token.id(), &payload(TAG_VALUE, 7, b""));
        }
        Poll::Pending
    }
}

fn payload(tag: u8, status: u16, body: &[u8]) -> Vec<u8> {
    let mut out = vec![tag];
    out.extend_from_slice(&status.to_le_bytes());
    out.extend_from_slice(body);
    out
}

/// Resolve a token the way the shim would, with a value and a status.
fn resolve(token: u32, status: u16) {
    task::resume(token, &payload(TAG_VALUE, status, b""));
}

// --- the executor ------------------------------------------------------------

#[test]
fn a_future_that_never_waits_finishes_during_the_spawn() {
    let _guard = harness();
    let before = task::live_tasks();

    spawn(async { record("ran") });

    assert_eq!(log(), ["ran"], "the synchronous part of a task runs before spawn returns");
    assert_eq!(task::live_tasks(), before, "a finished task must not keep its slot");
}

#[test]
fn a_task_that_needs_two_resumes_gets_them_in_order() {
    let _guard = harness();

    spawn(async {
        record("start");
        let first = Wait(reserve()).await;
        record(if first == 11 { "first" } else { "wrong first" });
        let second = Wait(reserve()).await;
        record(if second == 22 { "second" } else { "wrong second" });
    });

    assert_eq!(log(), ["start"], "it parks on the first await");
    assert_eq!(reserved().len(), 1, "the second token is not reserved until the first resolves");

    resolve(reserved()[0], 11);
    assert_eq!(log(), ["start", "first"]);
    assert_eq!(reserved().len(), 2);

    resolve(reserved()[1], 22);
    assert_eq!(log(), ["start", "first", "second"]);
}

#[test]
fn two_tasks_interleave_in_the_order_they_are_resumed() {
    let _guard = harness();

    spawn(async {
        Wait(reserve()).await;
        record("a");
    });
    spawn(async {
        Wait(reserve()).await;
        record("b");
    });

    let tokens = reserved();
    assert_eq!(tokens.len(), 2, "both tasks reached their await");
    assert!(log().is_empty());

    resolve(tokens[1], 0);
    resolve(tokens[0], 0);

    assert_eq!(log(), ["b", "a"], "a task waits for its own token and nothing else");
}

#[test]
fn a_task_spawned_from_inside_a_poll_is_queued_rather_than_nested() {
    let _guard = harness();

    spawn(async {
        record("outer start");
        spawn(async { record("inner") });
        record("outer end");
        Wait(reserve()).await;
        record("outer resumed");
    });

    assert_eq!(
        log(),
        ["outer start", "outer end", "inner"],
        "the inner task runs after the outer one yields, not underneath it"
    );

    resolve(reserved()[0], 0);
    assert_eq!(log().last(), Some(&"outer resumed"));
}

#[test]
fn a_resume_from_inside_the_poll_that_caused_it_still_lands() {
    let _guard = harness();

    spawn(async {
        let status = ResumesItself { token: reserve(), started: false }.await;
        record(if status == 7 { "resumed" } else { "wrong status" });
    });

    assert_eq!(log(), ["resumed"], "a synchronous resume is queued and polled by the drain loop");
}

#[test]
fn a_resume_for_a_token_nobody_holds_is_ignored() {
    let _guard = harness();

    // Nothing has ever been reserved under these.
    task::resume(0, &payload(TAG_VALUE, 0, b"x"));
    task::resume(1, &payload(TAG_VALUE, 0, b"x"));
    task::resume(u32::MAX, &payload(TAG_VALUE, 0, b"x"));
    task::resume(9999, &[]);

    // And a token that has already been resolved and dropped.
    spawn(async {
        Wait(reserve()).await;
        record("done");
    });
    let token = reserved()[0];
    resolve(token, 0);
    assert_eq!(log(), ["done"]);

    resolve(token, 0);
    resolve(token, 0);
    assert_eq!(log(), ["done"], "a second resume for a finished token changes nothing");

    // The raw export off the browser cannot read a pointer that does not fit
    // in a `u32`, so it must not try.
    task::on_resume(token, 0, 0);
    task::on_resume(token, 0xDEAD_BEEF, 64);
}

#[test]
fn a_reused_token_slot_does_not_answer_for_the_token_before_it() {
    let _guard = harness();

    let first = {
        let token = Token::new();
        token.id()
    };
    let second = Token::new();
    assert_ne!(first, second.id(), "a reused slot gets a new generation");

    task::resume(first, &payload(TAG_VALUE, 1, b""));
    let mut cx = Context::from_waker(Waker::noop());
    assert!(second.poll(&mut cx).is_pending(), "the stale resume went nowhere");
}

// --- cancellation ------------------------------------------------------------

#[test]
fn dropping_a_token_before_it_settles_aborts_the_host_call() {
    let _guard = harness();

    let before = task::aborts();
    drop(Token::new());
    assert_eq!(task::aborts(), before + 1, "an unsettled token cancels its host work");

    let settled = Token::new();
    settled.settle();
    drop(settled);
    assert_eq!(task::aborts(), before + 1, "a token that produced its value cancels nothing");
}

#[test]
fn dropping_a_future_that_is_in_flight_aborts_it() {
    let _guard = harness();

    let before = task::aborts();
    let live = task::live_tokens();

    let mut fut = Box::pin(next_rust_wasm::net::timeout(5_000));
    assert!(fut.as_mut().poll(&mut Context::from_waker(Waker::noop())).is_pending());
    assert_eq!(task::live_tokens(), live + 1, "the timer is outstanding");

    drop(fut);
    assert_eq!(task::aborts(), before + 1, "dropping the future cleared the timer");
    assert_eq!(task::live_tokens(), live, "and released the slot");
}

#[test]
fn a_future_that_ran_to_completion_aborts_nothing() {
    let _guard = harness();

    let before = task::aborts();
    spawn(async {
        Wait(reserve()).await;
        record("done");
    });
    resolve(reserved()[0], 0);

    assert_eq!(log(), ["done"]);
    assert_eq!(task::aborts(), before, "a settled token is not cancelled on the way out");
}

// --- the flat error payload --------------------------------------------------

/// A complete action error: status, message, the reserved entries, two fields.
fn action_error_payload() -> Vec<u8> {
    payload(
        TAG_ERROR,
        422,
        b"Could not save\n_code\tinvalid_input\n_form\tCheck the highlighted fields\nemail\tEnter an email address\npassword\tToo short\n",
    )
}

#[test]
fn an_action_error_is_read_by_splitting_bytes() {
    let error = ActionError::from_payload(&action_error_payload());

    assert_eq!(error.status(), 422);
    assert_eq!(error.message(), "Could not save");
    assert_eq!(error.code(), "invalid_input");
    assert_eq!(error.form_error(), "Check the highlighted fields");
    assert_eq!(error.field("email"), Some("Enter an email address"));
    assert_eq!(error.field("password"), Some("Too short"));
    assert_eq!(error.field("name"), None);
    assert_eq!(error.field("_code"), None, "the envelope is not a form field");
    assert_eq!(error.fields().collect::<Vec<_>>(), [("email", "Enter an email address"), ("password", "Too short")]);
}

#[test]
fn an_error_with_no_fields_still_has_a_message() {
    let error = ActionError::from_payload(&payload(TAG_ERROR, 500, b"Something went wrong"));

    assert_eq!(error.message(), "Something went wrong");
    assert_eq!(error.code(), "");
    assert_eq!(
        error.form_error(),
        "Something went wrong",
        "with nothing marked form-level, the message is what goes above the form"
    );
    assert_eq!(error.fields().count(), 0);
}

/// Anything the shim can send, including anything it should not, has to read
/// as *something*. Every prefix of a good payload is tried, which covers the
/// truncated header, the truncated message and a field line cut in half.
#[test]
fn every_truncation_of_an_error_payload_reads_without_panicking() {
    let whole = action_error_payload();
    for end in 0..=whole.len() {
        let error = ActionError::from_payload(&whole[..end]);
        let _ = error.status();
        let _ = error.message();
        let _ = error.code();
        let _ = error.form_error();
        let _ = error.field("email");
        let _ = error.fields().count();
    }
}

#[test]
fn a_malformed_error_payload_reads_as_empty_rather_than_failing() {
    for bytes in [
        &b""[..],
        &[TAG_ERROR][..],
        &[TAG_ERROR, 0][..],
        &[0xFF, 0xFF, 0xFF, 0xFF][..],
        // Not UTF-8, in the message and in a field name.
        &[TAG_ERROR, 1, 0, 0xC3, 0x28, b'\n', 0xFF, b'\t', b'x'][..],
        // Tabs and newlines in every wrong place.
        &[TAG_ERROR, 0, 0, b'\n', b'\n', b'\t', b'\t', b'\n'][..],
    ] {
        let error = ActionError::from_payload(bytes);
        let _ = error.message();
        let _ = error.code();
        let _ = error.form_error();
        let _ = error.field("email");
        let _ = error.fields().count();
    }
}

#[test]
fn a_field_line_without_a_tab_is_skipped_rather_than_guessed_at() {
    let error = ActionError::from_payload(&payload(TAG_ERROR, 400, b"No\nnonsense\nemail\tRequired\n"));

    assert_eq!(error.fields().collect::<Vec<_>>(), [("email", "Required")]);
    assert_eq!(error.field("nonsense"), None);
}
