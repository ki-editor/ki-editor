pub struct Foo {
    a: (),
    b: (),
}

pub(crate) fn foo() -> Foo {
    Foo { a: (), b: () }
}
