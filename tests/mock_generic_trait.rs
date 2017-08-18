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
pub trait TestTrait<'a, T,F> {
    type Assoc;
    fn func(&self, x: T, y: &F) -> i32;
}

#[test]#[use_mocks]
fn test() {
    let x = new_mock!(TestTrait<i32, f64, Assoc=String>);

    given! {
        bind y: i32 = 12;
        <x as TestTrait<i32, f64, Assoc=String>>::func(|&a| a < 2, |&&b| b < 2.2) then_return 23 times(1);
        <x as TestTrait<i32, f64, Assoc=String>>::func(|&a| a < 4, |&&b| b < 2.2) then_return_from |_| bound.y * 2 always;
    }

    expect_interactions! {
        <x as TestTrait<i32, f64, Assoc=String>>::func(|&a| a < 2, |&&b| b < 2.2) at_least 1;
        <x as TestTrait<i32, f64, Assoc=String>>::func(|&a| a > 2, |&&b| true) between 2,3;
    }

    assert!(x.func(1, &1.1) == 23);
    assert!(x.func(3, &1.1) == 24);
    assert!(x.func(3, &1.1) == 24);

    x.verify();
}
