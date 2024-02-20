//!
//! Small library helper that uses syn::visit::Visit trait to find all macro calls in project structure.
//!
//! By the way of traversing, library will resolve imports, so end user can
//! rename macros and mix macros with same name from different crates.
//! It also resolve mod calls and provide some context to macro handler.
//!

use std::{cell::RefCell, collections::BTreeMap, path::PathBuf, rc::Rc};

use proc_macro2::TokenStream;

/// Macro visitor.
///
/// Handle all macro calls, and call appropriate function.
/// on the way, it will find all `use` items, and add new imports to the list.
///
/// Creates new visitor for each function, to avoid mixed `use` items.
///
/// It uses lifetime to allow variable to be captured into closure.

pub type RcMacro<'a> = Rc<RefCell<dyn FnMut(MacroContext, TokenStream) + 'a>>;
pub type MacroMap<'a> = BTreeMap<String, RcMacro<'a>>;

/// Provided context to macro handler.
#[derive(Clone, Default, Debug)]
pub struct MacroContext {
    /// Path to modules from entrypoint.
    pub mod_path: Vec<String>,
    /// Filename of entrypoint.
    pub entrypoint: String,
    pub fn_call_name: Option<String>,
    // Somewhere at module path, one used `#[path = "foo.rs"]`
    pub used_path_attr: bool,
    src_filedir: PathBuf,
    // TODO: linenum/colnum
}
#[derive(Clone)]
pub struct Visitor<'a> {
    searched_imports: MacroMap<'a>,

    context: MacroContext,
}
impl std::fmt::Debug for Visitor<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Visitor").finish()
    }
}

impl<'a> Default for Visitor<'a> {
    fn default() -> Self {
        Self::new()
    }
}

impl<'a> Visitor<'a> {
    /// Creates empty visitor.
    pub fn new() -> Self {
        Self {
            searched_imports: BTreeMap::new(),
            context: MacroContext::default(),
        }
    }
    /// Add macro implementation to the macro
    pub fn add_macro(
        &mut self,
        imports: Vec<String>,
        macro_call: impl FnMut(MacroContext, TokenStream) + 'a,
    ) {
        let macro_call = Rc::new(RefCell::new(macro_call));
        for import in imports {
            self.searched_imports.insert(import, macro_call.clone());
        }
    }
    pub fn add_rc_macro(&mut self, imports: Vec<String>, macro_call: RcMacro<'a>) {
        for import in imports {
            self.searched_imports.insert(import, macro_call.clone());
        }
    }
    // Visit file content.
    pub fn visit_file_content(&mut self, content: &str) {
        println!("Process file with context: {:?}", self.context);
        println!("and imports: {:?}", self.searched_imports.keys());
        let file = syn::parse_file(content).unwrap();
        syn::visit::visit_file(self, &file)
    }
    /// Handle all *.rs files in src of project directory.
    ///
    /// `project_path` - is path to Cargo.toml of the project
    pub fn visit_project<P: AsRef<std::path::Path>>(&self, entrypoint: P) {
        let entrypoint = entrypoint.as_ref();

        println!(
            "Processing project with entrypoint: {:?}",
            entrypoint.display()
        );
        let content = std::fs::read_to_string(entrypoint).unwrap();

        let mut src_filedir = entrypoint.to_path_buf();
        src_filedir.pop();
        let entrypoint_name = entrypoint
            .file_stem()
            .map(|c| c.to_string_lossy())
            .unwrap_or_default();

        Self {
            context: MacroContext {
                entrypoint: entrypoint_name.to_string(),
                src_filedir,
                ..MacroContext::default()
            },
            ..self.clone()
        }
        .visit_file_content(&content)
    }
    fn new_subcall(&self, fn_name: String) -> Self {
        let mut new = self.clone();
        new.context.fn_call_name = Some(fn_name);
        new
    }

    // Hide current imports to parrent imports.
    fn new_mod(&self, mod_name: String) -> Self {
        let mut context = self.context.clone();
        context.mod_path.push(mod_name);
        Self {
            searched_imports: self.searched_imports.clone(),
            context,
            ..self.clone()
        }
    }
    fn get_macro(&self, path: syn::Path) -> Option<RcMacro<'a>> {
        let path_str = path
            .segments
            .iter()
            .map(|s| s.ident.to_string())
            .collect::<Vec<_>>()
            .join("::");
        self.searched_imports.get(&path_str).cloned()
    }
}

