#![feature(proc_macro)]
extern crate galvanic_mock;
use galvanic_mock::{mockable, use_mocks};

mod sub1 {
    pub mod sub2 {
        pub trait EmptyTrait { }
    }
}

#[mockable(intern ::sub1::sub2)]
trait EmptyTrait { }

#[test]#[use_mocks]
fn create_mock_for_referred_trait() {
    let mock = new_mock!(::sub1::sub2::EmptyTrait);
}
