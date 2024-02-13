//! Some main doc comment
//! This is main.rs entrypoint for bin target

mod outer;
mod outer_with_mod;
use macro_crate::macro_call;

fn main() {
    macro_call!("main", "main");
}

fn level1() {
    fn level2() {
        macro_call!("main", "level2");
    }
}

mod inner {
    use macro_crate::macro_call;
    fn some_test_fn() {
        macro_call!("main/inner", "some_test_fn");
        macro_call!("main/inner", "some_test_fn");
    }
}
