//! The loading bar.
//!
//! `[loader]` in `next-rust.toml` draws a bar along the top of the page while
//! the next page is on its way: `html[data-nr-loading]::before`, slid in from
//! the left `--nr-load` of the way by a transform. The stylesheet announces
//! itself with `--nr-loader` and sets the timing; the bar appears only once a
//! load has taken `--nr-load-delay` ms (a prefetched page never flashes it),
//! moves quickly at first and ever slower while waiting, then fills and fades
//! (`data-nr-loading="done"`) in `--nr-load-speed` ms each. [`start`] returns
//! the [`Done`] that ends that load; the bar stays while any load is open.
//!
//! Nothing here decides what a load *is* — [`crate::nav`] and [`crate::forms`]
//! open and close them. This module only draws.

use alloc::rc::Rc;
use alloc::string::String;
use core::cell::{Cell, RefCell};

use crate::dom::{self, Node};
use crate::host::{self, op};
use crate::task::{self, Timer};

/// The stylesheet's linear transition lasts a little longer: the bar never rests.
const STEP: f64 = 200.0;
const KEY: &str = "nr-load";

/// How long a bar left by the previous document may have been sitting there.
const CARRY: f64 = 35_000.0;
/// The bar a carried-over load runs on until the page is loaded.
const CARRY_LIMIT: f64 = 30_000.0;
/// A carried-over bar starts where the last one stood, but never past this.
const CARRY_MOST: f64 = 0.95;

// ---- state -------------------------------------------------------------------

#[derive(Default)]
struct Loader {
    /// How many loads are open. The bar stays while any of them is.
    open: u32,
    shown: bool,
    progress: f64,
    began: f64,
    /// The wait before the bar is shown at all. Its presence is the source's
    /// truthy `delay`: `start` looks at it to decide whether one is pending.
    delay: Option<Timer>,
    creep: Option<Timer>,
    hide: Option<Timer>,
}

crate::local! {
    static LOADER: RefCell<Loader> = RefCell::new(Loader::default());
}

/// The state, for as short a moment as possible: a host call that runs while
/// it is borrowed cannot re-enter the module, but a timer's callback can.
fn with<R>(f: impl FnOnce(&mut Loader) -> R) -> R {
    LOADER.with(|state| f(&mut state.borrow_mut()))
}

// ---- drawing ------------------------------------------------------------------

fn set(progress: f64) {
    with(|state| state.progress = progress);
    let mut value = number(progress * 100.0, 6);
    value.push('%');
    dom::style_set(&dom::root(), "--nr-load", &value);
}

/// Styles must be computed for a transition to have somewhere to start from.
fn flush() {
    let opacity = dom::computed(&dom::root(), "::before", "opacity");
    // Nothing wants the value; making the call is the whole point of it. This
    // says so, and keeps the read from reading as something left behind.
    core::hint::black_box(&opacity);
}

/// Where the bar stands `t` ms in: a third within half a second, 95% never.
fn curve(t: f64) -> f64 {
    0.95 - 0.435 * (exp(-t / 600.0) + exp(-t / 6000.0))
}

/// Aim at where it should be by the next step; the transition takes it there.
fn crawl() {
    let (progress, began) = with(|state| (state.progress, state.began));
    let aim = curve(dom::performance_now() - began + STEP);
    // `Math.max`, which is NaN when either side is: the source's is too.
    set(if progress > aim { progress } else { aim });
    let creep = Timer::after(STEP, crawl);
    with(|state| state.creep = Some(creep));
}

/// `from`: where a bar taken over from the previous document stood.
fn show(from: f64) {
    let root = dom::root();
    with(|state| {
        state.hide = None; // clearTimeout(hide)
        state.shown = true;
    });
    // A bar still fading from the last load is dropped, not dragged back.
    dom::remove_attr(&root, "data-nr-loading");
    flush();
    set(from);
    dom::set_attr(&root, "data-nr-loading", "");
    flush();
    let began = dom::performance_now();
    with(|state| state.began = began);
    crawl();
}

fn finish() {
    let was_shown = with(|state| {
        state.delay = None;
        state.creep = None;
        core::mem::replace(&mut state.shown, false)
    });
    if !was_shown {
        return;
    }
    let root = dom::root();
    dom::set_attr(&root, "data-nr-loading", "done");
    set(1.0);
    // Long enough for the fill and the fade, which run one after the other.
    let after = css_number(&root, "--nr-load-speed") * 2.0 + 50.0;
    let hide = Timer::after(after, || {
        let root = dom::root();
        dom::remove_attr(&root, "data-nr-loading");
        dom::style_remove(&root, "--nr-load");
    });
    with(|state| state.hide = Some(hide));
}