impl syn::visit::Visit<'_> for Visitor<'_> {
    fn visit_use_tree(&mut self, node: &syn::UseTree) {
        let mut new_imports = vec![];
        for (import, macro_call) in &self.searched_imports {
            let use_tree_form = use_tree_from_str(import);
            let new = compare_use_tree(use_tree_form, node.clone());
            if !new.is_empty() {
                println!("Extending use tree with: {:?}", new);
                new_imports.extend(new.into_iter().map(|i| (i, macro_call.clone())))
            }
        }
        self.searched_imports.extend(new_imports);
    }
    fn visit_item_fn(&mut self, node: &syn::ItemFn) {
        let mut new_visitor = self.new_subcall(node.sig.ident.to_string());
        syn::visit::visit_item_fn(&mut new_visitor, node);
    }

    fn visit_impl_item_fn(&mut self, i: &syn::ImplItemFn) {
        let mut new_visitor = self.new_subcall(i.sig.ident.to_string());
        syn::visit::visit_impl_item_fn(&mut new_visitor, i);
    }
    fn visit_item_mod(&mut self, i: &syn::ItemMod) {
        // get attrs #[path = "foo"];
        let path_attr = i
            .attrs
            .iter()
            .filter_map(|a| a.meta.require_name_value().ok())
            .filter(|meta| meta.path.is_ident("path"))
            .last();
        let path_attr = path_attr.map(|a| match &a.value {
            syn::Expr::Lit(syn::ExprLit {
                lit: syn::Lit::Str(lit_str),
                ..
            }) => PathBuf::from(lit_str.value()),
            _ => panic!("Expected literal string in path attribute"),
        });
        // Create new visitor for module
        let mut mod_visitor = self.new_mod(i.ident.to_string());

        println!("Found mod: {:?}", i.ident.to_string());
        // Process items if content is present.
        if let Some(c) = &i.content {
            println!("Processing module with inner content");
            for i in &c.1 {
                mod_visitor.visit_item(i)
            }
            return;
        }
        // Process module that uses #[path = "foo"];
        if let Some(path_attr) = path_attr {
            let full_path = mod_visitor.context.src_filedir.join(path_attr);
            println!(
                "Processing module that uses #[path = \"{}\"]",
                full_path.display()
            );
            let mut src_filedir = full_path.clone();
            src_filedir.pop();
            let mod_name = full_path
                .file_stem()
                .map(|c| c.to_string_lossy())
                .unwrap_or_default()
                .to_string();

            mod_visitor.context.used_path_attr = true;
            mod_visitor.context.src_filedir = src_filedir;
            mod_visitor.context.mod_path = vec![];
            let mod_path = resolve_module_path(&mod_visitor.context, &mod_name);

            let content = std::fs::read_to_string(mod_path).unwrap();
            mod_visitor.visit_file_content(&content);
            return;
        }

        let mod_path = resolve_module_path(&mod_visitor.context, &i.ident.to_string());
        println!(
            "Processing regular module with content in path: {}",
            mod_path.display()
        );
        // Or process file in case of `mod foo;` item.
        let content = std::fs::read_to_string(mod_path).unwrap();
        mod_visitor.visit_file_content(&content)
    }

    fn visit_macro(&mut self, i: &syn::Macro) {
        if let Some(macro_impl) = self.get_macro(i.path.clone()) {
            macro_impl.borrow_mut()(self.context.clone(), i.tokens.clone());
        }
    }
}

