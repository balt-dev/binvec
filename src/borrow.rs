use core::{cell::Cell, fmt::{Debug, Display}, hint::assert_unchecked, marker::PhantomData, ops::{Deref, Index, IndexMut, RangeBounds}, ptr::NonNull};


mod seal {
    use core::cell::Cell;
    use super::*;

    pub trait MutSeal {}
    impl<'slice, Word: SharedBitBuffer> MutSeal for BinarySlice<'slice, Const, Word> {}
    impl<'slice, Word: BitBuffer> MutSeal for BinarySlice<'slice, Mut, Word> {}

    pub trait BorrowSeal {}
    impl BorrowSeal for Const {}
    impl BorrowSeal for Mut {}

    pub trait BitSeal {}
    impl BitSeal for u8 {}
    impl BitSeal for u16 {}
    impl BitSeal for u32 {}
    impl BitSeal for u64 {}
    impl BitSeal for u128 {}
    impl BitSeal for usize {}
    impl BitSeal for i8 {}
    impl BitSeal for i16 {}
    impl BitSeal for i32 {}
    impl BitSeal for i64 {}
    impl BitSeal for i128 {}
    impl BitSeal for isize {}
    impl<T: BitSeal, const N: usize> BitSeal for [T; N] {}
    impl<T: BitSeal + Copy> BitSeal for Cell<T> {}
}

pub trait BitBuffer: seal::BitSeal {
    const WIDTH: usize;
    const ZERO: Self;
    /// Set the bit at the given index, assuming the index is in bounds.
    /// 
    /// # Safety
    /// The index must be < `Self::WIDTH`.
    unsafe fn set_bit(&mut self, index: usize, value: bool);
    /// Get the bit at the given index, assuming the index is in bounds.
    /// 
    /// # Safety
    /// The index must be < `Self::WIDTH`.
    unsafe fn get_bit(&self, index: usize) -> bool; 
}

/// A bit buffer that represents shared memory.
pub trait SharedBitBuffer: BitBuffer {
    /// Set the bit at the given index, assuming the index is in bounds.
    /// 
    /// # Safety
    /// The index must be < `Self::WIDTH`.
    unsafe fn set_shared_bit(&self, index: usize, value: bool);
}

macro_rules! bitwidth {
    ($($ty: ty: $uty: ty)*) => {
        $(
        impl BitBuffer for $uty {
            const WIDTH: usize = core::mem::size_of::<$uty>() * 8;
            const ZERO: Self = 0;
            unsafe fn set_bit(&mut self, index: usize, value: bool) {
                unsafe {
                    *self &= (!Self::ZERO) ^ (1 as Self).checked_shl(index as u32).unwrap_unchecked();
                    *self |= (value as Self).checked_shl(index as u32).unwrap_unchecked();
                }
            }
            unsafe fn get_bit(&self, index: usize) -> bool {
                (unsafe {
                    (
                        *self & (1 as Self)
                        .checked_shl(index as u32).unwrap_unchecked()
                    ).checked_shr(index as u32).unwrap_unchecked()
                }) > 0
            }
        }
        impl BitBuffer for $ty {
            const WIDTH: usize = core::mem::size_of::<$ty>() * 8;
            const ZERO: Self = 0;
            unsafe fn set_bit(&mut self, index: usize, value: bool) {
                unsafe {
                    assert_unchecked(index < Self::WIDTH);
                }
                *self &= ((!Self::ZERO as $uty) ^ (1 as $uty) << (index as u32)) as $ty;
                *self |= ((value as $uty) << (index as u32)) as $ty;
            }
            unsafe fn get_bit(&self, index: usize) -> bool {
                unsafe {
                    assert_unchecked(index < Self::WIDTH);
                }
                (((*self as $uty) & ((1 as $uty) << index)) >> index) > 0
            }
        })*
    };
}

bitwidth!(i8: u8 i16: u16 i32: u32 i64: u64 i128: u128 isize: usize);

