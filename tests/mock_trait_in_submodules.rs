#![feature(proc_macro)]
extern crate galvanic_mock;

mod sub1 {
    pub mod sub2 {
        use galvanic_mock::mockable;
        // this macro must be expanded first
        #[mockable(::sub1::sub2)]
        pub trait EmptyTrait { }
    }
}


mod test1 {
    use galvanic_mock::use_mocks;
    #[test]#[use_mocks]
    fn create_mock_from_other_submodule() {
        let mock = new_mock!(::sub1::sub2::EmptyTrait);
    }
}
