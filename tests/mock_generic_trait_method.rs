#![feature(proc_macro)]
extern crate galvanic_mock;
use galvanic_mock::{mockable, use_mocks};

#[mockable]
pub trait TestTrait {
    fn func<T: PartialEq + PartialOrd>(&self, x: T, y: T) -> i32;
}

#[test]#[use_mocks]
fn test() {
    let x = new_mock!(TestTrait);

    given! {
        <x as TestTrait>::func |&(ref a, ref b)| a == b then_return 23 always;
        <x as TestTrait>::func |&(ref a, ref b)| true then_return 46 always;
    }

    expect_interactions! {
        <x as TestTrait>::func |&(ref a, ref b)| a == b times 2;
        <x as TestTrait>::func |&(ref a, ref b)| a > b times 2;
        <x as TestTrait>::func |&(ref a, ref b)| a < b never;
    }

    assert!(x.func(1, 1) == 23);
    assert!(x.func(3, 2) == 46);
    assert!(x.func(3, 3) == 23);
    assert!(x.func(3, 2) == 46);

    x.verify();
}
