use std::time::Duration;

use binvec::vec::BinaryVec;

#[test]
fn push_pop() {
    let mut vec = BinaryVec::<usize>::new();
    for i in 0usize..200 {
        if i >= 100 {
            assert_eq!(vec.pop().expect("vector should not be empty"), i % 2 == 0)
        } else {
            vec.push(i % 2 == 1);
        }
        for j in 0..vec.as_slice().len() {
            assert_eq!(vec.as_slice()[j], (j % 2 == 1));
        }
        std::thread::sleep(Duration::from_millis(10));
    }
}