//! The allocator, because `no_std` does not come with one.
//!
//! `std`'s allocator on wasm is dlmalloc, and `std` as a whole costs about
//! eleven kilobytes of brotli here, which is more than the entire JavaScript
//! runtime this crate replaces. So the crate is `no_std` and brings its own.
//!
//! It is a segregated free list: every request is rounded up to a power of
//! two between 16 bytes and 32 KiB, and blocks of that size are kept on a
//! list of their own and handed straight back out next time. Anything larger
//! comes out of fresh memory and is never reused, which is fine because the
//! runtime's large allocations are page HTML — one at a time, and the browser
//! gives the memory back when the document goes away.
//!
//! No coalescing, so a page that allocated many 4 KiB blocks keeps that
//! memory for 4 KiB blocks. That is the right trade here: the runtime
//! allocates the same shapes over and over (headers, selectors, attribute
//! names, response text), so reuse within a class is almost total, and the
//! allocator itself is a few hundred bytes of wasm instead of several
//! kilobytes.

#[cfg(target_arch = "wasm32")]
use core::alloc::{GlobalAlloc, Layout};
#[cfg(target_arch = "wasm32")]
use core::cell::UnsafeCell;

/// The smallest block, and the alignment every block satisfies.
#[cfg(any(target_arch = "wasm32", test))]
const MIN: usize = 16;
/// Size classes from 16 bytes to 32 KiB: `MIN << (CLASSES - 1)`.
#[cfg(any(target_arch = "wasm32", test))]
const CLASSES: usize = 12;
/// A wasm page.
#[cfg(target_arch = "wasm32")]
const PAGE: usize = 65536;

#[cfg(target_arch = "wasm32")]
struct Heap {
    /// The head of each class's free list, as a pointer to the next free
    /// block; a free block's first word is the one after it.
    free: UnsafeCell<[usize; CLASSES]>,
    /// The next byte of memory nobody has been given yet.
    top: UnsafeCell<usize>,
    /// One past the last byte the module currently owns.
    end: UnsafeCell<usize>,
}

// SAFETY: `wasm32-unknown-unknown` is single threaded.
#[cfg(target_arch = "wasm32")]
unsafe impl Sync for Heap {}

/// Which class a request of `size` bytes falls in, or `None` when it is too
/// large for any of them.
#[cfg(any(target_arch = "wasm32", test))]
fn class_of(size: usize) -> Option<usize> {
    let mut class = 0;
    let mut block = MIN;
    while class < CLASSES {
        if size <= block {
            return Some(class);
        }
        block <<= 1;
        class += 1;
    }
    None
}

#[cfg(target_arch = "wasm32")]
impl Heap {
    /// Fresh memory, growing the module's own memory when there is not
    /// enough. `0` when the browser refuses to grow it.
    fn carve(&self, size: usize, align: usize) -> usize {
        // SAFETY: single threaded; no reference escapes this function.
        let top = unsafe { &mut *self.top.get() };
        let end = unsafe { &mut *self.end.get() };
        if *top == 0 {
            // `memory_size` counts the pages the module already has, which is
            // where the linker put the data segments and the stack.
            *top = core::arch::wasm32::memory_size(0) * PAGE;
            *end = *top;
        }
        let start = (*top + align - 1) & !(align - 1);
        let finish = match start.checked_add(size) {
            Some(finish) => finish,
            None => return 0,
        };
        if finish > *end {
            let pages = (finish - *end).div_ceil(PAGE);
            if core::arch::wasm32::memory_grow(0, pages) == usize::MAX {
                return 0;
            }
            *end += pages * PAGE;
        }
        *top = finish;
        start
    }
}

#[cfg(target_arch = "wasm32")]
static HEAP: Heap = Heap { free: UnsafeCell::new([0; CLASSES]), top: UnsafeCell::new(0), end: UnsafeCell::new(0) };

/// The runtime's allocator.
#[cfg(target_arch = "wasm32")]
pub struct Alloc;

#[cfg(target_arch = "wasm32")]
unsafe impl GlobalAlloc for Alloc {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        let align = layout.align().max(MIN);
        // A request that needs more alignment than a block guarantees is
        // carved fresh: the size classes are all `MIN`-aligned.
        let Some(class) = class_of(layout.size()).filter(|_| align <= MIN) else {
            return HEAP.carve(layout.size(), align) as *mut u8;
        };
        // SAFETY: single threaded; the borrow ends before anything else runs.
        let free = unsafe { &mut *HEAP.free.get() };
        let head = free[class];
        if head != 0 {
            // SAFETY: `head` is a block this allocator handed out and got
            // back, so its first word is the next free block.
            free[class] = unsafe { *(head as *const usize) };
            return head as *mut u8;
        }
        HEAP.carve(MIN << class, MIN) as *mut u8
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        let align = layout.align().max(MIN);
        let Some(class) = class_of(layout.size()).filter(|_| align <= MIN) else {
            // Carved fresh, and too large to be worth a list of its own.
            return;
        };
        // SAFETY: single threaded, and `ptr` is a block of exactly this class.
        let free = unsafe { &mut *HEAP.free.get() };
        unsafe { *(ptr as *mut usize) = free[class] };
        free[class] = ptr as usize;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn size_classes_cover_small_blocks_and_stop() {
        assert_eq!(class_of(0), Some(0));
        assert_eq!(class_of(16), Some(0));
        assert_eq!(class_of(17), Some(1));
        assert_eq!(class_of(32), Some(1));
        assert_eq!(class_of(33), Some(2));
        assert_eq!(class_of(MIN << (CLASSES - 1)), Some(CLASSES - 1));
        assert_eq!(class_of((MIN << (CLASSES - 1)) + 1), None);
        // A block of class `n` is `MIN << n` bytes, and every size between
        // the class below it and it lands in it.
        for class in 0..CLASSES {
            let size = MIN << class;
            assert_eq!(class_of(size), Some(class));
            assert_eq!(class_of(size - 1), Some(class), "one byte under a class boundary still fits it");
            if class > 0 {
                assert_eq!(class_of((MIN << (class - 1)) + 1), Some(class), "one byte over spills to the next");
            }
        }
    }
}
