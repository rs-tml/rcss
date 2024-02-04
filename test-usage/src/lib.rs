rcss::css! {
    @rcss(pub struct Foo);

    .my-class {
        color: red;
    }
}

rcss::css! {
    @rcss(extend Foo);
    @rcss(pub struct Bar);

    .my_class {
        color: red;
    }
}

#[test]
fn test() {
    let foo = Bar::default();
    let _ = foo.my_class;
}
