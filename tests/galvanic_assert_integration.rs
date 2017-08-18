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
#[allow(unused_imports)] use galvanic_mock::{mockable, use_mocks};
extern crate galvanic_assert;
#[allow(unused_imports)]  use galvanic_assert::matchers::*;

#[mockable]
trait TestTrait {
    fn func(&self, x: i32, y: f64) -> i32;
}

#[cfg(feature = "galvanic_assert_integration")]
#[test]
#[use_mocks]
fn test_per_argument_matcher_with_galvanic_assert_matchers() {
    let mock = new_mock!(TestTrait);

    given! {
        <mock as TestTrait>::func(lt(2), gt(3.3)) then_return 12 always;
    }

    assert_eq!(mock.func(1, 4.4), 12);
}

#[cfg(feature = "galvanic_assert_integration")]
#[test]
#[use_mocks]
fn test_explicit_matcher_with_galvanic_assert_matcher() {
    let mock = new_mock!(TestTrait);

    given! {
        <mock as TestTrait>::func eq((2, 3.3)) then_return 12 always;
    }

    assert_eq!(mock.func(2, 3.3), 12);
}
