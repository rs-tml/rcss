rcss::css! {
    @rcss(mod my_mod);

    .my-class {
        color: green;
    }
}

fn main() {
    let foo = rcss::css! {
        .other_class {
            color: red;
        }
    };
    println!("Hello, world {} {}!", foo.other_class, my_mod::STYLE);
}
