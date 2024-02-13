mod outer;
mod outer_with_mod;


use macro_crate::macro_call;

#[path = "mod_with_path.rs"]
mod some_mod;

mod some_inner {
    use super::*;
    fn some_test_fn() {
        macro_call!("lib/some_inner", "some_test_fn");
        macro_call!("lib/some_inner", "some_test_fn");
    }
}
