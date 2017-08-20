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
    fn func(&self, x: i32) -> i32;
}

#[use_mocks]
mod test_module1 {
    use super::TestTrait;

    #[test]
    fn test_per_argument_matcher() {
        let mock = new_mock!(TestTrait);

        given! {
            <mock as TestTrait>::func |_| true then_return 12 always;
        }

        assert_eq!(mock.func(1), 12);
    }
}

#[use_mocks]
mod test_module2 {
    use super::TestTrait;

    mod sub1 {
        mod sub2 {
            #[test]
            fn test_per_argument_matcher() {
                let mock = new_mock!(TestTrait);

                given! {
                    <mock as TestTrait>::func |_| true then_return 12 always;
                }

                assert_eq!(mock.func(1), 12);
            }
        }
    }
}
