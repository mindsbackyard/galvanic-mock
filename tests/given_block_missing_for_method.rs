#![feature(proc_macro)]
extern crate galvanic_mock;
use galvanic_mock::{mockable, use_mocks};

#[mockable]
trait TestTrait {
    fn func(&self, x: i32) -> i32;
}

#[test]
#[should_panic]
#[use_mocks]
fn test_per_argument_matcher() {
    let mock = new_mock!(TestTrait);
    mock.func(1);
}
