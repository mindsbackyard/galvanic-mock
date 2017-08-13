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

mod test_times {
    use super::*;

    #[test]
    #[use_mocks]
    fn matching_expectation() {
        let mock = new_mock!(TestTrait);

        given! {
            <mock as TestTrait>::func |_| true then_return 12 always;
        }

        expect_interactions! {
            <mock as TestTrait>::func(|&a| a < 2) times(2);
        }

        mock.func(1);
        mock.func(1);
        mock.func(3);
        mock.func(3);

        mock.verify();
    }

    #[test]
    #[should_panic]
    #[use_mocks]
    fn violating_expectation_too_few() {
        let mock = new_mock!(TestTrait);

        given! {
            <mock as TestTrait>::func |_| true then_return 12 always;
        }

        expect_interactions! {
            <mock as TestTrait>::func(|&a| a < 2) times(2);
        }

        mock.func(1);
        mock.func(3);
        mock.func(3);

        mock.verify();
    }

    #[test]
    #[should_panic]
    #[use_mocks]
    fn violating_expectation_too_many() {
        let mock = new_mock!(TestTrait);

        given! {
            <mock as TestTrait>::func |_| true then_return 12 always;
        }

        expect_interactions! {
            <mock as TestTrait>::func(|&a| a < 2) times(2);
        }

        mock.func(1);
        mock.func(1);
        mock.func(1);
        mock.func(3);
        mock.func(3);

        mock.verify();
    }
}

mod test_at_most {
    use super::*;

    #[test]
    #[use_mocks]
    fn matching_expectation_exact() {
        let mock = new_mock!(TestTrait);

        given! {
            <mock as TestTrait>::func |_| true then_return 12 always;
        }

        expect_interactions! {
            <mock as TestTrait>::func(|&a| a < 2) at_most 2;
        }

        mock.func(1);
        mock.func(1);
        mock.func(3);
        mock.func(3);

        mock.verify();
    }

    #[test]
    #[use_mocks]
    fn matching_expectation_fewer() {
        let mock = new_mock!(TestTrait);

        given! {
            <mock as TestTrait>::func |_| true then_return 12 always;
        }

        expect_interactions! {
            <mock as TestTrait>::func(|&a| a < 2) at_most 2;
        }

        mock.func(1);
        mock.func(3);
        mock.func(3);

        mock.verify();
    }

    #[test]
    #[should_panic]
    #[use_mocks]
    fn violating_expectation_too_many() {
        let mock = new_mock!(TestTrait);

        given! {
            <mock as TestTrait>::func |_| true then_return 12 always;
        }

        expect_interactions! {
            <mock as TestTrait>::func(|&a| a < 2) at_most 2;
        }

        mock.func(1);
        mock.func(1);
        mock.func(1);
        mock.func(3);
        mock.func(3);

        mock.verify();
    }
}

mod test_at_least {
    use super::*;

    #[test]
    #[use_mocks]
    fn matching_expectation() {
        let mock = new_mock!(TestTrait);

        given! {
            <mock as TestTrait>::func |_| true then_return 12 always;
        }

        expect_interactions! {
            <mock as TestTrait>::func(|&a| a < 2) at_least 2;
        }

        mock.func(1);
        mock.func(1);
        mock.func(3);
        mock.func(3);

        mock.verify();
    }

    #[test]
    #[should_panic]
    #[use_mocks]
    fn violating_expectation_too_few() {
        let mock = new_mock!(TestTrait);

        given! {
            <mock as TestTrait>::func |_| true then_return 12 always;
        }

        expect_interactions! {
            <mock as TestTrait>::func(|&a| a < 2) at_least 2;
        }

        mock.func(1);
        mock.func(3);
        mock.func(3);

        mock.verify();
    }

    #[test]
    #[use_mocks]
    fn matching_expectation_more() {
        let mock = new_mock!(TestTrait);

        given! {
            <mock as TestTrait>::func |_| true then_return 12 always;
        }

        expect_interactions! {
            <mock as TestTrait>::func(|&a| a < 2) at_least 2;
        }

        mock.func(1);
        mock.func(1);
        mock.func(1);
        mock.func(3);
        mock.func(3);

        mock.verify();
    }
}

