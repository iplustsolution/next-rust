//! The allocator islands run on.
//!
//! The first version of this was a bump allocator with a no-op `dealloc`,
//! which is fine for an island that allocates a handful of strings at
//! hydration and then stops. It is not fine for the two islands that made us
//! replace it: a tutor that builds and tears down DOM per streamed token, and
//! a scene that allocates per animation frame. Both churn — they allocate and
//! free the same shapes forever — and against a bump allocator that is a slow
//! leak that ends in a trap.
//!
//! So this is a real allocator, kept to the smallest design that handles churn
//! well: power-of-two size classes, a free list per class, and fresh pages from
//! `memory.grow` when a list is empty. Freed blocks go back on their class's
//! list and are reused immediately.
//!
//! What it deliberately does not do is coalesce adjacent free blocks. Merging
//! needs a boundary tag on every allocation — a word of overhead on every
//! string an island ever makes — to fix a fragmentation pattern that island
//! code does not have: islands allocate a few distinct shapes over and over,
//! and a class list reuses those perfectly. The cost of that choice, stated
//! honestly: a module that allocates a large transient and then wants that
//! exact memory back in small pieces will grow memory instead of reusing it.
//!
//! Single-threaded by construction — wasm islands run on the browser's one
//! thread — which is what makes the `static mut` state below sound.

use core::alloc::{GlobalAlloc, Layout};

/// Blocks below this are rounded up to it: a free block has to hold the `next`
/// pointer threaded through it, and eight bytes also covers `f64` alignment.
const MIN: usize = 8;

/// Size classes are `MIN << i`, so this covers 8 bytes to 8 KiB. Anything
/// larger goes on the oversized list, which is searched first-fit.
const CLASSES: usize = 11;

/// One wasm page.
const PAGE: usize = 64 * 1024;

/// Heads of the free lists: one per class, then the oversized list. A `0` head
/// is an empty list, which is why the heap never starts at address zero.
static mut FREE: [usize; CLASSES + 1] = [0; CLASSES + 1];

/// Next never-yet-allocated address, and the end of the memory we own.
static mut BRK: usize = 0;
static mut END: usize = 0;

/// Off the browser there is no `memory.grow`, so the heap is a fixed arena.
/// This exists so the crate's pure parts can be tested with `cargo test` on
/// the machine running it; the browser never sees it.
#[cfg(not(target_arch = "wasm32"))]
const HOST_ARENA: usize = 4 * 1024 * 1024;
#[cfg(not(target_arch = "wasm32"))]
static mut HOST_HEAP: [u8; HOST_ARENA] = [0; HOST_ARENA];

// Where the linker put the end of the module's static data.
#[cfg(target_arch = "wasm32")]
unsafe extern "C" {
    static __heap_base: u8;
}

/// The class a request falls into, and the block size that class hands out.
///
/// Alignment is folded into the size: classes are powers of two and blocks are
/// cut from an address aligned to their own size, so a block big enough is a
/// block aligned enough.
#[inline]
fn class_of(layout: Layout) -> (usize, usize) {
    let want = layout.size().max(layout.align()).max(MIN);
    let mut size = MIN;
    let mut class = 0;
    while size < want && class < CLASSES {
        size <<= 1;
        class += 1;
    }
    if class >= CLASSES {
        // Oversized: rounded to eight bytes and tracked with its length.
        (CLASSES, (want + MIN - 1) & !(MIN - 1))
    } else {
        (class, size)
    }
}

/// Claim `size` bytes of fresh memory, aligned to `align`, growing the wasm
/// memory when what we own is not enough. Returns 0 when memory cannot grow.
///
/// # Safety
/// The caller must hold the (implicit, single-threaded) heap lock.
unsafe fn fresh(size: usize, align: usize) -> usize {
    unsafe {
        if BRK == 0 {
            BRK = start();
            END = limit();
        }
        let at = (BRK + align - 1) & !(align - 1);
        let end = match at.checked_add(size) {
            Some(end) => end,
            None => return 0,
        };
        if end > END {
            let need = end - END;
            let pages = need.div_ceil(PAGE);
            if !grow(pages) {
                return 0;
            }
            END += pages * PAGE;
        }
        BRK = end;
        at
    }
}

#[cfg(target_arch = "wasm32")]
unsafe fn start() -> usize {
    (&raw const __heap_base) as usize
}

#[cfg(target_arch = "wasm32")]
fn limit() -> usize {
    core::arch::wasm32::memory_size(0) * PAGE
}

#[cfg(target_arch = "wasm32")]
fn grow(pages: usize) -> bool {
    core::arch::wasm32::memory_grow(0, pages) != usize::MAX
}

#[cfg(not(target_arch = "wasm32"))]
unsafe fn start() -> usize {
    (&raw const HOST_HEAP) as usize
}

#[cfg(not(target_arch = "wasm32"))]
unsafe fn limit() -> usize {
    unsafe { start() + HOST_ARENA }
}

