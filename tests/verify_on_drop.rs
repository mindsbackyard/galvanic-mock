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
fn verify_on_drop() {
    let mock = new_mock!(TestTrait);

    given! {
        <mock as TestTrait>::func |_| true then_return 12 always;
    }

    expect_interactions! {
        <mock as TestTrait>::func(|&a| a < 2) times(2);
    }

    mock.func(1);
}

#[test]
#[use_mocks]
fn disable_verify_on_drop() {
    let mut mock = new_mock!(TestTrait);

    given! {
        <mock as TestTrait>::func |_| true then_return 12 always;
    }

    expect_interactions! {
        <mock as TestTrait>::func(|&a| a < 2) times(2);
    }

    mock.should_verify_on_drop(false);
    mock.func(1);
}