impl<T: BitBuffer, const N: usize> BitBuffer for [T; N] {
    const WIDTH: usize = T::WIDTH * N;
    const ZERO: Self = [T::ZERO; N];
    unsafe fn get_bit(&self, index: usize) -> bool {
        let arr_index = index / T::WIDTH;
        unsafe { self[arr_index].get_bit(index % T::WIDTH) }
    }
    unsafe fn set_bit(&mut self, index: usize, value: bool) {
        let arr_index = index / T::WIDTH;
        unsafe { self[arr_index].set_bit(index % T::WIDTH, value) }
    }
}
impl<T: BitBuffer + Copy> BitBuffer for Cell<T> {
    const WIDTH: usize = T::WIDTH;
    const ZERO: Self = Cell::new(T::ZERO);
    unsafe fn set_bit(&mut self, index: usize, value: bool) {
        unsafe { 
            self.set_shared_bit(index, value)
        }
    }
    unsafe fn get_bit(&self, index: usize) -> bool {
        unsafe { 
            self.get().get_bit(index)
        }
    }
}


impl<T: SharedBitBuffer, const N: usize> SharedBitBuffer for [T; N] {
    unsafe fn set_shared_bit(&self, index: usize, value: bool) {
        let arr_index = index / Self::WIDTH;
        unsafe { self[arr_index].set_shared_bit(index % Self::WIDTH, value) }
    }
}
impl<T: BitBuffer + Copy> SharedBitBuffer for Cell<T> {
    unsafe fn set_shared_bit(&self, index: usize, value: bool) {
        let mut inner = self.get();
        unsafe { inner.set_bit(index, value) };
        self.set(inner);
    }
}

#[cfg(not(tarpaulin_include))]
pub trait BorrowType: seal::BorrowSeal { }

#[cfg(not(tarpaulin_include))]
/// Marks a binary borrow as immutable.
pub enum Const {}
impl BorrowType for Const { }
#[cfg(not(tarpaulin_include))]
/// Marks a binary borrow as mutable.
pub enum Mut {}
impl BorrowType for Mut { }

/// A pointer to a binary slice.
#[repr(C)]
pub struct BinarySlice<'slice, Ref: BorrowType = Const, Word: BitBuffer + 'slice = usize> {
    pub(crate) start: usize,
    pub(crate) length: usize,
    pub(crate) data: NonNull<Word>,
    pub(crate) _this_struct_is_unsafe_to_brace_init: PhantomData<&'slice (Ref, [Word])>
}

impl<'slice, Ref: BorrowType, Word: BitBuffer + 'slice> BinarySlice<'slice, Ref, Word> {
    
    /// Reinterprets a slice of any kind as another.
    /// 
    /// # Safety
    /// 
    /// This is always safe to call for [`Mut`] -> [`Const`].
    /// 
    /// This is also always safe to call for `T` -> `T`.
    /// 
    /// For [`Const`] -> [`Mut`], the reference must be unique when returned to safe code.
    pub const unsafe fn as_this<R: BorrowType>(&self) -> &BinarySlice<'slice, R, Word> {
        unsafe { &*(self as *const BinarySlice<'slice, Ref, Word> as *const BinarySlice<'slice, R, Word>) }
    }

    /// Returns whether the start of the binary slice is aligned to a word.
    pub const fn start_aligned(&self) -> bool {
        self.start % Word::WIDTH == 0
    }

    /// Returns whether the end of the binary slice is aligned to a word.
    pub const fn end_aligned(&self) -> bool {
        (self.start + self.length) % Word::WIDTH == 0
    }

    /// Converts a slice of any kind to another.
    /// 
    /// # Safety
    /// 
    /// This is always safe to call for [`Mut`] -> [`Const`].
    /// 
    /// This is also always safe to call for `T` -> `T`.
    /// 
    /// For [`Const`] -> [`Mut`], the reference must be unique when returned to safe code.
    pub const unsafe fn into_this<R: BorrowType>(self) -> BinarySlice<'slice, R, Word> {
        unsafe {
            let (start, len, data) = self.into_raw_parts();
            BinarySlice::from_raw_parts(start, len, data)
        }
    }

    /// Converts the slice to a constant slice.
    pub const fn into_const(self) -> BinarySlice<'slice, Const, Word> {
        unsafe { self.into_this() }
    }

    /// Reinterprets the slice as a constant slice.
    pub const fn as_const(&self) -> &BinarySlice<'slice, Const, Word> {
        unsafe { self.as_this() }
    }
}

impl<'slice, Word: BitBuffer + 'slice> Deref for BinarySlice<'slice, Mut, Word>
{
    type Target = BinarySlice<'slice, Const, Word>;

    fn deref(&self) -> &Self::Target {
        self.as_const()
    }
}

impl<W: BitBuffer> Clone for BinarySlice<'_, Const, W> { fn clone(&self) -> Self { *self } }
impl<W: BitBuffer> Copy for BinarySlice<'_, Const, W> {}

