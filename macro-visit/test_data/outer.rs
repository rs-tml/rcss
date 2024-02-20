use macro_crate::macro_call;
mod outer_child;

fn outer1() {
    macro_call!("*/outer", "outer1");
}