// ---- one load ------------------------------------------------------------------

/// The end of one load, which is the source's `done`: calling it twice does
/// nothing the second time, and never calling it leaves the load open, so
/// this deliberately has no `Drop`.
pub struct Done(Rc<Cell<bool>>);

impl Done {
    /// A load that was over before it began: what `start` hands back when no
    /// stylesheet asked for a bar, the source's `() => {}`.
    fn inert() -> Self {
        Self(Rc::new(Cell::new(true)))
    }

    /// End this load. The bar finishes once the last open load has ended.
    pub fn end(self) {
        close(&self.0);
    }
}

/// `done` itself: the flag is shared with the `limit` timer, so whichever
/// fires first is the one that counts.
fn close(ended: &Rc<Cell<bool>>) {
    if ended.replace(true) {
        return;
    }
    let open = with(|state| {
        state.open = state.open.saturating_sub(1); // `--open <= 0` and `open = 0`
        state.open
    });
    if open == 0 {
        finish();
    }
}

/// One load. `limit` ends it on its own after that many ms; `from` is where a
/// bar taken over from the previous document stood.
pub fn start(limit: Option<f64>, from: Option<f64>) -> Done {
    let root = dom::root();
    if dom::computed(&root, "", "--nr-loader").is_empty() {
        return Done::inert();
    }
    with(|state| state.open += 1);
    match from {
        // `if (from)`: a bar carried over from the last document is shown at once.
        Some(from) if truthy(from) => show(from),
        _ => {
            // `!shown && !delay`: a bar already up, or already on its way up,
            // is not asked for a second time.
            if with(|state| !state.shown && state.delay.is_none()) {
                let delay = Timer::after(css_number(&root, "--nr-load-delay"), || {
                    // Whatever opened the load may have finished by now.
                    let open = with(|state| {
                        state.delay = None;
                        state.open
                    });
                    if open > 0 {
                        show(0.0);
                    }
                });
                with(|state| state.delay = Some(delay));
            }
        }
    }
    let ended = Rc::new(Cell::new(false));
    if let Some(limit) = limit.filter(|limit| truthy(*limit)) {
        let ended = ended.clone();
        loose(Timer::after(limit, move || close(&ended)));
    }
    Done(ended)
}

/// Every load is over, whatever is still holding one: a page coming back out
/// of the back/forward cache left its bar behind in another life.
pub fn reset() {
    with(|state| state.open = 0);
    finish();
}

// ---- the page's own comings and goings ---------------------------------------------

/// The listeners the bar needs, and the bar the last document left behind.
pub fn install() {
    let window = window();
    // A full page load: the bar the last document left with carries on in this
    // one from where it stood, and completes once the page has loaded.
    let left = dom::storage_get(KEY).unwrap_or_default();
    dom::storage_remove(KEY);
    if let Some(from) = resume_from(&left, dom::now()) {
        let ended = start(Some(CARRY_LIMIT), Some(from)).0;
        if dom::prop_str(&dom::document(), "readyState").as_deref() == Some("complete") {
            // `load` has been and gone; end it the next time round the loop,
            // so a caller of `install` still sees the bar go up first.
            loose(Timer::after(0.0, move || close(&ended)));
        } else if let Some(window) = &window {
            listen(window, "load", move |_| close(&ended));
        }
    }
    let Some(window) = window else { return };
    listen(&window, "pagehide", |_| {
        let (shown, progress) = with(|state| (state.shown, state.progress));
        if shown {
            let mut value = number(progress, 6);
            value.push(' ');
            value.push_str(&number(dom::now(), 0));
            dom::storage_set(KEY, &value);
        }
    });
    // A page restored from the back/forward cache still shows the bar it left with.
    listen(&window, "pageshow", |event| {
        if event.is_some_and(|event| dom::prop_bool(&event, "persisted")) {
            reset();
        }
    });
}

/// The source's `left[1] > Date.now() - 35000 && left[0] > 0`, as one
/// decision: how far the carried-over bar starts, or nothing at all.
///
/// Both halves are read the way `>` reads a string, which is `Number(s)` and
/// not `parseFloat`: trailing rubbish is NaN, and every comparison with NaN
/// is false. A missing second half is `undefined`, which is NaN too.
fn resume_from(stored: &str, now: f64) -> Option<f64> {
    let mut left = stored.split(' ');
    let progress = to_number(left.next().unwrap_or(""))?;
    let at = to_number(left.next()?)?;
    if at > now - CARRY && progress > 0.0 {
        // `Math.min(+left[0], 0.95)`: a bar that was nearly full still has to
        // have somewhere left to go.
        Some(if progress > CARRY_MOST { CARRY_MOST } else { progress })
    } else {
        None
    }
}

