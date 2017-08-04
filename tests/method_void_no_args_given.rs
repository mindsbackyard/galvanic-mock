#![feature(proc_macro)]
extern crate galvanic_mock;
use galvanic_mock::{mockable, use_mocks};

#[mockable]
trait TestTrait {
    fn func(&self);
}

#[test]
#[use_mocks]
fn test() {
    let mock = new_mock!(TestTrait);

    given! {
        <mock as TestTrait>::func || true then_return () always;
    }

    mock.func();
}
