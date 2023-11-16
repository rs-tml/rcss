use tested_macro::css;


#[test]
fn basic() {
    css!{
        button:is(foo) {
            -webkit-user-select: none;
        }
        button:hover {
            background-color: yellow;
            color: green;
        }
    }
}