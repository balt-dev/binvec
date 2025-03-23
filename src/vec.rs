use core::{alloc::{Layout, LayoutError}, fmt::{Debug, Display}, hint::assert_unchecked, ops::Index, ptr::NonNull};

use _alloc::{alloc, boxed::Box, vec::Vec};

use crate::borrow::{BinarySlice, BitBuffer, Const, Mut, MutableBinSlice};

pub struct BinaryVec<Word: BitBuffer + 'static = usize> { length: usize, buf: BinarySlice<'static, Mut, Word> }

impl<Word: BitBuffer + 'static> Debug for BinaryVec<Word> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        Debug::fmt(&self.as_slice(), f)
    }
}

impl<Word: BitBuffer + 'static> Display for BinaryVec<Word> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        Display::fmt(&self.as_slice(), f)
    }
}

impl<Word: BitBuffer + 'static> BinaryVec<Word> {
    pub(crate) const fn layout(capacity: usize) -> Result<Layout, LayoutError> {
        Layout::array::<usize>(capacity.div_ceil(Word::WIDTH))
    }
    
    pub(crate) fn grow(&mut self) {
        let capacity = self.buf.len();

        let (new_cap, new_layout) = if capacity == 0 {
            (Word::WIDTH, Self::layout(Word::WIDTH).unwrap())
        } else {
            let new_cap = 2 * capacity;
            let new_layout = Self::layout(new_cap).unwrap();
            (new_cap, new_layout)
        };

        assert!(new_layout.size() <= isize::MAX as usize, "allocated binary array length does not fit in a positive isize");

        let new_ptr = if capacity == 0 {
            unsafe { alloc::alloc(new_layout) }
        } else {
            let (old_data, _, _) = self.as_slice().into_word_slice();
            let old_layout = Self::layout(capacity).unwrap();
            let old_ptr = old_data.as_ptr() as *mut Word as *mut u8;
            unsafe { alloc::realloc(old_ptr, old_layout, new_layout.size()) }
        };

        self.buf.data = match NonNull::new(new_ptr as *mut Word) {
            Some(p) => p,
            None => alloc::handle_alloc_error(new_layout),
        };
        self.buf.length = new_cap;
    }

    /// Constructs a new, empty binary vector.
    pub const fn new() -> Self {
        unsafe { Self::from_raw_parts(0, 0, 0, NonNull::dangling()) }
    }

    /// Constructs a new, empty binary vector, with a given bit capacity.
    pub fn with_capacity(capacity: usize) -> Self {
        if capacity == 0 { return Self::new() }
        let layout = Self::layout(capacity).expect("failed to build layout for given capacity");

        let new_ptr = unsafe { alloc::alloc(layout) };

        let data_ptr = match NonNull::new(new_ptr as *mut Word) {
            Some(p) => p,
            None => alloc::handle_alloc_error(layout),
        };
        let buf = unsafe {
            BinarySlice::<'static, Mut, Word>::from_raw_parts(0, capacity, data_ptr)
        };
        Self { length: 0, buf }
    }

    /// Constructs a binary vector from a start, length, capacity, and pointer.
    /// 
    /// # Safety
    /// - The pointer must be pointing to a region of memory that is valid up until `ceil((start + capacity) / Word::WIDTH)` bytes
    /// - The length and capacity must not > `isize::MAX`
    /// - The length must be less than or equal to the capacity
    pub const unsafe fn from_raw_parts(start: usize, length: usize, capacity: usize, data: NonNull<Word>) -> Self {
        unsafe {
            assert_unchecked(length as isize >= 0);
            Self { length, buf: BinarySlice::from_raw_parts(start, capacity, data) }
        }
    }

    /// Pushes a single bit to the vector.
    pub fn push(&mut self, bit: bool) {
        if self.length == self.buf.len() { self.grow(); }
        let len = self.length;
        unsafe { self.capacity_slice_mut().set_unchecked(len, bit) };
        self.length += 1;
    }

    /// Pops a single bit from the vector, or returns `None` if it's empty.
    pub fn pop(&mut self) -> Option<bool> {
        (self.length != 0).then(|| {
            self.length -= 1;
            let v = unsafe { self.capacity_slice().get_unchecked(self.length) };
            v
        })
    }

    /// Borrows the whole vector as a binary slice.
    pub const fn as_slice(&self) -> BinarySlice<'_, Const, Word> {
        unsafe { BinarySlice::from_raw_parts(
            0,
            self.length,
            self.buf.data
        ) }
    }

    /// Mutably borrows the whole vector as a binary slice.
    pub const fn as_mut_slice(&mut self) -> BinarySlice<'_, Mut, Word> {
        unsafe { BinarySlice::from_raw_parts(
            0,
            self.length,
            self.buf.data
        ) }
    }

    /// Returns the number of words currently completely allocated by the vector.
    pub const fn filled_words(&self) -> usize {
        (self.length / Word::WIDTH) * Word::WIDTH
    }

    /// Borrows the whole vector, including deallocated bits, as a binary slice.
    /// 
    /// # Safety
    /// Bits past the length of the vector must not be read from until written to.
    pub const unsafe fn capacity_slice(&self) -> BinarySlice<'_, Const, Word> {
        unsafe { BinarySlice::from_raw_parts(
            0,
            self.buf.len(),
            self.buf.data
        ) }
    }

    /// Mutably borrows the whole vector, including deallocated bits, as a binary slice.
    /// 
    /// # Safety
    /// Bits past the length of the vector must not be read from until written to.
    pub const unsafe fn capacity_slice_mut(&mut self) -> BinarySlice<'_, Mut, Word> {
        unsafe { BinarySlice::from_raw_parts(
            0,
            self.buf.len(),
            self.buf.data
        ) }
    }
}