mod test_between {
    use super::*;

    #[test]
    #[use_mocks]
    fn matching_expectation_exact_lower_bound() {
        let mock = new_mock!(TestTrait);

        given! {
            <mock as TestTrait>::func |_| true then_return 12 always;
        }

        expect_interactions! {
            <mock as TestTrait>::func(|&a| a < 2) between 2,4;
        }

        mock.func(1);
        mock.func(1);
        mock.func(3);
        mock.func(3);

        mock.verify();
    }

    #[test]
    #[use_mocks]
    fn matching_expectation_between_bounds() {
        let mock = new_mock!(TestTrait);

        given! {
            <mock as TestTrait>::func |_| true then_return 12 always;
        }

        expect_interactions! {
            <mock as TestTrait>::func(|&a| a < 2) between 2,4;
        }

        mock.func(1);
        mock.func(1);
        mock.func(3);
        mock.func(3);
        mock.func(1);

        mock.verify();
    }

    #[test]
    #[use_mocks]
    fn matching_expectation_exact_upper_bound() {
        let mock = new_mock!(TestTrait);

        given! {
            <mock as TestTrait>::func |_| true then_return 12 always;
        }

        expect_interactions! {
            <mock as TestTrait>::func(|&a| a < 2) between 2,4;
        }

        mock.func(1);
        mock.func(1);
        mock.func(3);
        mock.func(3);
        mock.func(1);
        mock.func(1);

        mock.verify();
    }

    #[test]
    #[should_panic]
    #[use_mocks]
    fn violating_expectation_too_few() {
        let mock = new_mock!(TestTrait);

        given! {
            <mock as TestTrait>::func |_| true then_return 12 always;
        }

        expect_interactions! {
            <mock as TestTrait>::func(|&a| a < 2) times(2);
        }

        mock.func(1);
        mock.func(3);
        mock.func(3);

        mock.verify();
    }

    #[test]
    #[should_panic]
    #[use_mocks]
    fn violating_expectation_too_many() {
        let mock = new_mock!(TestTrait);

        given! {
            <mock as TestTrait>::func |_| true then_return 12 always;
        }

        expect_interactions! {
            <mock as TestTrait>::func(|&a| a < 2) times(2);
        }

        mock.func(1);
        mock.func(1);
        mock.func(1);
        mock.func(1);
        mock.func(1);
        mock.func(3);
        mock.func(3);

        mock.verify();
    }
}

mod test_never {
    use super::*;

    #[test]
    #[use_mocks]
    fn matching_expectation() {
        let mock = new_mock!(TestTrait);

        given! {
            <mock as TestTrait>::func |_| true then_return 12 always;
        }

        expect_interactions! {
            <mock as TestTrait>::func(|&a| a < 2) never;
        }

        mock.func(3);
        mock.func(3);

        mock.verify();
    }

    #[test]
    #[should_panic]
    #[use_mocks]
    fn violating_expectation() {
        let mock = new_mock!(TestTrait);

        given! {
            <mock as TestTrait>::func |_| true then_return 12 always;
        }

        expect_interactions! {
            <mock as TestTrait>::func(|&a| a < 2) never;
        }

        mock.func(1);
        mock.func(3);
        mock.func(3);

        mock.verify();
    }
}


mod multiple_traits {
    use super::*;

    #[mockable(::multiple_traits)]
    trait OtherTrait {
        fn other_func(&self) -> i32;
    }

    #[test]
    #[use_mocks]
    fn matching_expectation() {
        let mock = new_mock!(TestTrait, ::multiple_traits::OtherTrait);

        given! {
            <mock as TestTrait>::func |_| true then_return 12 always;
            <mock as ::multiple_traits::OtherTrait>::other_func |_| true then_return 24 always;
        }

        expect_interactions! {
            <mock as TestTrait>::func(|&a| a < 2) never;
            <mock as ::multiple_traits::OtherTrait>::other_func |_| true never;
        }

        mock.func(3);
        mock.func(3);

        mock.verify();
    }
}
