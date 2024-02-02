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
#[cfg(feature = "build-helper")]
pub mod build_helper;

/// Api that mimic js "CSS modules", it generate struct with css classes as fields.
/// Output instance of this struct as result of macro call.
///
/// Usefull to avoid runtime errors, and to have autocompletion in IDE.
/// Internally wraps rename all css classes with random names,
/// keeping original names as prefix.
///
/// Example:
/// ```rust
/// let (css, inline) = rcss::inline::css_modules::css! {
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
/// let html = format!("<style>{}</style>\n{}", inline, html);
/// assert_eq!(html, "<style>.my-class-xxTX{color:red}</style>\n\
/// <div class=\"my-class-xxTX\">Hello</div>");
/// ```
pub use rcss_macro::css;
mod types;
pub use types::*;