impl<Word: BitBuffer + 'static + Copy> BinaryVec<Word> {
    /// Copies data from a slice to a new binary vector.
    /// 
    /// ```rust
    /// # use binvec::prelude::*;
    /// let s = binslice![1 1 0 1 0];
    /// let mut v = BinaryVec::copy_from_slice(s);
    /// v.push(true);
    /// assert!(v[5]);
    /// ```
    pub fn copy_from_slice<'lt>(slice: BinarySlice<'lt, Const, Word>) -> Self {
        let (src, start, length) = slice.into_word_slice();
        let mut this = Self::with_capacity(src.len() * Word::WIDTH);
        this.buf.start = start;
        this.buf.length = length;
        let dst = unsafe { this.capacity_slice_mut() }.into_word_slice();
        dst.copy_from_slice(src);
        this.length = slice.len();
        this
    }
}

impl<Word: BitBuffer + 'static> FromIterator<Word> for BinaryVec<Word> {
    fn from_iter<T: IntoIterator<Item = Word>>(iter: T) -> Self {
        let arr = iter.into_iter().collect::<Box<[_]>>();
        let len = arr.len();
        let ptr = NonNull::new(Box::leak(arr) as *mut _ as *mut Word).unwrap();
        unsafe {
            Self::from_raw_parts(0, len * Word::WIDTH, len * Word::WIDTH, ptr)
        }
    }
}

impl<Word: BitBuffer + 'static> FromIterator<bool> for BinaryVec<Word> {
    fn from_iter<T: IntoIterator<Item = bool>>(iter: T) -> Self {
        let mut filled_words = Vec::new();
        let mut word_buf = None;
        let mut word_index = 0;

        for value in iter {
            word_buf = Some(word_buf.unwrap_or(Word::ZERO));
            unsafe { word_buf.as_mut().unwrap().set_bit(word_index, value); }
            if word_index >= Word::WIDTH {
                filled_words.push(word_buf.take().unwrap());
                word_index = 0;
            }
            word_index += 1;
        }

        if word_index != 0 {
            filled_words.push(word_buf.take().unwrap());
        }

        let len = filled_words.len() * Word::WIDTH - ((Word::WIDTH - word_index) % Word::WIDTH);
        let cap = filled_words.capacity() * Word::WIDTH;

        let ptr = NonNull::new(Vec::leak(filled_words) as *mut _ as *mut Word).unwrap();
        unsafe {
            Self::from_raw_parts(0, len, cap, ptr)
        }
    }
}

impl<Word: BitBuffer + 'static> Index<usize> for BinaryVec<Word> {
    type Output = bool;

    fn index(&self, index: usize) -> &Self::Output {
        if self.as_slice()[index] { &true } else { &false }
    }
}

/// Creates a new vector from a static slice. See [`crate::binslice`].
/// 
/// # Examples
/// ```rust
/// # use binvec::prelude::*;
/// let mut v = binvec![1 1 0 1 1];
/// v.push(true);
/// assert!(v[5]);
/// ```
#[macro_export]
macro_rules! binvec {
    [] => {BinaryVec::new()};
    [$($tt: tt)*] => {
        BinaryVec::copy_from_slice(binslice![$($tt)*])
    }
}