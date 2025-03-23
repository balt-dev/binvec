use core::{fmt::{Debug, Display}, hint::assert_unchecked, ptr::NonNull};

use crate::{
    borrow::{BinarySlice, BitBuffer, Const, Mut, MutableBinSlice},
    raw_vec::RawBinaryVec
};

#[derive(PartialEq, Eq)]
pub struct BinaryVec<Word: BitBuffer = usize> { length: usize, buf: RawBinaryVec<Word> }

impl<Word: BitBuffer> Debug for BinaryVec<Word> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        Debug::fmt(&self.as_slice(), f)
    }
}

impl<Word: BitBuffer> Display for BinaryVec<Word> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        Display::fmt(&self.as_slice(), f)
    }
}

impl<Word: BitBuffer> BinaryVec<Word> {
    pub const fn new() -> Self {
        unsafe { Self::from_raw_parts(0, 0, NonNull::dangling()) }
    }
    /// Constructs a binary vector from a length, capacity, and pointer.
    /// 
    /// # Safety
    /// - The pointer must be pointing to a region of memory that is valid up until `ceil(capacity / Word::WIDTH)` bytes
    /// - The length and capacity must not > `isize::MAX`
    /// - The length must be less than or equal to the capacity
    pub const unsafe fn from_raw_parts(length: usize, capacity: usize, data: NonNull<Word>) -> Self {
        unsafe {
            // We're emulating the "Cap" type from std by doing this.
            assert_unchecked(length as isize >= 0);
        }
        Self { length, buf: unsafe { RawBinaryVec::from_raw_parts(capacity, data) } }
    }

    /// Pushes a single bit to the vector.
    pub fn push(&mut self, bit: bool) {
        if self.length == self.buf.capacity { self.buf.grow(); }
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
    pub const fn as_slice_mut(&mut self) -> BinarySlice<'_, Mut, Word> {
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

    /// Borrows the whole vector as a slice of its words.
    /// Returns a slice containing trailing data that does not fit in a word.
    pub fn as_word_slice<'this>(&'this self) -> (&'this [Word], BinarySlice<'this, Const, Word>) {
        let slice = unsafe {
            core::slice::from_raw_parts(self.buf.data.as_ptr(), self.length / Word::WIDTH)
        };
        (slice, self.as_slice().sub(..self.filled_words()).expect("filled words should be in bounds"))
    }

    /// Mutably borrows the whole vector as a slice of its words.
    /// Returns a slice containing trailing data that does not fit in a word.
    pub fn as_word_slice_mut<'this>(&'this mut self) -> (&'this mut [Word], BinarySlice<'this, Mut, Word>) {
        let slice = unsafe {
            core::slice::from_raw_parts_mut(self.buf.data.as_ptr(), self.length.div_ceil(Word::WIDTH))
        };
        let filled_words = self.filled_words();
        (slice, self.as_slice_mut().sub(..filled_words).expect("filled words should be in bounds"))
    }

    /// Borrows the whole vector, including deallocated bits, as a binary slice.
    /// 
    /// # Safety
    /// Bits past the length of the vector must not be read from until written to.
    pub const unsafe fn capacity_slice(&self) -> BinarySlice<'_, Const, Word> {
        unsafe { BinarySlice::from_raw_parts(
            0,
            self.buf.capacity,
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
            self.buf.capacity,
            self.buf.data
        ) }
    }
}

impl<Word: BitBuffer> BinaryVec<Word> where for<'s> BinarySlice<'s, Const, Word>: MutableBinSlice<'s, Word> {
    pub fn push_shared(&mut self, bit: bool) {
        if self.length == self.buf.capacity { self.buf.grow(); }
        unsafe { self.capacity_slice().set((self.length + 1) as usize, bit) };
        self.length += 1;
    }
}
