rcss::css_struct! { @rcss(mod foo); .my-class2 { color: blue; } }

#[any_attribute]
fn some_method(foo: Argument<impl Generic>) {
    let class = rcss::css! { .my-class { color: red; } };

    let class = rcss::css! { .container { background-color: black; } };
}