impl<'slice, Ref: BorrowType, Word: BitBuffer + 'slice> BinarySlice<'slice, Ref, Word> {
    pub const EMPTY: Self = unsafe { 
        Self::from_raw_parts(0, 0, NonNull::dangling())
    };

    /// Decomposes a slice reference to a binary slice to its start, length, and data pointer.
    pub const fn into_raw_parts(self) -> (usize, usize, NonNull<Word>) {
        (self.start, self.length, self.data)
    }

    const fn get_word_ptr(&self, bit_index: usize) -> Option<(NonNull<Word>, usize)> {
        if bit_index >= self.length { return None; }
        let word_offset = self.start / Word::WIDTH;
        Some(unsafe { (self.data.add(word_offset), bit_index % Word::WIDTH) })
    }

    /// Constructs a reference to a binary slice from a start, length, and data pointer.
    /// 
    /// # Safety
    /// The reference must be valid up until `ceil(length / Word::WIDTH)` words and live as long as `'slice``.
    pub const unsafe fn from_raw_parts(start: usize, length: usize, data: NonNull<Word>) -> Self {
        Self { length, start, data, _this_struct_is_unsafe_to_brace_init: PhantomData }
    }

    /// Returns the bit length of the array.
    pub const fn len(&self) -> usize {
        self.length
    }

    /// Returns the given bit in the array, or None if it doesn't exist.
    pub fn get(&self, index: usize) -> Option<bool> {
        let (word, bit) = self.get_word_ptr(index)?;
        Some( unsafe { word.as_ref().get_bit(bit) } )
    }

    /// Returns the given bit in the array, assuming it's in bounds.
    /// 
    /// # Safety
    /// The index must be in bounds.
    pub unsafe fn get_unchecked(&self, index: usize) -> bool {
        unsafe { self.get(index).unwrap_unchecked() }
    }

    fn get_sub_bounds(&self, bounds: impl RangeBounds<usize>) -> Option<(usize, usize)> {
        let start = self.start + match bounds.start_bound() {
            core::ops::Bound::Unbounded => 0,
            core::ops::Bound::Included(i) => *i,
            core::ops::Bound::Excluded(&usize::MAX) => return None,
            core::ops::Bound::Excluded(e) => *e+1,
        };
        if start >= self.len() { return None; }
        let length: usize = match bounds.end_bound() {
            core::ops::Bound::Included(&usize::MAX) |
            core::ops::Bound::Unbounded => usize::MAX,
            core::ops::Bound::Included(i) => *i+1,
            core::ops::Bound::Excluded(0) => return None,
            core::ops::Bound::Excluded(e) => *e,
        }.min(self.length);
        Some((start, length))
    }

    /// Slices the array, returning a subsection of it. Returns None if the slice is out of bounds or inverted.
    /// 
    /// # Example
    /// ```rust
    /// # use binvec::prelude::*;
    /// let s = binslice![1 1 1 0 0 0];
    /// eprintln!("{s} ");
    /// assert!(s[0]);
    /// ```
    pub fn sub(self, bounds: impl RangeBounds<usize>) -> Option<Self> {
        let (start, length) = self.get_sub_bounds(bounds)?;
        Some(unsafe { Self::from_raw_parts(start, length, self.data) })
    }
}

impl<'slice, Ref: BorrowType, Word: BitBuffer> Index<usize> for BinarySlice<'slice, Ref, Word> {
    type Output = bool;

    fn index(&self, index: usize) -> &Self::Output {
        let Some(v) = self.get(index) else {
            panic!("index out of bounds for binary array")
        };
        if v {&true} else {&false}
    }
}

