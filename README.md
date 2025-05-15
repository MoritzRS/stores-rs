# ![stores-rs](./logo.svg)

![Version](https://img.shields.io/badge/version-0.1.0-blue)
![License](https://img.shields.io/badge/license-MIT-green)

Observable primitives for rust, inspired by Svelte's Stores.

## Features

- [x] Events
- [x] Observable Values
- [x] Derived Values
- [x] Deduplication
- [x] Thread Safe
- [x] Useful Macros

## What is this?

This crate includes a limited set of observable/reactive primitives.
The design and implementation is inspired by Svelte's Store contract.
Since Observables are based on simple traits, users could also implement their own versions of it.

## When should I use this?

This crate is useful if you want to work with observable values.
A good example are UI-Projects (e.g. with GTK).

## Usage

### Install

```sh
cargo add --git https://github.com/MoritzRS/stores-rs --tag v0.1.0
```

### Examples

#### Event

An event can be used to trigger callbacks.
It holds no values.

```rust
use stores::{Emitter, Event};

fn main() {
    let event = Event::new();

    let unsubscribe = event.listen(|| {
        println!("Event triggered");
    });

    event.dispatch(); // "Event triggered"
    event.dispatch(); // "Event triggered"

    unsubscribe();
    event.dispatch(); // Nothing
}
```

#### Observable

An observable is the most basic primitive in this crate.
You can read and write values and subscribe to changes.

```rust
use stores::{Readable, Writable, Observable};

fn main() {
    let my_number = Observable::new(0);

    let unsubscribe = my_number.subscribe(|value| {
        println!("My number is: {}", value);
    }); // "My number is: 0"

    my_number.set(1); // "My number is: 1"
    my_number.set(2); // "My number is: 2"
    my_number.update(|value| value * 10); // "My number is: 20"

    unsubscribe();
    my_number.set(30); // Nothing
}
```

#### Derived

A derived an read only observable that computes its value from its dependencies.
You can read its value and subscribe to changes.

```rust
use stores::{Readable, Writable, Observable, Derived, derived};

fn main() {
    let a = Observable::new(1);
    let b = Observable::new(2);

    // Manual Creation
    let sum = Derived::new(&[a.clone(), b.clone()], {
        let a = a.clone();
        let b = b.clone();
        move || a.get() * b.get()
    });

    // Macro based Creation
    let sum = derive!([a, b] => move || a.get() * b.get());
}
```

#### Deduped

A deduped behaves like an observable, but only notifies its subscribers when its value has actually changed.
It can either be standalone or wrap other observables.
When it wraps another writable observable, it propagates changes to the original.

```rust
use stores::{Readable, Writable, Observable, Deduped};

fn main() {
    let my_number = Observable::new(0);
    let deduped = Deduped::from(my_number.clone());

    let _ = deduped.subscribe(|value| {
        println!("Value is {}", value);
    }); // "Value is 0"

    my_number.set(1); // "Value is 1"
    my_number.set(2); // "Value is 2"
    my_number.set(2); // Nothing
}
```

## Disclaimer

This is one of my first rust projects.
I developed it just because I needed for another project.
Although I am quite satisfied with the result, breaking changes may occur when my projects demand it.

## License

MIT © Moritz R. Schulz
