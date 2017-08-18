/* Copyright 2017 Christopher Bacher
 *
 * Licensed under the Apache License, Version 2.0 (the "License");
 * you may not use this file except in compliance with the License.
 * You may obtain a copy of the License at
 *
 * http://www.apache.org/licenses/LICENSE-2.0
 *
 * Unless required by applicable law or agreed to in writing, software
 * distributed under the License is distributed on an "AS IS" BASIS,
 * WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
 * See the License for the specific language governing permissions and
 * limitations under the License.
 */
#![feature(proc_macro)]
extern crate galvanic_mock;
extern crate galvanic_assert;

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
