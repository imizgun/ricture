use rustix::mm::{Advice, madvise};

/// Size of a transparent huge page on x86-64.
const HUGE_PAGE: usize = 2 << 20;

/// A zeroed buffer for screen-sized pixel data, which asks the kernel for huge
/// pages.
///
/// Every 4 KiB page of a fresh buffer costs a page fault on first write; for a
/// 2880x1800 frame that is ~5000 faults and ~15 ms, a large share of startup.
/// With huge pages it is a handful of faults (~4 ms). Many distributions only
/// grant them on request (`transparent_hugepage=madvise`), hence the hint;
/// where they are unavailable this is an ordinary buffer.
pub fn alloc_frame(len: usize) -> Vec<u8> {
    // Large zeroed allocations come straight from fresh `mmap`ed memory, so
    // no page has been touched yet when the hint is given.
    let buffer = vec![0u8; len];
    advise_huge_pages(&buffer);
    buffer
}

fn advise_huge_pages(buffer: &[u8]) {
    let start = buffer.as_ptr() as usize;
    // madvise works on whole pages; only the aligned middle can be huge anyway.
    let first = start.next_multiple_of(HUGE_PAGE);
    let end = (start + buffer.len()) & !(HUGE_PAGE - 1);
    if end <= first {
        return;
    }
    // SAFETY: the range lies inside `buffer`'s allocation, and this advice
    // only affects how the kernel backs the memory, never its contents.
    let _ = unsafe { madvise(first as *mut _, end - first, Advice::LinuxHugepage) };
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn frame_is_zeroed_and_sized() {
        for len in [0, 7, HUGE_PAGE + 13, 3 * HUGE_PAGE] {
            let frame = alloc_frame(len);
            assert_eq!(frame.len(), len);
            assert!(frame.iter().all(|&b| b == 0));
        }
    }
}