// ---- the bits of the browser this file needs ------------------------------------------

/// The ABI has no opcode for `window`, and does not need one: `defaultView`
/// is the same object, and the three listeners the bar adds all go on it.
fn window() -> Option<Node> {
    dom::prop_node(&dom::document(), "defaultView")
}

/// `target.addEventListener(event, handler)`, for a listener that is never
/// removed: the handle the shim hands back is given straight back to it, and
/// the listener stays on the page for as long as the page does.
fn listen(target: &Node, event: &str, handler: impl Fn(Option<Node>) + 'static) {
    // The callback is registered first: nothing may allocate between taking
    // the string's address and handing it over.
    let callback = task::on(handler);
    let (p, l) = host::s(event);
    host::release(host::call(op::ADD_LISTENER, target.raw(), p, l, callback, 0));
}

/// A timer that is never cleared, as `setTimeout(done, limit)` is not: holding
/// the handle would cancel it when the handle went out of scope.
fn loose(timer: Timer) {
    core::mem::forget(timer);
}

/// `parseFloat(css(name)) || 0`, which is how the source reads every number
/// the stylesheet sets.
fn css_number(root: &Node, name: &str) -> f64 {
    let value = parse_float_prefix(&dom::computed(root, "", name)).unwrap_or(0.0);
    if truthy(value) { value } else { 0.0 }
}

/// `if (n)` for a number: zero, minus zero and NaN are the falsy ones.
fn truthy(value: f64) -> bool {
    value != 0.0 && !value.is_nan()
}

// ---- numbers ------------------------------------------------------------------------

const LN2_HI: f64 = 6.931_471_803_691_238e-1;
const LN2_LO: f64 = 1.908_214_929_270_588e-10;

/// `Math.exp(x)` for the `x <= 0` this file asks of it.
///
/// `core` has no `exp` and the bar is not worth a dependency. e^-x is
/// 2^-n · e^-r once n takes out the whole multiples of ln 2; the remainder is
/// never more than half a ln 2, where a dozen terms of the series are already
/// past what a double can tell apart, and 2^-n is exact.
fn exp(x: f64) -> f64 {
    if x.is_nan() {
        return x;
    }
    // e^x for x <= 0 is e^-|x|; positive arguments never reach here.
    let x = -x;
    if x <= 0.0 {
        return 1.0;
    }
    if x > 708.4 {
        // Below the smallest double that is not a subnormal, where `2^-n`
        // stops being exact. `Math.exp` answers 1e-308 here; the curve
        // reached 0.95, as far as a double can tell, minutes before this.
        return 0.0;
    }
    // `log2 e` is `1 / ln 2`: how many whole halvings are in the argument.
    let n = (x * core::f64::consts::LOG2_E + 0.5) as i32;
    // Two halves of ln 2: `n * LN2_HI` is exact, so the reduction costs nothing.
    let r = (x - f64::from(n) * LN2_HI) - f64::from(n) * LN2_LO;
    let u = -r;
    let mut term = 1.0;
    let mut sum = 1.0;
    let mut k = 1;
    while k <= 13 {
        term *= u / f64::from(k);
        sum += term;
        k += 1;
    }
    // 2^-n by squaring: every factor is a power of two, so nothing is rounded.
    let mut scale = 1.0;
    let mut half = 0.5;
    let mut rest = n;
    while rest > 0 {
        if rest & 1 == 1 {
            scale *= half;
        }
        half *= half;
        rest >>= 1;
    }
    sum * scale
}

/// `parseFloat(s)`: leading whitespace, an optional sign, digits with at most
/// one dot and an optional exponent, and it stops at the first character that
/// cannot go on. `None` is the `NaN` that nothing was parsed.
fn parse_float_prefix(text: &str) -> Option<f64> {
    number_prefix(text).map(|(value, _)| value)
}

/// `Number(s)`, which is `parseFloat` that has to reach the end of the string,
/// and `0` for one with nothing but whitespace in it. `None` is `NaN`.
///
/// Hexadecimal and binary literals are the one thing this leaves out: what it
/// reads is a progress and a `Date.now()`, and neither is ever written that way.
fn to_number(text: &str) -> Option<f64> {
    let text = text.trim_matches(char::is_whitespace);
    if text.is_empty() {
        return Some(0.0); // `+"" === 0`
    }
    let (value, used) = number_prefix(text)?;
    (used == text.len()).then_some(value)
}

