# Galvanic-mock: behaviour-driven mocking for generic traits
[![Build Status](https://travis-ci.org/mindsbackyard/galvanic-mock.svg?branch=master)](https://travis-ci.org/mindsbackyard/galvanic-mock)
[![Crates.io](https://img.shields.io/crates/v/galvanic-mock.svg)](https://crates.io/crates/galvanic-mock)

This crate provides procedural macros (`#[mockable]`, `#[use_mocks]`) for mocking the behaviour of traits.

 * define given **behaviours** for mock objects based on **patterns**
 * state **expectations** for interactions with mocks
 * mock **multiple** traits at once
 * mock **generic traits** and **traits with associated types**
 * mock **generic trait methods**
 * apply **#[derive(..)]** and other attributes to your mocks
 * **[galvanic-assert](https://www.github.com/mindsbackyard/galvanic-assert)** matchers like `eq`, `lt`, ... can be used in behaviours
 * integrate with **[galvanic-test](https://www.github.com/mindsbackyard/galvanic-test)** and **[galvanic-assert](https://www.github.com/mindsbackyard/galvanic-assert)**
 * be used with your favourite test framework

 The crate is part of **galvanic**---a complete test framework for **Rust**.
 The framework is shipped in three parts, so you can choose to use only the parts you need.

## A short introduction to galvanic-mock

In a well designed software project with loose coupling and dependency injection,
a mock eases the development of software tests. It imitates the behaviour of a *real* object,
i.e., an object present in the production code, to decouple a tested software component from the rest of the system.

**Galvanic-mock** is a behaviour-driven mocking library for traits in Rust.
It allows the user to create a mock object for one or multiple traits emulating their behaviour according to **given** patterns of interaction.
A pattern for a trait's method consists of a boolean matcher for its argument (either for each argument or for all at once), a constant or function calculating the return value, and a the number of repetitions for which the pattern is valid.

```rust
// this crate requires a nightly version of rust
#![feature(proc_macro)]
extern crate galvanic_mock;
use galvanic_mock::{mockable, use_mocks};

#[mockable]
trait MyTrait {
    fn foo(&self, x: i32, y: i32) -> i32;
}

#[test]
#[use_mocks]
fn simple_usage_of_mocks() {
    // create a new object implementing `MyTrait`
    let mock = new_mock!(MyTrait);
    let some_calculation = 1 + 2*3;

    // define behaviours how your mocks should react given some input
    given! {
        // make val available to your behaviours (must implement `Clone`), the type is **not** optional!
        bind val: i32 = some_calculation;

        // define input matchers per argument and return a constant value whenever it matches
        <mock as MyTrait>::foo(|&x| x < 7, |&y| y % 2 == 0) then_return 12 always;
        // or define a single input matcher for all arguments and return the result of a function
        <mock as MyTrait>::foo |&(x, y)| x < y then_return_from |&(x,y)| y - x always;
        // with the `bound` variable you can access variable declared with `bind VAR: TYPE = VALUE;`
        <mock as MyTrait>::foo(|_| true) then_return_from |&(x,_)| x*bound.val always;
    }

    // only matches the last behaviour
    assert_eq!(mock.foo(12, 4), 84);
    // would match the first and the second behaviour, but the first matching behaviour is always used
    assert_eq!(mock.foo(3, 4), 12);
    // matches the second behaviour
    assert_eq!(mock.foo(12, 14), 2);
}
```

Besides emulating the behaviour of an object it is also possible to state expectations about the interactions with object.
Patterns for **expected** behaviours work similar to pattterns for given behaviours.
The example below illustrates these concepts.

```rust
#![feature(proc_macro)]
extern crate galvanic_mock;
use galvanic_mock::{mockable, use_mocks};

// matchers from galvanic_assert can be used as argument matchers
extern crate galvanic_assert;
use galvanic_assert::matchers::{gt, leq, any_value};

#[mockable]
trait MyTrait {
    fn foo(&self, x: i32, y: i32) -> i32;
}

#[mockable]
trait MyOtherTrait<T> {
    fn bar(&self, x: T) -> T;
}

#[test]
#[use_mocks]
fn simple_use_of_mocks() {
    // to mock multiple traits just separate them with a colon
    // specify all types for generic traits as you would specify a type
    let mock = new_mock!(MyTrait, MyOtherTrait<String>);

    // expectations are matched top-down, but once the specified match count is reached it won't match again
    given! {
        // instead of repeating the trait over and over, you can open a block
        <mock as MyTrait>::{
            // this behaviour will match only twice
            foo any_value() then_return_from |_| 7 times 2;
            foo(gt(12), any_value()) then_return 2 always;
        };
        // for generic traits all generic types and associated types need to be given
        <mock as MyOtherTrait<String>>::bar(|x| x == "hugo") then_return "got hugo".to_string() always;
    }

    // expectations are matched top-down, but will never be exhausted
    expect_interactions! {
        // `times` expects an exact number of matching interactions
        <mock as MyOtherTrait<String>>::bar any_value() times 1;
        // besides `times`, also `at_least`, `at_most`, `between`, and `never` are supported
        // all limits are inclusive
        <mock as MyTrait>::foo(any_value(), leq(2)) between 2,5;
    }

    assert_eq!(mock.foo(15, 1), 7);
    assert_eq!(mock.bar("hugo".to_string()), "got hugo".to_string());
    assert_eq!(mock.foo(15, 2), 7);
    assert_eq!(mock.foo(15, 5), 2);

    // the expected interactions are verified when the mock is dropped or when `mock.verify()` is called
}
```


## Documentation

Before reading the documentation make sure to read the examples in the introduction as the documentation will use them as a basis for explanation.

To use the mocking library make sure that you use a **nightly** version of Rust as the crate requires the `proc_macro` feature.
Add the dependency to your `Cargo.toml` preferably as a dev dependency.
```toml
[dev-dependencies]
galvanic-mock = "*" # galvanic uses `semver` versioning
```

At the root of your crate (either `main.rs` or `lib.rs`) add the following to activate the required features and to import the macros.
```rust
#![feature(proc_macro)]
extern crate galvanic_mock;
// The use statement should be placed where the #[mocakable] and #[use_mocks] attributes
// are actually used, or reimported.
use galvanic_mock::{mockable, use_mocks};
```

If we want to use `galvanic-assert` matchers in mocks then we have to enable the `galvanic_assert_integration` feature as follows.
```toml
[dev-dependencies]
galvanic-mock = { version = "*", features = ["galvanic_assert_integration"] }
galvanic-assert = "*" # galvanic-assert uses semver versioning too. To find the version required by `galvanic-mock` check version of the optional dependency in the manifest `Cargo.toml`.
```
If the integration feature is enabled, `extern crate galvanic_assert` has to be specified along with `extern crate galvanic_mock` or the library will fail to compile (even if no `galvanic_assert` matchers are used).

### Defining mockable traits with `#[mockable]`

Before a trait can be mocked, you have to tell the mocking framework about its name, generics, associated types, and methods.
If the trait is part of your own crate you just apply the `#[mockable]` attribute to the trait definition.
```Rust
#[mockable]
trait MyTrait {
    fn foo(&self, x: i32, y: i32) -> i32;
}
```
This registers `MyTrait` as mockable.
Further it assumes that `MyTrait` is defined at the top-level of a crate or that it is always imported by name when mocked, e.g., with `use crate::module::MyTrait`.

If the trait is defined in a submodule, its path should be provided to the attribute.
```Rust
mod sub {
    #[mockable(::sub)]
    trait MyTrait {
        fn foo(&self, x: i32, y: i32) -> i32;
    }
}
```

However the trait is annotated, this will be the only way to refer to it later.
There is no name resolution built in, e.g., the above trait must always be used as `::sub::MyTrait`.
The user of the mocked trait is responsible that the trait is visible to the location where the mock is used under the provided path.
It is therefore recommended that *global* paths are used as in the example above.

#### Mocking *external* traits

An external trait can be mocked by prefixing the path in the attribute with the `extern` keyword.
The full trait definition must be restated, though its definition will be omitted the macro's expansion.
```Rust
#[mockable(extern some_crate::sub)]
trait MyTrait {
    fn foo(&self, x: i32, y: i32) -> i32;
}
```

#### Fixing issues with the macro expansion order

As any other macro, `#[mockable]` is subject to the macro expansion order.
Further a mockable trait must be defined before it can be used.
If this is an issue for an internal trait, its definition can be restated similar to *external* traits.
```Rust
// this occurance of the trait declaration will be removed
#[mockable(intern ::sub)]
trait MyTrait {
    fn foo(&self, x: i32, y: i32) -> i32;
}

// a mock is created somewhere here
...

// the true declaration is encountered later
mod sub {
    trait MyTrait {
        fn foo(&self, x: i32, y: i32) -> i32;
    }
}
```

### Declaring mock usage with `#[use_mocks]`

Any location (`fn`, `mod`) where mocks should be use must be annotated with `#[use_mocks]`.
```Rust
#[test]
#[use_mocks]
fn some_test {
    ...
}
```

If `#[use_mocks]` is applied to a module then the mock types are shared within all submodules and functions.
```Rust
#[use_mocks]
mod test_module {
    #[test]
    fn some_test {
        ...
    }

    #[test]
    fn some_other_test {
        ...
    }
}
```
Though never apply `#[use_mocks]` to an item within some other item which has already a `#[use_mocks]` attribute.

The following macros can only be used within locations annotated with `#[use_mocks]`.

### Creating new mocks with `new_mock!`

To create a new mock object use the `new_mock!` macro followed by a list of mocked traits.
For generic traits specify all their type arguments and associated types.
The created object satisfies the stated trait bounds and may also be converted into a boxed trait object.
```Rust
#[use_mocks]
fn some_test {
    let mock = new_mock!(MyTrait, MyOtherTrait<i32, f64, Assoc=String>);
    ...
}
```
A new mock type will be created for each mock object.
If further attributes should be applied to that type provide them after the type list.
```Rust
#[use_mocks]
fn some_test {
    let mock = new_mock!(MyTrait #[derive(Clone)]#[other_attribute]);
    ...
}
```

When the same mock setup code is shared across multiple tests we can place the mock creation code in a separate factory function, call it in the respective test cases, and modify it further (e.g. adding specific behaviours).
To be able to do this we need to know the name of the created mock type.
So far those types have been anonymous and a name has been chosen by the `new_mock!` command.
It is possible to supply an explicit mock type name.
```Rust
#[use_mocks]
mod test_module {
    fn create_mock() -> mock::MyMockType {
        new_mock!(MyTrait #[some_attribute] for MyMockType)
        ... // define given/expec behaviours
    }

    #[test]
    fn some_test {
        let mock: mock::MyMockType = create_mock();
        ... // define further test=specific given/expec behaviours
    }
}

```
The created type is placed in a `mock` module which is automatically visible to all (sub-)modules and functions within the item annotated with `#[use_mocks]`.

### Defining behaviour with `given!` blocks

After creating a mock object you can invoke the mocked traits' methods on it.
Though as it is just a mock the called methods **will panic** as they don't know what to do.
First you need to define behaviours of the object based on conditions on the method arguments.
Following the terminology of [Behaviour Driven Development (BDD)](https://en.wikipedia.org/wiki/Behavior-driven_development) this is done with a `given!` block.
It sets up the preconditions of the scenario we are testing.
```Rust
given! {
    <mock as MyTrait>::func |&(x, y)| x < y then_return 1 always;
    ...
}
```
A `given!` block consists of several *given statements* with the following pattern.
```Rust
given! {
    <OBJECT as TRAIT>::METHOD ARGUMENT_MATCHERS THEN REPEAT;
    ...
}
```
The statement resembles [Universal Function Call Syntax](https://doc.rust-lang.org/book/first-edition/ufcs.html) with additional components:
* `OBJECT` ... the mock object for which we define the pattern
* `TRAIT` ... the mocked trait to which the `METHOD` belongs. Refering to the trait follows the same rules as `new_mock!`. *The UFC syntax is not optional and for now you must provide generic/associated type arguments in the same order as in the `new_mock!` statement which created the `OBJECT`.*
* `METHOD` ... the method to which the behaviour belongs to
* `ARGUMENT_MATCHERS` ... a precondition on the method arguments which must be fulfilled for the behaviour to be invoked
* `THEN` ... defines what happens after the behaviours has been selected, e.g., return a constant value
* `REPEAT` ... defines how often the behaviour can be matched before it becomes invalid

When a method is invoked its given behaviours' preconditions are checked top-down and the first matching behaviour is selected.
A given block is *not* a global definition and behaves as any other block/statement:
If the control flow never enters the block the behaviours won't be added to the mock object.
If a block is entered multiple times or if another block is reached, then its behaviours are appended to the current list of behaviours.

As writing the full UFC syntax gets tiresome if many behaviours need to be defined for a mock object, a bit of syntactic sugar has been added.
```Rust
given! {
    <OBJECT as TRAIT>::{
        METHOD ARGUMENT_MATCHERS THEN REPEAT;
        METHOD ARGUMENT_MATCHERS THEN REPEAT;
        ...
    };
    ...
}
```
These behaviour blocks get rid of unnecessary duplication.
Note that the semicolon at the end of the block is *not optional*.

*Further note that mocking **static** methods is currently not supported!*.

#### Argument patterns

Preconditions on the method arguments can be defined in two forms: **per-argument** and **explicit**.
Most of the time per-argument patterns will be enough and are considered more readable.
```Rust
given! {
    <mock as MyTrait>::func(|&x| x == 2, |&y| y < 3.0) then_return 1 always;
}
```
The argument matchers follow the closure syntax and its parameters are passed *by immutable reference* and must return a `bool` or something that implements `std::convert::Into<bool>`.
Although we use closure syntax, **this is not a closure** meaning that you can't capture variables from the scope outside the given block.
We will learn later how we can **bind** values from the outer scope to make them available to the given statements.

If the `galvanic_assert_integration` feature is enabled then the matchers from `galvanic-assert` can be used instead of the closure syntax.
See the introduction for some examples

The second form receives all arguments at once in a tuple.
```Rust
given! {
    <mock as MyTrait>::func |&(x, y)| x < y then_return 1 always;
}
```
Again the tuple of curried arguments is passed by reference.
Note that we have to use `ref` when decomposing tuples with non-copyable objects (as in any other pattern in Rust).
Observe the **lack of brackets** after `func` in this form.
The brackets are used to distinguish between the two variants.

#### Returning values

Defining the behaviours' actions once selected is done in the `THEN` part of the statement.
We can either return the value of a constant expression with `then_return`:
```RUST
given! {
    <mock as MyTrait>::func ... then_return (1+2).to_string() always;
}
```

Or we compute a value based on the arguments of the function call with `then_return_from`.
The arguments are again passed as a reference to curried argument tuple.
Note again that we use closure syntax but we cannot capture variables from the outside scope.
```RUST
given! {
    <mock as MyTrait>::func ... then_return_from |&(x,y)| (x + y)*2 always;
}
```
Or simply panic:
```RUST
given! {
    <mock as MyTrait>::func ... then_panic always;
}
```

#### Repetition

The final element of a behaviour is the number of *matching* repetitions before the behaviour is exhausted and will no longer match.
The may either be `always` (as used up to now) or `times` followed by an integer expression.
```Rust
let x: i32 = func()

given! {
    <mock as MyTrait>::func |&(x, y)| x < y then_return 1 times x+1;
}
```
Contrary to argument matchers and then-expressions the `times` expression is evaluated in the context of the given block.

#### Binding values from the outer scope

Up until now argument matchers and then-expressions cannot refer to the outside context.
The reason for this is mainly due to lifetime issues with references when actual closures would be passed to the mock objects.
To get around these issues it is possible to **bind** values from the outside scope in a given block.
```Rust
let x = 1;
given! {
    bind value1: f64 = 12.23;
    bind value2: i32 = x*2;

    <mock as MyTrait>::func |&(_, y)| y > bound.value2 then_return bound.value1 always;
    <mock as MyTrait>::func |&(_, y)| y <= bound.value2 then_return_from |&(x, _)| x*bound.value1 always;
}
```
Bind statements must occur before the given statements with the general form:
```Rust
given! {
    bind VARIABLE: TYPE = EXPRESSION:
    ...
}
```
Note that the type is not optional here.
All variables defined with `bind` can later be accessed with a member of the `bound` variable.
The bind expressions will be evaluated when the given block is entered.
That also means if a given block is entered multiple times the bind statements will be reevaluated for the new behaviours.

#### Behaviours for generic trait methods

Be careful when you try to mock *generic methods* as below.
```Rust
#[mockable]
trait MyTrait {
    fn generic_func<T,F>(x: T, y: F) -> T;
}
...
given! {
    <mock as MyTrait>::generic_func |&(ref x, ref y)| ... then_return 1 always;
}
```
The behaviour will be applied regardless of the actual types used.
Meaning that besides the trait bounds defined on the type arguments you cannot use much else.
We cannot assume, e.g., that `x` is always a `i32` although we might know that depending on the context.
In such a case we must either detect the type ourselves or use `unsafe` casts.
*This will likely change in future versions and get easier/more useful.*

#### Behaviours for static trait methods

*This is currently not supported but is high priority for one of the next versions.*


### Expecting interactions with `expect_interactions!` blocks

Besides defining how a mock should act it is a common use case to want to know that some interactions, i.e., method calls, happened with the mock.
This can be done with an expect block which works similar to given blocks.
```Rust
expect_interactions! {
    <mock as MyTrait>::func(|&x| x == 2, |&y| y < 12.23) times 2;
}
```
Again the block consists of several expect statements with the following general form.
```Rust
expect_interactions! {
    <OBJECT as TRAIT>::METHOD ARGUMENT_MATCHERS REPEAT;
    ...
}
```
Trait blocks, argument matchers, bindings, and evaluation order work in the same way as given blocks.
Repeat expressions support a few different options.
Further a expect behaviour will never be exhausted.
The expect statements only specify the testing order of the patterns, they do not specify the expected order of interactions.
The order of interactions in a `expect_interactions` block is assumed to be arbitrary.
Also only the first matching expect expression will be counted.
Later expression whose argument matchers would also be satisfied with the same arguments will not be evaluated.

*Specifying a fixed order is currently not supported.*

The expectations are verified once the mock object is dropped or if `mock.verify()` is called.
If the expected interactions did not happen as specified when verified the current thread will panic.
If other interactions not matching any expect behaviour occured then they won't be seen as errors.

#### Repetition

The repeat expressions for a expect block can be one of the following.
* `times EXPRESSION` ... states that *exactly* `EXPRESSION` number of matches must occur.
* `never` ... states the interaction should never be encountered (same as `times 0`).
* `at_least EXPRESSION` ... states that at least `EXPRESSION` (inclusive) number of matches must occur.
* `at_most EXPRESSION` ... states that at most `EXPRESSION` (inclusive) number of matches must occur.
* `between EXPRESSION1, EXPRESSION2` ... states that a number of matches in the inclusive range [`EXPRESSION1`, `EXPRESSION2`] should occur.

### The `Mock` interface

All mocks support some basic methods for controlling the mock.
* `should_verify_on_drop(bool)` ... if called with `false` verification on drop will be disabled and vice versa.
* `reset_given_behaviours()` ... removes all given behaviours from the mock
* `reset_expected_behaviours()` ... removes all expectations from the mock
* `are_expected_behaviours_satisfied()` ... return `true` if all expectations are currently satisfied, `false` otherwise.
* `verify()` ... panics if some expectaions are currently unsatisfied.