#[cfg(not(target_arch = "wasm32"))]
fn grow(_pages: usize) -> bool {
    false
}

/// The global allocator for a module built with [`crate::islands!`].
pub struct Heap;

unsafe impl GlobalAlloc for Heap {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        unsafe {
            let (class, size) = class_of(layout);
            if class < CLASSES {
                let head = FREE[class];
                if head != 0 {
                    FREE[class] = *(head as *const usize);
                    return head as *mut u8;
                }
                return fresh(size, size) as *mut u8;
            }
            // Oversized. First fit over the list, remembering the previous
            // block so it can be unlinked, then fresh memory.
            let mut prev = 0usize;
            let mut cur = FREE[CLASSES];
            while cur != 0 {
                let have = *((cur + MIN) as *const usize);
                if have >= size {
                    let next = *(cur as *const usize);
                    if prev == 0 {
                        FREE[CLASSES] = next;
                    } else {
                        *(prev as *mut usize) = next;
                    }
                    return cur as *mut u8;
                }
                prev = cur;
                cur = *(cur as *const usize);
            }
            fresh(size, layout.align().max(MIN)) as *mut u8
        }
    }

    unsafe fn dealloc(&self, at: *mut u8, layout: Layout) {
        unsafe {
            if at.is_null() {
                return;
            }
            let (class, size) = class_of(layout);
            let at = at as usize;
            *(at as *mut usize) = FREE[class];
            if class == CLASSES {
                // A free block has no user in it, so its own second word is
                // where the length goes. Nothing is stored while it is live,
                // which is why an allocation carries no header at all.
                *((at + MIN) as *mut usize) = size;
            }
            FREE[class] = at;
        }
    }
}

/// How many bytes the heap has taken from the browser, and how much of that is
/// still in use. Islands do not need this; the size report and the tests do.
pub fn stats() -> (usize, usize) {
    unsafe { (END, BRK) }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn classes_round_up_to_a_power_of_two_that_is_also_aligned_enough() {
        for (size, align, want) in
            [(1usize, 1usize, 8usize), (8, 1, 8), (9, 1, 16), (16, 16, 16), (17, 32, 32), (4096, 8, 4096)]
        {
            let layout = Layout::from_size_align(size, align).unwrap();
            let (class, block) = class_of(layout);
            assert!(class < CLASSES, "{size}/{align} should not be oversized");
            assert_eq!(block, want, "{size}/{align}");
            assert!(block >= size && block >= align);
        }
        // Past the last class the request is only rounded to the word size.
        let (class, block) = class_of(Layout::from_size_align(9000, 8).unwrap());
        assert_eq!(class, CLASSES);
        assert_eq!(block, 9000);
    }

    /// The heap is process-wide state and `cargo test` runs tests on several
    /// threads, so the allocator's behaviour is asserted in one test rather
    /// than three. That is not a workaround for a real race: on the browser
    /// there is only ever one thread, which is the assumption the allocator is
    /// built on.
    #[test]
    fn the_allocator_reuses_what_it_is_given_back() {
        churn_reuses_memory_instead_of_growing();
        oversized_blocks_are_reused_when_one_is_big_enough();
        distinct_live_allocations_never_overlap();
    }

    /// The property the tutor and scene islands depend on: allocate and free
    /// the same shape forever, and the heap must stop growing.
    fn churn_reuses_memory_instead_of_growing() {
        let heap = Heap;
        let layout = Layout::from_size_align(48, 8).unwrap();
        let first = unsafe { heap.alloc(layout) };
        assert!(!first.is_null());
        unsafe { heap.dealloc(first, layout) };
        for _ in 0..10_000 {
            let at = unsafe { heap.alloc(layout) };
            assert_eq!(at, first, "a freed block of the same class must come back");
            unsafe { heap.dealloc(at, layout) };
        }
    }

    fn oversized_blocks_are_reused_when_one_is_big_enough() {
        let heap = Heap;
        let big = Layout::from_size_align(40_000, 8).unwrap();
        let a = unsafe { heap.alloc(big) };
        assert!(!a.is_null());
        unsafe { heap.dealloc(a, big) };
        let b = unsafe { heap.alloc(Layout::from_size_align(30_000, 8).unwrap()) };
        assert_eq!(a, b, "a smaller oversized request fits the block we just freed");
    }

    fn distinct_live_allocations_never_overlap() {
        let heap = Heap;
        let mut live = alloc::vec::Vec::new();
        for size in [8usize, 9, 33, 120, 1000] {
            let layout = Layout::from_size_align(size, 8).unwrap();
            for _ in 0..16 {
                let at = unsafe { heap.alloc(layout) } as usize;
                assert_ne!(at, 0);
                for (other, other_size) in &live {
                    let (other, other_size): (usize, usize) = (*other, *other_size);
                    assert!(at + size <= other || other + other_size <= at, "{at}+{size} overlaps {other}");
                }
                live.push((at, size));
            }
        }
    }
}
