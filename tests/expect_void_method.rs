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

#[mockable]
trait TestTrait {
    fn func(&self, x: i32);
}

mod test_times {
    use super::*;

    #[test]
    #[use_mocks]
    fn matching_expectation() {
        let mock = new_mock!(TestTrait);

        given! {
            <mock as TestTrait>::func(|_| true) then_return ()  always;
        }

        expect_interactions! {
            <mock as TestTrait>::func(|&a| a < 2) times(1);
        }

        mock.func(1);

        mock.verify();
    }
}
