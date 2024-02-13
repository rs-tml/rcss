use proc_macro2::{TokenStream, TokenTree};

#[test]
fn test_data() {
    let mut num_calls = 0;
    let macro_handler = |context: macro_visit::MacroContext, token_stream: TokenStream| {
        let mut iter_tt = token_stream.into_iter();
        let Some(TokenTree::Literal(path_to_mod)) = iter_tt.next() else {
            panic!("Expected literal");
        };
        iter_tt.next(); // skip comma
        let fn_call = if let Some(TokenTree::Literal(fn_call)) = iter_tt.next() {
            let fn_call = fn_call.to_string();
            // skip quotes
            let fn_call = fn_call.trim_matches('"');
            Some(fn_call.to_string())
        } else {
            None
        };
        num_calls += 1;

        let path_to_mod = path_to_mod.to_string();
        // skip quotes
        let path_to_mod = path_to_mod.trim_matches('"');
        let arg_path: Vec<&str> = path_to_mod.split('/').collect();
        let (arg_entrypoint, arg_mod_path) = arg_path.split_first().unwrap();
        assert_eq!(arg_mod_path, context.mod_path);
        if *arg_entrypoint != "*" {
            assert_eq!(arg_entrypoint, &context.entrypoint);
        }
        assert_eq! {
            context.fn_call_name, fn_call
        }
    };
    let mut visitor = macro_visit::Visitor::new();

    let macro_paths = vec![format!("macro_crate::macro_call")];
    visitor.add_macro(macro_paths, macro_handler);
    let path_to_manifest = env!("CARGO_MANIFEST_DIR");
    let entrypoint = format!("{}/test_data/main.rs", path_to_manifest);
    visitor.visit_project(entrypoint);

    let entrypoint = format!("{}/test_data/lib.rs", path_to_manifest);
    visitor.visit_project(entrypoint);
    drop(visitor);
    assert_eq!(num_calls, 17);
}
