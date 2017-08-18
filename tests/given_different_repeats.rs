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

#[test]
#[use_mocks]
fn test_times_within_bound() {
    let mock = new_mock!(TestTrait);

    given! {
        <mock as TestTrait>::func |_| true then_return 12 times(2);
    }

    assert_eq!(mock.func(1), 12);
    assert_eq!(mock.func(2), 12);
}

#[test]
#[should_panic]
#[use_mocks]
fn test_times_out_of_bound() {
    let mock = new_mock!(TestTrait);

    given! {
        <mock as TestTrait>::func |_| true then_return 12 times(2);
    }

    assert_eq!(mock.func(1), 12);
    assert_eq!(mock.func(2), 12);
    assert_eq!(mock.func(3), 12);
}

#[test]
#[use_mocks]
fn test_times_in_sequence() {
    let mock = new_mock!(TestTrait);

    given! {
        <mock as TestTrait>::{
            func |_| true then_return 12 times(1);
            func |_| true then_return 24 times(1);
        }
    }

    assert_eq!(mock.func(1), 12);
    assert_eq!(mock.func(2), 24);
}

#[test]
#[use_mocks]
fn test_multiple_given_blocks_with_reset_of_given_behaviours() {
    let mut mock = new_mock!(TestTrait);

    given! {
        <mock as TestTrait>::func |_| true then_return 12 always;
    }
    assert_eq!(mock.func(1), 12);

    mock.reset_given_behaviours();

    given! {
        <mock as TestTrait>::func |_| true then_return 24 always;
    }
    assert_eq!(mock.func(2), 24);
}
