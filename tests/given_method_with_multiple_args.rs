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
    fn func(&self, x: i32, y: f64) -> i32;
}

#[test]
#[use_mocks]
fn test_per_argument_matcher() {
    let mock = new_mock!(TestTrait);

    given! {
        <mock as TestTrait>::func(|&a| a < 2, |&b| b > 3.3) then_return 12 always;
        <mock as TestTrait>::func(|&a| a >= 2, |&b| b > 3.3) then_return 24 always;
    }

    assert_eq!(mock.func(1, 4.4), 12);
    assert_eq!(mock.func(1, 4.4), 12);
    assert_eq!(mock.func(3, 4.4), 24);
    assert_eq!(mock.func(3, 4.4), 24);
}

#[test]
#[use_mocks]
fn test_explicit_matcher() {
    let mock = new_mock!(TestTrait);

    given! {
        <mock as TestTrait>::func |&(a,b)| a < 2 then_return 12 always;
        <mock as TestTrait>::func |&(a,b)| a >= 2 then_return 24 always;
    }

    assert_eq!(mock.func(1, 4.4), 12);
    assert_eq!(mock.func(1, 4.4), 12);
    assert_eq!(mock.func(3, 4.4), 24);
    assert_eq!(mock.func(3, 4.4), 24);
}
