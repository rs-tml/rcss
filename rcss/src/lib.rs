//! CSS embeding library.
//!
//! The purpose of this library is to add support of embeded
//! styles to wasm frameworks and other rust driven web apps.
//!
//! Can output css files, or elements in inline `<style>` element.
//! Provide scope or css_modules like api.
//!
//! For scoped api, it generate random class name as scope identifier.
//! For css_modules api, it modify class names to be unique.
//!
//! Aim of macros in this library is to give support of unquoted styles on stable rust.
//!
// https://drafts.csswg.org/css-scoping/

/// Api that mimic js "CSS modules", it generate struct with css classes as fields.
/// Output instance of this struct as result of macro call.
///
/// Usefull to avoid runtime errors, and to have autocompletion in IDE.
/// Internally wraps rename all css classes with random names,
/// keeping original names as prefix.
///
/// Example:
/// ```rust
/// use rcss::*;
/// let css = rcss::css! {
///    .my-class {
///      color: red;
///   }
/// };
///
/// // Note: css.my_class is in snake_case, uses feature = "auto-snake-case"
/// assert!(css.my_class.contains("my-class"));
/// // WARN: runtime panic if class is not found, feature = "indexed-classes"
/// assert!(css["my-class"].contains("my-class"));
///
/// // Example of usage
/// let html = format!(r#"<div class="{}">Hello</div>"#, css.my_class);
/// // Note: This style is only used for current style, and doesn't combine well with extend.
/// let html = format!("<style>{}</style>\n{}", css.scope_style(), html);
/// assert_eq!(html, "<style>.my-class-Mlfe{color:red}</style>\n\
/// <div class=\"my-class-Mlfe\">Hello</div>");
/// ```
pub use rcss_macro::css;
/// Common types that used in defining scopes for css.
mod types;
pub use types::*;
/// Traits that are used to define chain of css scopes.
pub mod extend;

#[doc(hidden)]
pub mod reexport {
    pub use const_format;
}
