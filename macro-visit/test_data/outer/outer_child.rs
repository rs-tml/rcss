use super::*;

macro_crate::macro_call! {
    "*/outer/outer_child"
}

mod inside_outer {
    use super::*;
    mod inside_outer_child;
}
