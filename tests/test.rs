
use std::cell::Cell;

use binvec::prelude::*;


#[test]
fn push_pop() {
    macro_rules! m {
        ($($ty: ty)*) => {$(
            println!("Testing for type {}", stringify!($ty));
            let mut vec = BinaryVec::<$ty>::new();
            for i in 0usize..200 {
                if i >= 100 {
                    assert_eq!(vec.pop().expect("vector should not be empty"), i % 2 == 0)
                } else {
                    vec.push(i % 2 == 1);
                }
                for j in 0..vec.as_slice().len() {
                    assert_eq!(vec.as_slice()[j], (j % 2 == 1));
                }
            }
        )*}
    }
    m!{u8 u16 u32 u64 u128 usize i8 i16 i32 i64 i128 isize [u8; 3] [u8; 5] [usize; 2]}
}

#[test]
fn test_display() {
    let buf = BinaryVec::<usize>::from_iter([true, false, true, true, true]);
}

#[test]
fn get_set_collect() {
    let mut buf = BinaryVec::from_iter([[const { Cell::new(0u8) }; 5]]);
    buf.as_mut_slice().set(4, true);
    assert!(buf.as_slice()[4]);

    let a = &buf;
    let b = &buf;
    a.as_slice().set(1, true);
    assert!(b.as_slice()[1]);
}

#[test]
fn slice() {
    const T: bool = true;
    const F: bool = false;
    let buf = BinaryVec::<Cell<u8>>::from_iter([T, F, T, F, F, F, T, T]);

    let a = buf.as_slice();
    let b = buf.as_slice();
    println!("{a} {b}");
    a.set(1, true);
    assert!(b[1]);
}