// Compare two paths, and return new one, if path was renamed.
// Expect left path to be flat, and right might be nested.
pub(crate) fn compare_use_tree(left: syn::UseTree, right: syn::UseTree) -> Vec<String> {
    match (left, right) {
        (syn::UseTree::Glob(_), _)
        | (syn::UseTree::Group(_), _)
        | (syn::UseTree::Rename(_), _) => {
            panic!("Import path is not valid")
        }
        // If right is glob, then we remove prefix, and keep the rest import path as synonim.
        (left_tree, syn::UseTree::Glob(_)) => {
            vec![create_import_path(left_tree)]
        }
        // If right is group - traverse each group item.
        (left_tree, syn::UseTree::Group(right_g)) => {
            right_g.items.into_iter().flat_map(move |item| {
                compare_use_tree(left_tree.clone(), item)
            }).collect::<Vec<_>>()
        }
        // Name is terminal node,
        // if it equal - we can use macro by its name without full path.
        (syn::UseTree::Name(left_i), syn::UseTree::Name(right_i))
        if right_i.ident == left_i.ident  =>
        {
            vec![create_import_path(syn::UseTree::Name(left_i))]
        }
        // Same but ident is renambed
        (syn::UseTree::Name(left_i), syn::UseTree::Rename(right_r))
        if right_r.ident == left_i.ident => {
            vec![create_import_path(syn::UseTree::Name(
                syn::UseName {
                    ident: right_r.rename,
                }))]
        }
        (syn::UseTree::Path(left_p), syn::UseTree::Name(right_i))
        if right_i.ident == left_p.ident => {
            vec![create_import_path(syn::UseTree::Path(left_p))]
        }
        (syn::UseTree::Path(left_p), syn::UseTree::Rename(right_r))
        if right_r.ident == left_p.ident => {
            let mut new_tree = left_p.clone();
            new_tree.ident = right_r.rename;
            vec![create_import_path(syn::UseTree::Path(new_tree))]
        }
        (syn::UseTree::Path(left_p), syn::UseTree::Path(right_p))
        if right_p.ident == left_p.ident => {
            // traverse deeper, while path is same
            compare_use_tree(*left_p.tree, *right_p.tree)
        }
        (syn::UseTree::Path(_), syn::UseTree::Name(_))
        | (syn::UseTree::Path(_), syn::UseTree::Rename(_))
        | (syn::UseTree::Name(_), syn::UseTree::Name(_))
        | (syn::UseTree::Name(_), syn::UseTree::Rename(_))
        | (syn::UseTree::Path(_), syn::UseTree::Path(_))
        // not comparable
        | (syn::UseTree::Name(_), syn::UseTree::Path(_))
         => {
            // if path is different, then we can't add new synonim for this import.
            vec![]
        }
    }
}
pub(crate) fn use_tree_from_str(path: &str) -> syn::UseTree {
    syn::parse_str(path).unwrap()
}

pub(crate) fn create_import_path(remining: syn::UseTree) -> String {
    let mut path = String::new();
    match remining {
        syn::UseTree::Name(ident) => {
            path.push_str(&ident.ident.to_string());
        }
        syn::UseTree::Path(path_tree) => {
            path.push_str(&path_tree.ident.to_string());
            path.push_str("::");
            path.push_str(&create_import_path(*path_tree.tree));
        }
        syn::UseTree::Rename(_) | syn::UseTree::Group(_) | syn::UseTree::Glob(_) => {
            panic!("Import path is not valid")
        }
    }
    path
}

// Resolve path to a mod, based on current module path and module_name.
fn resolve_module_path(context: &MacroContext, mod_name: &str) -> PathBuf {
    let mut mod_folder: PathBuf = context.src_filedir.to_path_buf();

    if context.mod_path.len() > 1 {
        for parent_mod in &context.mod_path[..context.mod_path.len() - 1] {
            mod_folder.push(parent_mod);
        }
    }

    let mod_path = mod_folder.join(format!("{mod_name}.rs"));
    println!("Probing path: {:?}", mod_path);
    if mod_path.exists() {
        return mod_path;
    } else {
        let mut mod_path = mod_folder.join(mod_name);
        mod_path.push("mod.rs");
        println!("Probing path: {:?}", mod_path);
        if mod_path.exists() {
            return mod_path;
        }
    }
    panic!(
        "Cannot find module '{}' relative to path {:?}, src_dir: {}",
        mod_name,
        mod_folder,
        context.src_filedir.display()
    );
}

#[cfg(test)]
mod test {
    use super::*;

    // Check that Visitor can find macro call
    #[test]
    fn test_simple_macro_call() {
        let mut found = false;
        let mut visitor = super::Visitor::new();
        let macro_call = |_, _| {
            found = true;
        };
        visitor.add_macro(vec!["rcss::file::css_module::css".to_owned()], macro_call);
        let input = syn::parse_str::<syn::Item>(
            r#"rcss::file::css_module::css! { .my-class { color: red; } }"#,
        )
        .unwrap();
        syn::visit::visit_item(&mut visitor, &input);
        drop(visitor);
        assert!(found)
    }

