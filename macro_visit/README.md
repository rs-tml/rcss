Simple helper to register macro handler and process their input from build.rs
Currentyl support only function-like macro.

Usefull when your macro should output file, but you want to avoid race conditions during incremental compiliation and other routines.

Uses [syn](https://crates.io/crates/syn) to parse file and visit their macros.


Imagine you have some macro `css` that implement scoping css technic.
It process css syntax and return css_class. But you also need to agregate all `css!` calls and save them into `style.css`
```rust
fn main() {
    use rcss::css as my_css;
    let css_class = my_css!{
        .container {
            background-color: black;
        }
    };
    
    let html = format!(r#"<div class="{} my-class">Hello</div>"#, css_class);
    let html = format!(r#"<link rel="stylesheet" href="style.css" />{}"#, html);
    // output html.
}

```
In order to agregate all css, you should open all src files,
find if css! was used and handle their input.
You also should handle imports and renames.

Target of this crate is to deal with this problems.

In build.rs
```rust
fn main () {
    let project_path = std::env!("CARGO_MANIFEST_DIR");
    let crate_name = std::env::var("CARGO_CRATE_NAME").unwrap_or("rcss".to_owned());
    let collect_style = RefCell::new(String::new());

    let mut css_handler = |token_stream: TokenStream| {
       collect_style.borrow_mut().push_str(/* Handle token_stream */ )
    };

    let mut visitor = Visitor::new();

    let css_macro_path = vec![format!("{crate_name}::css")];
    let css_macro = Rc::new(RefCell::new(MacroCall::new(&mut css_handler)));
    visitor.add_macro(css_macro_path, css_macro);
    visitor.visit_project(project_path);
    let content = collect_style.into_inner();
    // .. save content to a file
}

```

`macro_visit` will find all occurences of css macro, even if it was renamed.
Currently it only looks for imports inside one file at time, and does not parallelize file processing.


