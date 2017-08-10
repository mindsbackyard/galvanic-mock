#![feature(proc_macro)]
extern crate galvanic_mock;
use galvanic_mock::{mockable, use_mocks};

#[mockable]
trait TestTrait {
    fn func(&self, x: i32) -> i32;
}

#[test]
#[use_mocks]
fn test_times_within_bound() {
    let mock = new_mock!(TestTrait);

    given! {
        <mock as TestTrait>::func |_| true then_return 12 times(2);
    }

    assert_eq!(mock.func(1), 12);
    assert_eq!(mock.func(2), 12);
}

#[test]
#[should_panic]
#[use_mocks]
fn test_times_out_of_bound() {
    let mock = new_mock!(TestTrait);

    given! {
        <mock as TestTrait>::func |_| true then_return 12 times(2);
    }

    assert_eq!(mock.func(1), 12);
    assert_eq!(mock.func(2), 12);
    assert_eq!(mock.func(3), 12);
}

#[test]
#[use_mocks]
fn test_times_in_sequence() {
    let mock = new_mock!(TestTrait);

    given! {
        <mock as TestTrait>::{
            func |_| true then_return 12 times(1);
            func |_| true then_return 24 times(1);
        }
    }

    assert_eq!(mock.func(1), 12);
    assert_eq!(mock.func(2), 24);
}

#[test]
#[use_mocks]
fn test_multiple_given_blocks_with_reset_of_given_behaviours() {
    let mut mock = new_mock!(TestTrait);

    given! {
        <mock as TestTrait>::func |_| true then_return 12 always;
    }
    assert_eq!(mock.func(1), 12);

    mock.reset_given_behaviours();

    given! {
        <mock as TestTrait>::func |_| true then_return 24 always;
    }
    assert_eq!(mock.func(2), 24);
}