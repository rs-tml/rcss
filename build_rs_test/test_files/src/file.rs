rcss::file::css_module::css_struct! { Foo => .my-class2 { color: blue; } }

#[any_attribute]
fn some_method(foo: Argument<impl Generic>) {
    let class = rcss::file::css_module::css! { .my-class { color: red; } };

    let class = rcss::file::scoped::css! { .container { background-color: black; } };
}
