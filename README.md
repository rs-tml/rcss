# RCSS - Reusable CSS in the Rust app.

RCSS allows you to write styles directly in Rust code, mostly without quotes.

```rust
let css = css!{
    .container {
        background-color: black;
    }
    #id {
        color: red;
    }
    button {
        border: 1px solid black;
    }
};
assert!(css.container.starts_with("container"));
```

RCSS uses LightningCSS (browser-grade CSS preprocessors) to parse and generate CSS code.
It can bundle all CSS into static files, or generate inline CSS content.

It uses an approach similar to css_modules from JS world to expose CSS classes in a type-safe manner. For a selector that doesn't use class names, it uses a scoped-style approach (add a custom class with unique name).

Styles defined with RCSS can be extended and reused in other components.

```rust 
// Declare a public module name foo;
css!{
    @rcss(pub struct Foo);

    .container {
        color: green;
        background-color: black;
    }
};

// Component define what type of css it needs.
fn hello_world<C>(css: C) -> String
C: rcss::ScopeChain<Root = Foo>
{
    // use ScopeChain to include style chain. Checkout rcss-layers for details.

    format!(r#"<div class="{}">Hello</div>"#, css.container)
}
...

// Extend style from module foo, typecheck that no new classes, ids or types are added.
let css = css!{
    @rcss(extend ::path::to::Foo);

    .container {
        background-color: red;
    }
};

let html = hello_world(css);
```
To better control cascading and avoid conflicts, RCSS provides crate `rcss-layers` that can save extended styles into CSS `@layer`.

## Usage:

```toml
[dependencies]
rcss = "0.2"
```

RCSS is not focused on any specific web framework, it can be used even without frameworks, just with `format!`, but to make it more convenient, this repository
will also include integration with popular web frameworks.
Currently, there is only one integration crate: `rcss-leptos` for [leptos](https://github.com/leptos-rs/leptos)

### Usage with leptos:
To use rcss with leptos, add this to your `Cargo.toml`:

```toml
[dependencies]
rcss = "0.2"
rcss-leptos = "0.2"
```

then in component, you can use `css!` macro to define styles.
```rust
use rcss::css;
use leptos::*;

#[component]
fn hello_world() -> Node {
    let css = css!{
        .container {
            background-color: black;
            color: white;
        }
    };
    rcss_leptos::register_style_chain_as_layers(css.clone());

    view! {
        <div class=css.container>Hello</div>
    }
}
```
More examples can be found in `examples/leptos` directory.



## Macro implementation details: 
There are two ways of writing function macros in Rust.
- One is to handle `TokenStream` from proc-macro.
This way saves links to the original code, therefore IDE and compiler can show errors linked to the original code, variables can be resolved and so on. But `TokenStream` in Rust is specialized for Rust syntax and it's harder to support foreign syntax in it.

Check out [rstml::RawText](https://github.com/rs-tml/rstml/blob/main/src/node/raw_text.rs) or [rstml::NodeName](https://github.com/rs-tml/rstml/blob/main/src/node/node_name.rs) both of these structs provide a hacky way to parse HTML-specific syntax, like dash-seperated-idents or unquoted text.

- The other way is to handle macro input as a regular string.
This way is more flexible because you can use any parser you want, but you lose all the benefits of `TokenStream`` and have to write your parser.

Unlike HTML templating where you need some way to mix reactive variables from Rust and templates -
CSS usually contains static predefined content, which rarely needs to be generated at runtime.
Therefore link between the original code and IDE is less important. Instead, most of the users just want to write CSS for their components near its implementation.

So instead of writing a custom parser on top of `proc-macro::TokenStream`, this library tries to convert macro calls into strings and work with convenient CSS preprocessors.
