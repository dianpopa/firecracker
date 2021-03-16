// Copyright 2021 Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0 OR BSD-3-Clause

//! Bitmap backend implementation based on atomic integers.

use std::sync::atomic::{AtomicU64, Ordering};

use crate::bitmap::{Bitmap, RefSlice, WithBitmapSlice};

#[cfg(feature = "backend-mmap")]
use crate::mmap::NewBitmap;

/// `Bitmap` implements a simple bit map on the page level with test and set operations. It is
/// page-size aware, so it converts addresses to page numbers before setting or clearing the bits.
#[derive(Debug)]
pub struct AtomicBitmap {
    map: Vec<AtomicU64>,
    size: usize,
    page_size: usize,
}

impl AtomicBitmap {
    /// Create a new bitmap of `byte_size`, with one bit per `page_size`.
    /// In reality this is rounded up, and you get a new vector of the next multiple of 64 bigger
    /// than `size` for free.
    pub fn new(byte_size: usize, page_size: usize) -> Self {
        // Bit size is the number of bits in the bitmap, always at least 1 (to store the state of
        // the '0' address).
        let bit_size = std::cmp::max(1, byte_size / page_size);
        // Create the map of `AtomicU64`, allowing the bit set operations to be done on a non-mut
        // `Bitmap`, avoiding the need for a Mutex or other serialization.
        let map_size = ((bit_size - 1) >> 6) + 1;
        let map: Vec<AtomicU64> = (0..map_size).map(|_| AtomicU64::new(0)).collect();

        AtomicBitmap {
            map,
            size: bit_size,
            page_size,
        }
    }

    /// Is bit `n` set? Bits outside the range of the bitmap are always unset.
    #[inline]
    pub fn is_bit_set(&self, n: usize) -> bool {
        if n <= self.size {
            (self.map[n >> 6].load(Ordering::Acquire) & (1 << (n & 63))) != 0
        } else {
            // Out-of-range bits are always unset.
            false
        }
    }

    /// Is the bit corresponding to address `addr` set?
    pub fn is_addr_set(&self, addr: usize) -> bool {
        self.is_bit_set(addr / self.page_size)
    }

    /// Set a range of bits starting at `start_addr` and continuing for the next `len` bytes.
    pub fn set_addr_range(&self, start_addr: usize, len: usize) {
        let first_bit = start_addr / self.page_size;
        let page_count = (len + self.page_size - 1) / self.page_size;
        for n in first_bit..(first_bit + page_count) {
            if n > self.size {
                // Attempts to set bits beyond the end of the bitmap are simply ignored.
                break;
            }
            self.map[n >> 6].fetch_or(1 << (n & 63), Ordering::SeqCst);
        }
    }

    /// Get the length of the bitmap in bits (i.e. in how many pages it can represent).
    pub fn len(&self) -> usize {
        self.size
    }

    /// Is the bitmap empty (i.e. has zero size)? This is always false, because we explicitly
    /// round up the size when creating the bitmap. We will not need this function but:
    /// https://rust-lang.github.io/rust-clippy/master/index.html#len_without_is_empty
    pub fn is_empty(&self) -> bool {
        false
    }

    /// Reset all bitmap bits to 0.
    pub fn reset(&self) {
        for it in self.map.iter() {
            it.store(0, Ordering::Release);
        }
    }
}

impl Clone for AtomicBitmap {
    fn clone(&self) -> Self {
        let map = self
            .map
            .iter()
            .map(|i| i.load(Ordering::Acquire))
            .map(AtomicU64::new)
            .collect();
        AtomicBitmap {
            map,
            size: self.size,
            page_size: self.page_size,
        }
    }
}

impl<'a> WithBitmapSlice<'a> for AtomicBitmap {
    type S = RefSlice<'a, Self>;
}

impl Bitmap for AtomicBitmap {
    fn mark_dirty(&self, offset: usize, len: usize) {
        self.set_addr_range(offset, len)
    }

    fn dirty_at(&self, offset: usize) -> bool {
        self.is_addr_set(offset)
    }

    fn slice_at(&self, offset: usize) -> <Self as WithBitmapSlice>::S {
        RefSlice::new(self, offset)
    }
}

impl Default for AtomicBitmap {
    fn default() -> Self {
        AtomicBitmap::new(0, 0x1000)
    }
}

#[cfg(feature = "backend-mmap")]
impl NewBitmap for AtomicBitmap {
    fn with_len(len: usize) -> Self {
        use std::convert::TryFrom;

        // There's no unsafe potential in calling this function.
        let page_size = unsafe { libc::sysconf(libc::_SC_PAGE_SIZE) };
        // The `unwrap` is safe to use because the above call should always succeed on the
        // supported platforms, and the size of a page will always fit within a `usize`.
        AtomicBitmap::new(len, usize::try_from(page_size).unwrap())
    }
}

// #[cfg(test)]
// pub mod tests {
//     use super::*;
//
//     use crate::bitmap::tests::test_bitmap;
//
//     #[test]
//     fn bitmap_basic() {
//         let b = AtomicBitmap::new(1024, 128);
//         assert_eq!(b.is_empty(), false);
//         assert_eq!(b.len(), 8);
//         b.set_addr_range(128, 129);
//         assert!(!b.is_addr_set(0));
//         assert!(b.is_addr_set(128));
//         assert!(b.is_addr_set(256));
//         assert!(!b.is_addr_set(384));
//
//         #[allow(clippy::redundant_clone)]
//         let copy_b = b.clone();
//         assert!(copy_b.is_addr_set(256));
//         assert!(!copy_b.is_addr_set(384));
//
//         b.reset();
//         assert!(!b.is_addr_set(128));
//         assert!(!b.is_addr_set(256));
//         assert!(!b.is_addr_set(384));
//     }
//
//     #[test]
//     fn bitmap_out_of_range() {
//         let b = AtomicBitmap::new(1024, 128);
//         // Set a partial range that goes beyond the end of the bitmap
//         b.set_addr_range(768, 512);
//         assert!(b.is_addr_set(768));
//         // The bitmap is never set beyond its end
//         assert!(!b.is_addr_set(1152));
//     }
//
//     #[test]
//     fn test_bitmap_impl() {
//         let b = AtomicBitmap::new(0x2000, 128);
//         test_bitmap(&b);
//     }
// }
