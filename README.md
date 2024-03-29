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
    rcss_leptos::register_style(css.clone());

    view! {
        <div class=css.container>Hello</div>
    }
}
```
More examples can be found in `examples/leptos` directory.


## Bundling CSS:
RCSS can bundle all CSS into a static file.
To do that one can use `rcss-bundler` crate in build.rs.
    
```toml
[build-dependencies]
rcss-bundler = "0.2"
```

```rust
fn main() {
    rcss_bundler::bundle_build_rs();
}
```

By default, the bundler will save CSS into `$OUT_DIR/styles.css`. However, this can be customized through cargo metadata.


```toml
[package.metadata.rcss]
output-path = "style/counters.css" # Path to save styles
disable-styles = false # If set to true will force `rcss-macro` to remove style strings from macro output.
```

Note: `disable-styles` can be ignored by `rcss-macro` if `rcss-bundler` was added after the first build.
One can use `cargo clean -p rcss-macro` or `cargo clean -p rcss-macro --target-dir target/front` (in case of cargo-leptos) to force cargo rebuild `rcss-macro`.

## Known issues:
1. `rcss-bundler` should be used only from one root crate. If you have multiple crates that use `rcss-bundler`, or workspace that share building artifacts of multiple root crates in one target directory - it will cause a conflict with configuring. For example, if one root crate sets `disable-styles=true` and one does not, the `rcss-bundler` will throw a warning and use `disable-styles=false` for all workspace/crates.

2. Currently, CSS parsing errors are not fully integrated with rust diagnostics messages, so if you have invalid CSS syntax it will just throw a generic error, without highlighting the exact place of the error.

3. Unquoted text in macros has a few limitations like it can't contain single quotes `'` or unfinished braces (like `{` or `(`). This is because of the way Rust provides TokenStream to macro.

4. Unquoted text doesn't work well with `em` units and some hex numbers that start with number and has letter `e` (like `#0ed`) because rust parses them as exponential number literals, and expecting a number after `e` instead of a letter "m".

For problems 3-4, one can use interpolation as workaround `#{"3em"}` with quoted text inside.

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
