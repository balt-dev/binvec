use core::{alloc::{Layout, LayoutError}, hint::assert_unchecked, ptr::NonNull};
use _alloc::alloc;

use crate::borrow::BitBuffer;

#[derive(Debug, PartialEq, Eq)]
pub(crate) struct RawBinaryVec<Word: BitBuffer = usize> { pub capacity: usize, pub data: NonNull<Word> }

impl<Word: BitBuffer> RawBinaryVec<Word> {
    /// Constructs a raw binary vector from a capacity, and pointer.
    /// 
    /// # Safety
    /// - The pointer must be pointing to a region of memory that is valid up until `ceil(capacity / size_of::<usize>())`
    /// - The capacity must not be > `isize::MAX`
    pub(crate) const unsafe fn from_raw_parts(capacity: usize, data: NonNull<Word>) -> Self {
        unsafe {
            // We're emulating the "Cap" type from std by doing this.
            assert_unchecked(capacity as isize >= 0);
        }
        Self { capacity, data }
    }

    pub(crate) const fn layout(capacity: usize) -> Result<Layout, LayoutError> {
        Layout::array::<usize>(capacity.div_ceil(Word::WIDTH))
    }

    pub(crate) fn grow(&mut self) {
        let (new_cap, new_layout) = if self.capacity == 0 {
            (Word::WIDTH, Self::layout(Word::WIDTH).unwrap())
        } else {
            let new_cap = 2 * self.capacity;
            let new_layout = Self::layout(new_cap).unwrap();
            (new_cap, new_layout)
        };

        assert!(new_layout.size() <= isize::MAX as usize, "allocated binary array length does not fit in a positive isize");

        let new_ptr = if self.capacity == 0 {
            unsafe { alloc::alloc(new_layout) }
        } else {
            let old_layout = Self::layout(self.capacity).unwrap();
            let old_ptr = self.data.as_ptr() as *mut u8;
            unsafe { alloc::realloc(old_ptr, old_layout, new_layout.size()) }
        };

        self.data = match NonNull::new(new_ptr as *mut Word) {
            Some(p) => p,
            None => alloc::handle_alloc_error(new_layout),
        };
        self.capacity = new_cap;
    }
}

impl<Word: BitBuffer> Drop for RawBinaryVec<Word> {
    fn drop(&mut self) {
        if self.capacity != 0 {
            unsafe {
                alloc::dealloc(
                    self.data.as_ptr() as *mut u8,
                    Self::layout(self.capacity).expect("failed to create array layout for binary vec drop")
                )
            }
        }
    }
}