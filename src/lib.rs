#![no_std]
#![warn(clippy::pedantic, clippy::perf, clippy::stylea)]
extern crate alloc as _alloc;


pub mod borrow;
pub(crate) mod raw_vec;
pub mod vec;

pub mod prelude {
    pub use super::{
        vec::BinaryVec,
        borrow::{BinarySlice, Const, Mut, MutableBinSlice as _}
    };
}