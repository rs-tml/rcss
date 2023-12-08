# RCSS - Macro that embeds CSS into Rust app.

## Motivation: 
There are two ways of writing function macros in Rust.
- One is to handle `TokenStream` from proc-macro.
This way saves links to the original code, therefore IDE and compiler can show errors linked to the original code, variables can be resolved and so on. But `TokenStream` in Rust is specialized for Rust syntax and it's harder to support foreign syntax in it.

Check out [rstml::RawText](https://github.com/rs-tml/rstml/blob/main/src/node/raw_text.rs) or [rstml::NodeName](https://github.com/rs-tml/rstml/blob/main/src/node/node_name.rs) both of these structs provide a hacky way to parse HTML-specific syntax, like dash-seperated-idents or unquoted text.

- The other way is to handle macro input as a regular string.
This way is more flexible because you can use any parser you want, but you lose all the benefits of `TokenStream`` and have to write your parser.

Unlike HTML templating where you need some way to mix reactive variables from Rust and templates -
CSS usually contains static predefined content, which rarely needs to be generated at runtime or compile time.
Therefore link between the original code and IDE is less important. Instead, most of the users just want to write CSS for their components near its implementation.

So instead of writing a custom parser on top of `proc-macro::TokenStream`, this library tries to convert macro calls into strings and work with convenient CSS preprocessors.

## Info:

RCSS supports various embedding modes:
- Can preprocess CSS and output struct that work like css_modules.
- Can output class-name for scoped-style API (this class name should be injected into all elements).

Support multiple backends:
[stylers](https://github.com/abishekatp/stylers) - a library that provides CSS-like syntax, and is aimed for the `leptos`` framework. Has a smaller dependencies footprint, but slower and less powerful.

- [lightningcss](https://lightningcss.dev/) - library from Parcel, written on top of browser grade `cssparser` - extremely fast and powerful. But has a large dependencies footprint.

- [procss](https://github.com/ProspectiveCo/procss) (in plans) - simple nom-based CSS parser, and preprocessor. Has a small number of dependencies, but works slower than `lightningcss`, and doesn't support all CSS syntax.

Can output inline CSS content or aggregate all CSS into one file (using build.rs).
Contains various helpers for macro and build.rs writing.

## Usage:
Lib contains a feature flag for each backend, so you can choose which one to use.

```toml
[dependencies]
rcss = { version = "0.1", features = ["lightningcss"] }
```
By default "lightningcss" is enabled, but you can avoid using it by setting `default-features = false`.
This allows for optimized compilation time and reduces dependency's footprint on demand.

All API is divided into modules, and can be represented as a tree:
```text
--- rcss
    |- inline  (api that return css as string in second output params)
    |   |- css_modules 
    |   |- scoped
    |- file  (api that doesn't return css as string, but provide a way to agregate them into file using build.rs)
        |- css_modules
        |- scoped
```
Macros in this library can't be used from other macros, because they need access to source code.
Macros in `file` should be used in pair with build.rs helper. 

Scoped CSS is inspired by Vue, other js frameworks and Shadow DOM, in vue it uses custom attributes and CSS preprocessors in order to scope CSS rules.
Our approach uses class names, that should be attached to each styled html element.
It can be automatically injected by a framework, or manually by the user.
Scoped API contains a single macro `css!` that can be used to generate scoped CSS class names.
It returns a tuple of `(css_class, inline_css)` on inline API and `(css_class)` on file API.
`css_class` is a string that should be injected into each styled html element.
`inline_css` is a string that should be injected to `<style>` or in another way delivered to the client as CSS content.

`css_modules` is familiar for js developers, it dynamically changes each class-name, and provides a single object with original class names as fields, and new class names as values.
The same approach is applied in Rust, but instead of an object, it returns a newly generated struct.
Css modules API contains two macros `css!` and `css_struct!` (`css_mod!` for inline API).
`css!` macro works in a similar way as scoped API, but instead of `class_name` it returns struct which fields can be used to style HTML elements.
 `css_struct!` is used to define CSS struct in a global context.

The rest of the config specific to each backend, is planned to be configured through environment variables.

## Examples: 

RCSS can be used without frameworks at all, for example using scoped-style API:
```rust
use rcss::inline::scoped::css;

fn main() {
    let (css_class, style) = css!{
        .container {
            background-color: black;
        }
    };
    
    let html = format!(r#"<div class="{} my-class">Hello</div>"#, css_class);
    let html = format!(r#"<style>{}</style>{}"#, style, html);
    // output html.
}

```

It's easy to use it with any framework,
example for `leptos` framework, using `css_modules` API:

```rust

use leptos::prelude::*;
use rcss::inline::css_modules::css;

#[component]
fn some_component() -> impl View {
    let (css_class, inline_css) = css!{
        .container {
            background-color: black;
        }
    };
    
    view! {
        <div class={css_class.container}>
            Hello
        </div>
        <style>
            {inline_css}
        </style>

    }
}

```

Leptos also supports scoped-style API through class injecting, so a more complex example using scoped-API can be written as:

build.rs:
```rust
use rcss::builer_helper::process_styles_to_file;
fn main () {
    let project_path = std::env!("CARGO_MANIFEST_DIR");
    let output = format!("{}/target/styles.rs", project_path);
    process_styles_to_file(project_path, output);
}
```

component.rs:
```rust
use leptos::prelude::*;
use rcss::file::scoped::css;

#[component]
fn some_component() -> impl View {
    let css_class = css!{
        .container {
            background-color: black;
        }
    };
    
    view! {
        <div class={css_class}>
            Hello
        </div>

    }
}

```

