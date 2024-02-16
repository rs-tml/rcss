use rcss::{css, extend::ScopeChain, ScopeCommon};

css! {
    @rcss(pub struct Button);
    /* CSS */
    .button {
        appearance: none;
        background-color: #FAFBFC;
        border: 1px solid rgba(27, 31, 35, 0.15);
        border-radius: 6px;
        box-shadow: rgba(27, 31, 35, 0.04) 0 1px 0, rgba(255, 255, 255, 0.25) 0 1px 0 inset;
        box-sizing: border-box;
        color: #24292F;
        cursor: pointer;
        display: inline-block;
        font-family: -apple-system, system-ui, "Segoe UI", Helvetica, Arial, sans-serif, "Apple Color Emoji", "Segoe UI Emoji";
        font-size: 14px;
        font-weight: 500;
        line-height: 20px;
        list-style: none;
        padding: 6px 16px;
        position: relative;
        transition: background-color 0.2s cubic-bezier(0.3, 0, 0.5, 1);

    }

    .button:hover {
        background-color: #F3F4F6;
        text-decoration: none;
        transition-duration: 0.1s;
    }

    .button:disabled {
        background-color: #FAFBFC;
        border-color: rgba(27, 31, 35, 0.15);
        color: #959DA5;
        cursor: default;
    }

    .button:active {
        background-color: #EDEFF2;
        box-shadow: rgba(225, 228, 232, 0.2) 0 1px 0 inset;
        transition: none 0s;
    }
}
use crate::Html;

pub fn default_button(text: String) -> Html {
    button::<Button>(text, None)
}

pub fn button<CSS>(text: String, css: Option<CSS>) -> Html
where
    CSS: ScopeCommon + ScopeChain<Root = Button>,
{
    let css = css.map(ScopeChain::into_root).unwrap_or_default();

    format!(
        "<button 
            class='{class}'
            role=\"button\">
        {text}
        </button>",
        class = css.button
    )
}
