pub use super::*;

#[test]
fn test() {
    assert!(std::hint::black_box(2) == 2);
}
