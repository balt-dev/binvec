#![no_std]
#![warn(clippy::pedantic, clippy::perf, clippy::stylea)]

#[cfg(feature = "alloc")]
extern crate alloc as _alloc;


pub mod borrow;

#[cfg(feature = "alloc")]
pub mod vec;

pub mod prelude {
    pub use super::borrow::{BinarySlice, Const, Mut, MutableBinSlice as _};
    #[cfg(feature = "alloc")]
    pub use super::vec::BinaryVec;
    pub use super::{binslice, binvec};
}