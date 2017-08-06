#![feature(proc_macro)]
extern crate galvanic_mock;
use galvanic_mock::{mockable, use_mocks};

#[mockable]
trait TestTrait {
    fn func(&self, x: i32) -> i32;
}

#[test]
#[use_mocks]
fn test_then_return_from() {
    let mock = new_mock!(TestTrait);

    given! {
        <mock as TestTrait>::func |_| true then_return_from |&(a,)| a*2 always;
    }

    assert_eq!(mock.func(1), 2);
    assert_eq!(mock.func(2), 4);
}


#[test]
#[should_panic]
#[use_mocks]
fn test_then_panic() {
    let mock = new_mock!(TestTrait);

    given! {
        <mock as TestTrait>::func |&(a,)| a < 2 then_panic always;
    }

    mock.func(1);
}