impl<'slice, Word: BitBuffer + 'slice> BinarySlice<'slice, Const, Word> {
    /// Constructs a bitslice from a slice of binary words and a length.
    /// 
    /// Note that bits are counted from the first bit up, so a slice
    /// of length 8 made from `0b00000001` will have a `1` at index 7, not index 0.
    /// 
    /// Will return None if the length is greater than the length of the slice.
    /// 
    /// # Example
    /// 
    /// ```rust
    /// # use binvec::borrow::{BinarySlice, Const, Mut};
    /// static BITS: BinarySlice<'static, Const, u8> = 
    ///     BinarySlice::<Const, u8>::from_word_slice(&[0b01010101u8], 8).unwrap();
    /// 
    /// assert!(BITS[0]);
    /// assert!(!BITS[1]);
    /// assert!(BITS.get(9).is_none());
    /// ```
    // TODO: Add start: usize parameter
    pub const fn from_word_slice(slice: &'slice [Word], length: usize) -> Option<Self> {
        let mut len = slice.len() * Word::WIDTH;
        if len < length { return None; }
        if len > length { len = length; }
        Some(unsafe { BinarySlice::from_raw_parts(
            0,
            len,
            NonNull::new(slice.as_ptr() as *mut _).expect("slice reference should not be null")
        )})
    }

    /// Deconstructs the slice reference into its underlying word buffer, and a start and length index into that buffer.
    /// 
    /// Note that this will contain bits not within the slice if the start and end are not aligned -
    /// this can be checked with [`Self::start_aligned`] and [`Self::end_aligned`] respectively.
    pub const fn into_word_slice(self) -> (&'slice [Word], usize, usize) {
        let word_start = self.start / Word::WIDTH;
        let word_end = (self.start + self.len()).div_ceil(Word::WIDTH);
        let word_len = word_end - word_start;
        let bit_start = self.start % Word::WIDTH;
        let bit_length = (self.start + self.length) - word_start * Word::WIDTH;
        (unsafe {
            core::slice::from_raw_parts(
                self.data.add(word_start).as_ptr(),
                word_len
            )
        }, bit_start, bit_length)
    }
}

impl<'slice, Ref: BorrowType, Word: SharedBitBuffer + 'slice> BinarySlice<'slice, Ref, Word> {
    fn _set_shared(&self, index: usize, value: bool) -> Option<bool> {
        let (word, bit) = self.get_word_ptr(index)?;
        unsafe { 
            let old_value = word.as_ref().get_bit(bit);
            word.as_ref().set_shared_bit(bit, value);
            Some(old_value)
        }
    }
}

impl<'slice, Word: BitBuffer + 'slice> BinarySlice<'slice, Mut, Word> {
    fn _set(&self, index: usize, value: bool) -> Option<bool> {
        let (mut word, bit) = self.get_word_ptr(index)?;
        unsafe { 
            let old_value = word.as_ref().get_bit(bit);
            word.as_mut().set_bit(bit, value);
            Some(old_value)
        }
    }

    /// Constructs a bitslice from a slice of binary words and a length.
    /// 
    /// Note that bits are counted from the first bit up, so a slice
    /// of length 8 made from `0b00000001` will have a `1` at index 7, not index 0.
    /// 
    /// Will return None if the length is greater than the length of the slice.
    /// 
    /// # Example
    /// 
    /// ```rust
    /// # use binvec::borrow::{BinarySlice, Const};
    /// static BITS: BinarySlice<'static, Const, u8> = 
    ///     BinarySlice::<Const, u8>::from_word_slice(&[1], 2).unwrap();
    /// 
    /// assert!(BITS[0]);
    /// assert!(!BITS[1]);
    /// assert!(BITS.get(9).is_none());
    /// ```
    pub const fn from_word_slice(slice: &'slice mut [Word], length: usize) -> Option<Self> {
        let mut len = slice.len() * Word::WIDTH;
        if len < length { return None; }
        if len > length { len = length; }
        Some(unsafe { BinarySlice::from_raw_parts(
            0,
            len,
            NonNull::new(slice.as_ptr() as *mut _).expect("slice reference should not be null")
        )})
    }

    /// Deconstructs the slice reference into its underlying word buffer.
    /// 
    /// Note that this will contain bits not within the slice if the start and end are not aligned -
    /// this can be checked with [`Self::start_aligned`] and [`Self::end_aligned`] respectively.
    pub const fn into_word_slice(self) -> &'slice mut [Word] {
        let word_start = self.start / Word::WIDTH;
        let word_end = (self.start + self.len()).div_ceil(Word::WIDTH);
        let word_len = word_end - word_start;
        unsafe {
            core::slice::from_raw_parts_mut(
                self.data.add(word_start).as_ptr(),
                word_len
            )
        }
    }
}

pub trait MutableBinSlice<'slice, Word: BitBuffer + 'slice>: seal::MutSeal {
    /// Sets a bit in the slice to a value.
    /// Returns the old value, or does nothing and returns `None` if the value was outside of the slice.
    fn set(&self, i: usize, v: bool) -> Option<bool>;

    /// Sets a bit in the slice to a value, assuming it's in bounds.
    /// 
    /// # Safety
    /// The index must be in bounds for the slice.
    unsafe fn set_unchecked(&self, i: usize, v: bool) -> bool {
        unsafe { self.set(i, v).unwrap_unchecked() }
    }
}

