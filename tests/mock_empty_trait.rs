#![feature(proc_macro)]
extern crate galvanic_mock;
use galvanic_mock::{mockable, use_mocks};


#[mockable]
trait EmptyTrait { }

#[test]#[use_mocks]
fn create_mock_with_empty_trait() {
    let mock = new_mock!(EmptyTrait);
}