/// The number at the front of `text`, and how many bytes of it that took.
///
/// `num::parse_prefix` is the crate's one `parseFloat`: a second copy here
/// would be a second answer to the same question, and about a kilobyte of
/// wasm for it.
fn number_prefix(text: &str) -> Option<(f64, usize)> {
    crate::num::parse_prefix(text)
}

/// `String(n)` for the two numbers this file writes out: a percentage for
/// the stylesheet and a progress and timestamp for the next document.
fn number(value: f64, places: u32) -> String {
    crate::num::to_string(value, places)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// How far `exp` is from the real thing, relative, over the whole range
    /// the curve asks about and well past it.
    fn exp_error(x: f64) -> f64 {
        let want = libm_exp(x);
        if want == 0.0 { (exp(x) - want).abs() } else { ((exp(x) - want) / want).abs() }
    }

    /// `f64::exp` is `std`, which the wasm build does not have; the tests do.
    fn libm_exp(x: f64) -> f64 {
        x.exp()
    }

    #[test]
    fn exp_is_accurate() {
        // The values the curve actually asks for, against `Math.exp`'s own.
        assert!((exp(0.0) - 1.0).abs() < 1e-15);
        assert!((exp(-1.0) - 0.367_879_441_171_442_33).abs() < 1e-15);
        assert!((exp(-0.5) - 0.606_530_659_712_633_4).abs() < 1e-15);
        assert!((exp(-1.0 / 3.0) - 0.716_531_310_573_789_3).abs() < 1e-15);
        assert!((exp(-5.0) - 0.006_737_946_999_085_467).abs() < 1e-16);
        assert!((exp(-50.0) - 1.928_749_847_963_918e-22).abs() < 1e-36);

        // And a sweep: nothing anywhere near the range is worse than 1e-14.
        let mut worst: f64 = 0.0;
        let mut step = 0;
        while step <= 60_000 {
            let t = f64::from(step);
            worst = worst.max(exp_error(-t / 600.0)).max(exp_error(-t / 6000.0));
            step += 1;
        }
        assert!(worst < 1e-14, "worst relative error {worst:e}");

        // Out to where a double stops being able to hold the answer, which is
        // four ulp away the whole way: a good deal better than the 1e-9 the
        // bar would have settled for.
        let mut x = 0.0;
        while x < 708.4 {
            assert!(exp_error(-x) < 1e-14, "at -{x}: {}", exp_error(-x));
            x += 0.125;
        }
        assert_eq!(exp(-709.0), 0.0);
        assert_eq!(exp(-800.0), 0.0);
        assert_eq!(exp(f64::NEG_INFINITY), 0.0);
        assert!(exp(f64::NAN).is_nan());
    }

    #[test]
    fn curve_creeps_and_never_arrives() {
        // 0.95 - 0.435 * 2 at the start: the bar is already a twelfth in.
        assert!((curve(0.0) - 0.08).abs() < 1e-12);
        assert!(curve(200.0) > curve(0.0));
        assert!((curve(600.0) - (0.95 - 0.435 * (libm_exp(-1.0) + libm_exp(-0.1)))).abs() < 1e-12);
        // A third of the way within half a second, as the source's comment says.
        assert!(curve(500.0) > 0.33 && curve(500.0) < 0.4);
        assert!(curve(6000.0) > 0.78 && curve(6000.0) < 0.81);
        assert!(curve(30_000.0) > 0.94);

        // Always climbing, and always with somewhere left to climb to, over
        // twice as long as the longest load the runtime will hold open.
        let mut last = f64::NEG_INFINITY;
        let mut step = 0;
        while step <= 120_000 {
            let value = curve(f64::from(step));
            assert!(value > last, "not monotone at {step}");
            assert!(value < 0.95, "reached 0.95 at {step}");
            last = value;
            step += 25;
        }
        // Past about three and a half minutes the gap left is smaller than a
        // double can hold beside 0.95 and the curve stops moving. The
        // JavaScript stops in exactly the same place, for the same reason,
        // and no load lives that long: `start` gives up after 30 seconds.
        assert!(curve(400_000.0) >= curve(300_000.0));
    }

    #[test]
    fn parses_a_number_off_the_front() {
        assert_eq!(parse_float_prefix("120ms"), Some(120.0));
        assert_eq!(parse_float_prefix(".5s"), Some(0.5));
        assert_eq!(parse_float_prefix(" 200 "), Some(200.0));
        assert_eq!(parse_float_prefix("0.3"), Some(0.3));
        assert_eq!(parse_float_prefix("1e3x"), Some(1000.0));
        assert_eq!(parse_float_prefix("1e"), Some(1.0));
        assert_eq!(parse_float_prefix("1e+2"), Some(100.0));
        assert_eq!(parse_float_prefix("1e-2"), Some(0.01));
        assert_eq!(parse_float_prefix("-4.25rem"), Some(-4.25));
        assert_eq!(parse_float_prefix("+7"), Some(7.0));
        assert_eq!(parse_float_prefix("5."), Some(5.0));
        assert_eq!(parse_float_prefix("\n\t 12"), Some(12.0));
        assert_eq!(parse_float_prefix("Infinity!"), Some(f64::INFINITY));
        assert_eq!(parse_float_prefix("none"), None);
        assert_eq!(parse_float_prefix(""), None);
        assert_eq!(parse_float_prefix("  "), None);
        assert_eq!(parse_float_prefix("."), None);
        assert_eq!(parse_float_prefix(".e3"), None);
        assert_eq!(parse_float_prefix("-"), None);
        assert_eq!(parse_float_prefix("px"), None);
        // `parseFloat(css(...)) || 0` is what the source does with all of that.
        assert_eq!(css_number(&dom::root(), "--nothing"), 0.0);
    }

    #[test]
    fn reads_a_whole_number_or_nothing() {
        assert_eq!(to_number("0.4"), Some(0.4));
        assert_eq!(to_number(" 12 "), Some(12.0));
        assert_eq!(to_number(""), Some(0.0));
        assert_eq!(to_number("   "), Some(0.0));
        assert_eq!(to_number("0.4x"), None);
        assert_eq!(to_number("abc"), None);
    }

    #[test]
    fn resumes_only_a_fresh_bar() {
        let now = 1_700_000_000_000.0;
        assert_eq!(resume_from("0.4 1699999999000", now), Some(0.4));
        assert_eq!(resume_from("0.99 1699999999000", now), Some(0.95));
        // Older than 35s: that bar belongs to a page someone left long ago.
        assert_eq!(resume_from("0.4 1699999960000", now), None);
        // Exactly on the line is not past it.
        assert_eq!(resume_from("0.4 1699999965000", now), None);
        assert_eq!(resume_from("0.4 1699999965001", now), Some(0.4));
        // A bar that had not moved yet is not worth carrying over.
        assert_eq!(resume_from("0 1700000000000", now), None);
        assert_eq!(resume_from("-1 1700000000000", now), None);
        // Nothing stored, half stored, or stored by something else.
        assert_eq!(resume_from("", now), None);
        assert_eq!(resume_from("0.4", now), None);
        assert_eq!(resume_from("banana", now), None);
        assert_eq!(resume_from("banana split", now), None);
        assert_eq!(resume_from("0.4 later", now), None);
        assert_eq!(resume_from("0.4x 1699999999000", now), None);
        assert_eq!(resume_from("  ", now), None);
    }

    #[test]
    fn prints_numbers_the_way_the_page_reads_them() {
        assert_eq!(number(0.0, 6), "0");
        assert_eq!(number(42.0, 6), "42");
        assert_eq!(number(8.0, 6), "8");
        assert_eq!(number(0.5, 6), "0.5");
        assert_eq!(number(95.0, 6), "95");
        assert_eq!(number(1_700_000_000_000.0, 0), "1700000000000");
        assert_eq!(number(0.123_456_7, 6), "0.123457");
        assert_eq!(number(100.0, 6), "100");
        // What `set` writes: the curve's own start, as a percentage.
        assert_eq!(number(curve(0.0) * 100.0, 6), "8");
    }

    #[test]
    fn a_done_is_idempotent() {
        let ended = Rc::new(Cell::new(false));
        with(|state| state.open = 2);
        let done = Done(ended.clone());
        done.end();
        assert!(ended.get());
        assert_eq!(with(|state| state.open), 1);
        // The `limit` timer shares the flag: whichever went first is the one
        // that counted, and the second does nothing.
        close(&ended);
        assert_eq!(with(|state| state.open), 1);
        // And one that was never a load at all leaves the count alone.
        Done::inert().end();
        assert_eq!(with(|state| state.open), 1);
        with(|state| state.open = 0);
    }
}