impl<'slice, Word: SharedBitBuffer> MutableBinSlice<'slice, Word> for BinarySlice<'slice, Const, Word> {
    fn set(&self, i: usize, v: bool) -> Option<bool> { self._set_shared(i, v) }
}

impl<'slice, Word: BitBuffer> MutableBinSlice<'slice, Word> for BinarySlice<'slice, Mut, Word> {
    fn set(&self, i: usize, v: bool) -> Option<bool> { self._set(i, v) }
}

impl<'slice, Ref: BorrowType, Word: BitBuffer> IntoIterator for BinarySlice<'slice, Ref, Word> {
    type Item = bool;

    type IntoIter = BinarySliceIter<'slice, Ref, Word>;

    fn into_iter(self) -> Self::IntoIter {
        let start = self.start;
        let len = self.length;
        BinarySliceIter {
            slice: self, front_index: start, back_index: len
        }
    }
}

/// Iterator over a binary slice.
/// 
/// Note that this returns booleans.
pub struct BinarySliceIter<'slice, Ref: BorrowType, Word: BitBuffer> {
    slice: BinarySlice<'slice, Ref, Word>, front_index: usize, back_index: usize
}

impl<'slice, Ref: BorrowType, Word: BitBuffer> Iterator for BinarySliceIter<'slice, Ref, Word> {
    type Item = bool;

    fn next(&mut self) -> Option<Self::Item> {
        if self.front_index >= self.back_index { return None; }
        let value = self.slice.as_const().get(self.front_index)?;
        self.front_index += 1;
        Some(value)
    }
}


impl<'slice, Ref: BorrowType, Word: BitBuffer> Debug for BinarySlice<'slice, Ref, Word> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        Display::fmt(self, f)
    }
}

impl<'slice, Ref: BorrowType, Word: BitBuffer> Display for BinarySlice<'slice, Ref, Word> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "[")?;
        for (i, b) in self.as_const().into_iter().enumerate() {
            if f.alternate() && i % Word::WIDTH == 0 { write!(f, "\n\t")?; }
            write!(f, "{}", b as u8)?;
        }
        if f.alternate() { write!(f, "\n")?; }
        write!(f, "]")
    }
}

unsafe impl<'slice, Word: BitBuffer> Send for BinarySlice<'slice, Const, Word> {}
unsafe impl<'slice, Word: BitBuffer> Sync for BinarySlice<'slice, Const, Word> {}

unsafe impl<'slice, Word: SharedBitBuffer> Send for BinarySlice<'slice, Mut, Word> {}
unsafe impl<'slice, Word: SharedBitBuffer> Sync for BinarySlice<'slice, Mut, Word> {}


#[diagnostic::on_unimplemented(
    label = "if you're trying to set a value within a binary slice, use `BinarySlice::set`",
)]
trait _NoIndexMut {}

#[diagnostic::do_not_recommend]
impl<'s, R, B> IndexMut<usize> for BinarySlice<'s, R, B> where for<'__> Self : _NoIndexMut, R: BorrowType, B: BitBuffer
{ fn index_mut(&mut self, _index: usize) -> &mut Self::Output { unimplemented!() } }

/// Constructs a `'static` binary slice.
/// 
/// Due to macro limitations, this always uses a [`u8`] as its underlying type,
/// and each 0 and 1 must be space-separated. Commas will be ignored.
/// 
/// # Example
/// 
/// ```rust
/// # use binvec::binslice;
/// 
/// assert_eq!(
///     (binslice![ 1 1 1 0 0 1 1 0, 1 1 0 ].into_word_slice().0),
///     [0b01100111u8, 0b011]
/// )
/// ```
#[macro_export]
macro_rules! binslice {
    [] => { $crate::borrow::BinarySlice::EMPTY };
    [$($tt: tt)+] => { $crate::_binslice!(@construct [$($tt)+] ) };
}