    #[test]
    fn test_macro_inside_fn() {
        let mut found = false;
        let mut visitor = super::Visitor::new();
        let macro_call = |_, _| {
            found = true;
        };
        visitor.add_macro(vec!["rcss::file::css_module::css".to_owned()], macro_call);
        let input = syn::parse_quote!(
            fn test() {
                rcss::file::css_module::css! { .my-class { color: red; } }
            }
        );
        syn::visit::visit_item(&mut visitor, &input);
        drop(visitor);
        assert!(found)
    }

    #[test]
    fn test_macro_inside_impl_fn() {
        let mut found = false;
        let mut visitor = super::Visitor::new();
        let macro_call = |_, _| {
            found = true;
        };
        visitor.add_macro(vec!["rcss::file::css_module::css".to_owned()], macro_call);
        let input = syn::parse_quote!(
            impl Test {
                fn test() {
                    rcss::file::css_module::css! { .my-class { color: red; } }
                }
            }
        );
        syn::visit::visit_file(&mut visitor, &input);
        drop(visitor);
        assert!(found)
    }

    #[test]
    fn test_macro_inside_fn_with_outer_and_inner_reimport() {
        let mut found = false;
        let mut visitor = super::Visitor::new();
        let macro_call = |_, _| {
            found = true;
        };
        visitor.add_macro(vec!["rcss::file::css_module::css".to_owned()], macro_call);
        let input = syn::parse_quote!(
            use rcss::file;
            fn test() {
                use file::css_module;
                file::css_module::css! { .my-class { color: red; } }
            }
        );
        syn::visit::visit_file(&mut visitor, &input);
        drop(visitor);
        assert!(found)
    }
    //check that import handle name;
    #[test]
    fn test_compare_use_by_name() {
        let path = "rcss::file::css_module::css_struct";
        let path = super::use_tree_from_str(path);
        let use_item: syn::ItemUse = syn::parse_quote! {
            use rcss::file;
        };

        let new_imports = compare_use_tree(path, use_item.tree);
        assert_eq!(new_imports, vec!["file::css_module::css_struct".to_owned()]);
    }

    #[test]
    fn test_compare_use_in_group() {
        let path = "rcss::file::css_module::css_struct";
        let path = super::use_tree_from_str(path);
        let use_item: syn::ItemUse = syn::parse_quote! {
            use rcss::file::{css_module, scoped};
        };

        let new_imports = compare_use_tree(path, use_item.tree);
        assert_eq!(new_imports, vec!["css_module::css_struct".to_owned()]);
    }

    #[test]
    fn test_compare_use_by_glob() {
        let path = "rcss::file::css_module::css_struct";
        let path = super::use_tree_from_str(path);
        let use_item: syn::ItemUse = syn::parse_quote! {
            use rcss::file::*;
        };

        let new_imports = compare_use_tree(path, use_item.tree);
        assert_eq!(new_imports, vec!["css_module::css_struct".to_owned()]);
    }
    #[test]
    fn test_compare_use_by_glob_in_group() {
        let path = "rcss::file::css_module::css_struct";
        let path = super::use_tree_from_str(path);
        let use_item: syn::ItemUse = syn::parse_quote! {
            use rcss::file::{*, scoped};
        };

        let new_imports = compare_use_tree(path, use_item.tree);
        assert_eq!(new_imports, vec!["css_module::css_struct".to_owned()]);
    }

    #[test]
    fn test_compare_deep_group_with_glob() {
        let path = "rcss::file::css_module::css_struct";
        let path = super::use_tree_from_str(path);
        let use_item: syn::ItemUse = syn::parse_quote! {
            use rcss::file::{*, css_module::{css, *}};
        };

        let new_imports = compare_use_tree(path, use_item.tree);
        assert_eq!(
            new_imports,
            vec!["css_module::css_struct".to_owned(), "css_struct".to_owned()]
        );
    }

    #[test]
    fn test_compare_with_rename() {
        let path = "rcss::file::css_module::css";
        let path = super::use_tree_from_str(path);
        let use_item: syn::ItemUse = syn::parse_quote! {
            use rcss::file::{*, css_module::{css as css2, *}};
        };

        let new_imports = compare_use_tree(path, use_item.tree);
        assert_eq!(
            new_imports,
            vec![
                "css_module::css".to_owned(),
                "css2".to_owned(),
                "css".to_owned()
            ]
        );
    }
}
