//! CSS embeding library.
//!
//! The purpose of this library is to add support of embeded
//! styles to wasm frameworks and other rust driven web apps.
//!
//! Suport various css preprocessors:
//! - Browsergrade lightningcss
//! - lightweight procss
//! - stylers_core from leptos ecosystem
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

pub mod inline {
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
    pub mod css_modules {
        pub use rcss_macro::css_module_inline as css;
        /// Generate module with css classes object named Css and static variable
        /// STYLE with style string.
        /// Uses ident from first macro argument as module name.
        /// Expect arrow '=>' after module name.
        ///
        /// Example:
        /// ```rust
        /// rcss::inline::css_modules::css_mod! {
        /// module =>
        ///     .my-class {
        ///         color: red;
        ///     }
        /// }
        /// ```
        ///
        pub use rcss_macro::css_module_mod as css_mod;
    }

    /// Scoped css, that output content for inline `<style>` element.
    /// Uses random class as scope separator. So it should be used inside each element.
    /// Output class name and style as result of macro call.
    ///
    ///
    /// Example:
    /// ```rust
    /// let (class, style) = rcss::inline::scoped::css! {
    ///   .my-class {
    ///     color: red;
    ///   }
    /// };
    ///
    /// // Example of usage
    /// let html = format!(r#"<div class="{} my-class">Hello</div>"#, class);
    /// let html = format!(r#"<style>{}</style>{}"#, style, html);
    ///
    /// ```
    ///
    pub mod scoped {
        pub use rcss_macro::css_scoped_inline as css;
    }
}

pub mod file {
    /// Scoped css, that accumulate all styles into a single file.
    /// Should be used with build.rs api to generate css file.
    /// Uses random class as scope separator. So it should be used inside each element.
    /// Output class name as result of macro call.
    /// Panics if called without build.rs initialization
    ///
    /// Example:
    /// ```rust
    /// let class = rcss::file::scoped::css! {
    ///     .my-class {
    ///         color: red;
    ///     }
    ///     .my-class2 {
    ///         color: blue;
    ///     }
    /// };
    ///
    /// // Example of usage
    ///
    /// let html = format!(r#"<div class="{} my-class">Hello</div>"#, class);
    /// // File style.css will be generated in target dir.
    /// // Use build.rs api to specify its name.
    /// let html = format!(r#"<link rel="stylesheet" href="style.css" />{}"#, html);
    /// ```
    pub mod scoped {
        pub use rcss_macro::css_scoped as css;
    }

    pub mod css_module {
        pub use rcss_macro::css_module as css;

        /// Generate struct with css classes as fields.
        /// Uses ident from first macro argument as struct name.
        ///  
        /// Example:
        /// ```rust
        /// rcss::file::css_module::css_struct! {
        /// Foo =>
        ///  .my-class {
        ///     color: red;
        ///  }
        /// }
        /// ```
        ///
        pub use rcss_macro::css_module_struct as css_struct;
    }
}