#[macro_export]
#[doc(hidden)]
macro_rules! _binslice {
    [@count $tt: tt $($tts: tt)*] => {
        1 + $crate::_binslice!(@count $($tts)*)
    };
    [@count] => {
        0
    };
    [@construct [0 $($tt: tt)*] $($bits: literal)*] => {
        $crate::_binslice!(@construct [$($tt)*] $($bits)* 0)
    };
    [@construct [1 $($tt: tt)*] $($bits: literal)*] => {
        $crate::_binslice!(@construct [$($tt)*] $($bits)* 1)
    };
    [@construct [, $($tt: tt)*] $($bits: literal)*] => {
        $crate::_binslice!(@construct [$($tt)*] $($bits)*)
    };
    [@construct [] $($rest: literal)*] => {
        $crate::borrow::BinarySlice::<'static, $crate::borrow::Const, u8>::from_word_slice(
            $crate::_binslice!(@list $crate::_binslice!(@create $($rest)* | | )),
            $crate::_binslice!(@count $($rest)+ )
        ).unwrap()
    };
    [@list $expr: expr ] => {& $expr };
    [@create |  | $($num: expr)* ] => {[$($num),*]};
    [@create $a: literal $($rest: literal)* |  | $($num: expr)* ] => {
        $crate::_binslice!(@create $($rest)* | $a | $($num)* )
    };
    [@create | $a: literal | $($num: expr)* ] => {[
        $($num,)*
        $a as u8
    ]};
    [@create $b: literal $($rest: literal)* | $a: literal | $($num: expr)* ] => {
        $crate::_binslice!(@create $($rest)* | $a $b | $($num)* )
    };
    [@create | $a: literal $b: literal | $($num: expr)* ] => {[
        $($num,)*
        (($a as u8) | (($b as u8) << 1))
    ]};
    [@create $c: literal $($rest: literal)* | $a: literal $b: literal | $($num: expr)* ] => {
        $crate::_binslice!(@create $($rest)* | $a $b $c | $($num)* )
    };
    [@create | $a: literal $b: literal $c: literal | $($num: expr)* ] => {[
        $($num,)*
        (($a as u8) | (($b as u8) << 1) | (($c as u8) << 2))
    ]};
    [@create $d: literal $($rest: literal)* | $a: literal $b: literal $c: literal | $($num: expr)* ] => {
        $crate::_binslice!(@create $($rest)* | $a $b $c $d | $($num)* )
    };
    [@create | $a: literal $b: literal $c: literal $d: literal | $($num: expr)* ] => {[
        $($num,)*
        (($a as u8) | (($b as u8) << 1) | (($c as u8) << 2) | (($d as u8) << 3))
    ]};
    [@create $e: literal $($rest: literal)* | $a: literal $b: literal $c: literal $d: literal | $($num: expr)* ] => {
        $crate::_binslice!(@create $($rest)* | $a $b $c $d $e | $($num)* )
    };
    [@create | $a: literal $b: literal $c: literal $d: literal $e: literal | $($num: expr)* ] => {[
        $($num,)*
        (($a as u8) | (($b as u8) << 1) | (($c as u8) << 2) | (($d as u8) << 3) | (($e as u8) << 4))
    ]};
    [@create $f: literal $($rest: literal)* | $a: literal $b: literal $c: literal $d: literal $e: literal | $($num: expr)* ] => {
        $crate::_binslice!(@create $($rest)* | $a $b $c $d $e $f | $($num)* )
    };
    [@create | $a: literal $b: literal $c: literal $d: literal $e: literal $f: literal | $($num: expr)* ] => {[
        $($num,)*
        (($a as u8) | (($b as u8) << 1) | (($c as u8) << 2) | (($d as u8) << 3) | (($e as u8) << 4) | (($f as u8) << 5))
    ]};
    [@create $g: literal $($rest: literal)* | $a: literal $b: literal $c: literal $d: literal $e: literal $f: literal | $($num: expr)* ] => {
        $crate::_binslice!(@create $($rest)* | $a $b $c $d $e $f $g | $($num)* )
    };
    [@create | $a: literal $b: literal $c: literal $d: literal $e: literal $f: literal $g: literal | $($num: expr)* ] => {[
        $($num,)*
        (($a as u8) | (($b as u8) << 1) | (($c as u8) << 2) | (($d as u8) << 3) | (($e as u8) << 4) | (($f as u8) << 5) | (($g as u8) << 6))
    ]};
    [@create $h: literal $($rest: literal)* | $a: literal $b: literal $c: literal $d: literal $e: literal $f: literal $g: literal | $($num: expr)* ] => {
        $crate::_binslice!(
            @create $($rest)* |  | 
            ($a as u8 | (($b as u8) << 1) | (($c as u8) << 2) | (($d as u8) << 3) | (($e as u8) << 4) | (($f as u8) << 5) | (($g as u8) << 6) | (($h as u8) << 7))
            $($num)*
        )
    };
}