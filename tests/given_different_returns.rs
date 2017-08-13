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
use galvanic_mock::{mockable, use_mocks};

#[mockable]
trait TestTrait {
    fn func(&self, x: i32) -> i32;
}

#[test]
#[use_mocks]
fn test_then_return_from() {
    let mock = new_mock!(TestTrait);

    given! {
        <mock as TestTrait>::func |_| true then_return_from |&(a,)| a*2 always;
    }

    assert_eq!(mock.func(1), 2);
    assert_eq!(mock.func(2), 4);
}


#[test]
#[should_panic]
#[use_mocks]
fn test_then_panic() {
    let mock = new_mock!(TestTrait);

    given! {
        <mock as TestTrait>::func |&(a,)| a < 2 then_panic always;
    }

    mock.func(1);
}
