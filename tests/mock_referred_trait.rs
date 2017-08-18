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
