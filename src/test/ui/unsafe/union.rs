// revisions: mir thir
// [thir]compile-flags: -Z thir-unsafeck

#![feature(untagged_unions)]

union Foo {
    bar: i8,
    zst: (),
    pizza: Pizza,
}

struct Pizza {
    topping: Option<PizzaTopping>
}

#[allow(dead_code)]
enum PizzaTopping {
    Cheese,
    Pineapple,
}

fn do_nothing(_x: &mut Foo) {}

pub fn main() {
    let mut foo = Foo { bar: 5 };
    do_nothing(&mut foo);

    // This is UB, so this test isn't run
    match foo {
        Foo { bar: _a } => {}, //~ ERROR access to union field is unsafe
    }
    match foo { //[mir]~ ERROR access to union field is unsafe
        Foo {
            pizza: Pizza { //[thir]~ ERROR access to union field is unsafe
                topping: Some(PizzaTopping::Cheese) | Some(PizzaTopping::Pineapple) | None
            }
        } => {},
    }

    // MIR unsafeck incorrectly thinks that no unsafe block is needed to do these
    match foo {
        Foo { zst: () } => {}, //[thir]~ ERROR access to union field is unsafe
    }
    match foo {
        Foo { pizza: Pizza { .. } } => {}, //[thir]~ ERROR access to union field is unsafe
    }

    // binding to wildcard is okay
    match foo {
        Foo { bar: _ } => {},
    }
    let Foo { bar: _ } = foo;
}
