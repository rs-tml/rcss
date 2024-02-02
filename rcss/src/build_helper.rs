use proc_macro2::TokenStream;
use std::{cell::RefCell, path::Path};
use syn::spanned::Spanned;

use macro_visit::Visitor;
use rcss_core::CssOutput;

/// Method that will find all css macro calls in the given project path,
/// and accumulate they css into single file.
///

pub fn process_styles_to_file(
    project_path: &str,
    output_file: impl AsRef<Path>,
) -> std::io::Result<()> {
    let output = process_styles(project_path, |_| todo!());
    CssOutput::merge_to_file(&output, output_file)
}

// Scan project_path using syn folder, and find all css macro calls.
pub fn process_styles<F>(project_path: &str, preprocessor: F) -> Vec<CssOutput>
where
    F: Fn(&str) -> CssOutput,
{
    let crate_name = std::env::var("CARGO_CRATE_NAME").unwrap_or("rcss".to_owned());

    let collect_style = RefCell::new(Vec::new());

    let css_handler = |token_stream: TokenStream| {
        let source_text = token_stream
            .span()
            .source_text()
            .expect("cannot find source text for macro call");
        collect_style.borrow_mut().push(preprocessor(&source_text));
    };
    let mut visitor = Visitor::new();

    let css_struct_paths = vec![format!("{crate_name}::css")];
    visitor.add_macro(css_struct_paths, css_handler);

    visitor.visit_project(project_path);
    collect_style.into_inner()
}
