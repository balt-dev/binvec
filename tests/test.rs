use std::time::Duration;

use binvec::vec::BinaryVec;


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