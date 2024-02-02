rcss::css! {
    @rcss(pub mod my_mod);

    .my-class {
        color: red;
    }
}

fn test() {
    let foo = my_mod::default();
    let _ = foo.my_class;
}
