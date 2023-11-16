//! Based on https://www.w3.org/TR/css-syntax-3 and https://developer.mozilla.org/en-US/docs/Web/CSS/Syntax
//! 
//! With `TokenStream`` limitations:
//! - no comments/new lines/whitespaces are represented in parsed result.
//! - non ascii characters allowed only inside strings, and limited to utf8.
//! - idents are not properly validated
//! - idents cannot end with "-", double minus ("--") is only allowed at start and single minus only in middle of ident.
//! So token stream like "-- foo - a" and "--foo-a" both parsed as "--foo-a" ident.
//! - Can't distinguish between ".foo.bar" and ".foo .bar" selector.
//! - URL, CDO, CDC tokens are not supported.
//! - Maybe something else.

use std::fmt::Display;

use primitive_tokens::{Value, Dimension, Number, DotNumber, Sign, Function, PercentToken, StringToken};
use proc_macro2::{Ident, TokenStream, TokenTree};
use syn::{ext::IdentExt, Token, LitInt, token::{Brace, Paren}, parse::{Parse, discouraged::Speculative, ParseStream}, spanned::Spanned};
use quote::TokenStreamExt;

mod primitive_tokens {
    use proc_macro2::{Literal, TokenStream};
    use syn::{Token, LitInt, token::{Paren, Brace}, parse::{Parse, discouraged::Speculative, ParseStream}, Error, spanned::Spanned};

    use crate::{CssIdent, parse_tts_until_block, parse_array_terminated};


    #[derive(Clone, Debug, syn_derive::Parse, syn_derive::ToTokens)]
    pub struct HashToken {
        pub token_hash: Token![#],
        pub ident: CssIdent,
    }
    
    // #[derive(Clone, Debug, syn_derive::Parse, syn_derive::ToTokens)]
    // enum HexDigits {
    //     #[parse(peek=LitInt)]
    //     Number(LitInt),
    //     Ident(Ident)
    // }
    // impl HexDigits {
    //     fn check_valid_str(var: &str) -> bool {
    //         var.matches(|c| {
    //              c >= 'a' && c<='z' ||
    //             c >= 'A' && c<='Z' ||
    //             c >= '0' && c<='9' ||
    //             c == '_' ||
    //             c == '_'
    //         }).next().is_some()
    //     }
    // }
    
    // Single or double quoted text.
    #[derive(Clone, Debug, syn_derive::ToTokens )]
    pub struct StringToken {
        pub literal: Literal,
    }
    impl Parse for StringToken {
        fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
            let literal: Literal = input.parse()?;
            let repr = literal.to_string();
            match repr.chars().nth(0) {
                Some('\'') | Some('\"') => {},
                _ => {
                    return Err(Error::new(literal.span(), "Expected string literal"))
                }
            }
            Ok(StringToken {
                literal
            })
        }
    }

    #[derive(Clone, Debug, syn_derive::Parse, syn_derive::ToTokens)]
    pub enum Sign {
        #[parse(peek = Token![-])]
        Minus(Token![-]),
        #[parse(peek = Token![+])]
        Plus(Token![+])
    }

    #[derive(Clone, Debug, syn_derive::ToTokens)]
    pub struct DotNumber {
        pub token_sign: Option<Sign>,
        pub token_dot: Token![.],
    }

    #[derive(Clone, Debug, syn_derive::ToTokens)]
    pub struct Number {
        pub sign_dot: Option<DotNumber>,
        // expect empty suffix, or suffix = 'eXX'
        // expect no dot in number when sign_dot::is_some
        pub number: LitInt,
        #[to_tokens(|_,_| ())]
        pub e_suffix: String,

    }
    impl Number {
        fn parse_any_suffix(input: syn::parse::ParseStream) -> syn::Result<Self> {
            let sign_dot = if (input.peek(Token![-]) || input.peek(Token![+])) &&
                input.peek2(Token![.]) {
                Some(DotNumber {
                    token_sign: Some(input.parse()?),
                    token_dot: input.parse()?,
                })
            } else if input.peek(Token![.]){
                Some(DotNumber {
                    token_sign: None,
                    token_dot: input.parse()?
                })
            } else {
                None
            };
            let number: LitInt = input.parse()?;
            if sign_dot.is_some() && number.to_string().starts_with(".") {
                return Err(
                    Error::new(sign_dot.unwrap().span(), "More than one dot in number")
                )
            }
            let suffix = number.suffix();
            let mut suffix_iter = suffix.chars();
            let mut e_suffix = String::new();
            if let Some(c) = suffix_iter.next() {
                if c == 'e' || c == 'E' {
                    e_suffix.push(c);
                    for c in suffix_iter {
                        if c >= '0' && c <= '9' {
                            e_suffix.push(c);
                        } else {
                            break
                        }
                    }
                }
            }
            Ok(Number {
                sign_dot,
                number,
                e_suffix,
            })
        }
        // suffix after eXX
        fn suffix(&self) -> &str {
            &self.number.suffix()[self.e_suffix.len()..]
        }
    }

    impl Parse for Number {
        fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
           let num = Self::parse_any_suffix(input)?;
           if !num.suffix().is_empty() {
                return Err(
                    Error::new(num.span(), "No suffix in number was expected")
                )
           }
           Ok(num)
        }
    }
    #[derive(Clone, Debug, syn_derive::ToTokens)]
    pub struct Dimension {
        pub number: Number,
        // copied from number.suffix
        pub dimension: String,
    }

    impl Parse for Dimension {
        fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
            let number = Number::parse_any_suffix(input)?;
            let dimension = number.suffix().to_string();
            Ok(
                Dimension { number, dimension }
            )
        }
    }

    #[derive(Clone, Debug, syn_derive::Parse, syn_derive::ToTokens)]
    pub struct PercentToken {
        pub number: Number,
        pub percentage: Token![%],
    }

    #[derive(Clone, Debug, syn_derive::ToTokens)]
    pub enum Value {
        Function(Function),
        Number(Number),
        Percent(PercentToken),
        Dimension(Dimension),
        Ident(CssIdent),
        String(StringToken),
        Verbatim(TokenStream),
    }

    impl Value  {
        pub fn parse_array_until_block_or_colon(input: ParseStream) -> syn::Result<Vec<Self>> {
            let result = parse_array_terminated(input, 
                |p| p.peek(Brace) || p.peek(Token![;]) || p.peek(Token![:]  ))?;

            let mut new_result = vec![];
            for val in result {
                if let Value::Ident(ident) = val {
                    for ident in ident.split_ident()? {
                        new_result.push(Value::Ident(ident))
                    }
                } else {
                    new_result.push(val)
                }
            }
            return Ok(new_result)
            
        }
    }
    impl Parse for Value {
        fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
            let fork = input.fork();
            let value = if let Ok(number) = fork.parse() {
                if input.peek(Token![%]) {
                    Some(Value::Percent(PercentToken {
                        number,
                        percentage: fork.parse().expect("checked")
                    }))
                } else {
                    if !number.suffix().is_empty() {
                        Some(Value::Number(number))
                    } else {
                        let dimension = number.suffix().to_string();
                        Some(Value::Dimension(Dimension {
                            number, dimension
                        }))
                    }
                    
                }
            } else { None };

            if let Some(value) = value {
                input.advance_to(&fork);
                return Ok(value)
            }
            let fork = input.fork();
            if let Ok(v) = fork.parse::<StringToken>() {
                input.advance_to(&fork);
                return Ok(Value::String(v))
            }
            let fork = input.fork();
            if let Ok(ident) =  fork.parse::<CssIdent>() {
                if !fork.peek(Paren) {
                    input.advance_to(&fork);
                    return Ok(Value::Ident(ident))
                } else {
                    if let Ok(func) = input.parse::<Function>(){
                        return Ok(Value::Function(func))
                    }
                }
            }
            Ok(Value::Verbatim(parse_tts_until_block(input)?))
            
        }
    }

    use quote::TokenStreamExt;
    #[derive(Clone, Debug, syn_derive::Parse, syn_derive::ToTokens)]
    pub struct Function {
        pub name: CssIdent,
        #[syn(parenthesized)]
        pub paren_token: Paren,
        #[syn(in = paren_token)]
        #[to_tokens(|tokens, val| tokens.append_all(val))]
        #[parse(Value::parse_array_until_block_or_colon)]
        pub args: Vec<Value>
    }
}


#[derive(Clone, Debug, syn_derive::Parse, syn_derive::ToTokens)]
pub enum IdentFragment {
    #[parse(peek = Ident::peek_any)]
    Ident(#[parse(IdentExt::parse_any)]Ident),
    #[parse(peek = Token![-])]
    Minus(Token![-]),
    Digit(LitInt),
}

#[derive(Clone, Default, Debug, syn_derive::Parse, syn_derive::ToTokens)]
pub struct CssIdent {
    #[to_tokens(|tokens, val| tokens.append_all(val))]
    /// TODO: Better error in case of invalid token
    #[parse(parse_array_ident)]
    fragments: Vec<IdentFragment>
}

impl CssIdent {
    // Is canonical ident for stable rust
    // returns false if no fragments are present, or latest fragment is minus. 
    fn is_canonical_stable(&self) -> bool {
        !matches!(self.fragments.last(), None | Some(IdentFragment::Minus(_)))
    }
    // Try to split ident, if there is "--" is present
    fn split_ident(self) -> syn::Result<Vec<Self>>  {
        let mut idents = Vec::new();
        let mut ident = Self::default();

        // TODO: Replace with Span::source_text when Span::join will be ready.
        // stable rust hack
        // dbg!(&fragments);

        for fragments in self.fragments.windows(2) {
            match fragments {
                [IdentFragment::Minus(_), IdentFragment::Minus(_)] if ident.is_canonical_stable()  => {
                    idents.push(ident);
                    ident = Self::default();
                }
                _ => {}
            }
            ident.fragments.push(fragments[0].clone())
        }
        if let Some(fragment) = self.fragments.last() {

            ident.fragments.push(fragment.clone())
        }
        if !ident.is_canonical_stable() {
            return Err(syn::Error::new(ident.span(), "Ident cannot end with '-' "))
        }
        idents.push(ident);

        return Ok(idents)
    }

    fn parse_array(input: ParseStream) -> syn::Result<Vec<Self>> {
        let result: Vec<IdentFragment> = parse_array_ident(input)?;
        let ident = Self {
            fragments: result
        };
        ident.split_ident()

    }
}

fn parse_array_terminated<T: Parse>(input: ParseStream, terminate: impl Fn(ParseStream) -> bool) -> syn::Result<Vec<T>> {
    let mut fragments = Vec::new();
    while !input.is_empty() {
        if terminate(&input.fork()) {
            break
        }
        let fragment = input.parse()?;
        fragments.push(fragment);
    }
    Ok(fragments)
}


fn parse_array_ident<T: Parse>(input: ParseStream) -> syn::Result<Vec<T>> {
    parse_array_terminated(input, 
        |p| p.peek(Paren) || p.peek(Brace) || p.peek(Token![;]) || p.peek(Token![:]  ))
}
fn parse_tts_until_block(input: ParseStream) -> syn::Result<TokenStream> {
    let mut tokens = TokenStream::new();

    let result = parse_array_terminated::<TokenTree>(input, 
        |p| p.peek(Brace) || p.peek(Token![;] ))?;
    tokens.append_all(result);
    Ok(tokens)
}
fn parse_to_eof<T: Parse>(input: ParseStream) -> syn::Result<Vec<T>> {
    parse_array_terminated(input, 
        |p| p.is_empty() )
}

#[derive(Clone, Debug, syn_derive::Parse, syn_derive::ToTokens)]
pub struct CssValue {
    #[parse(parse_tts_until_block)]
    pub val: TokenStream,
}

#[derive(Clone, Debug, syn_derive::Parse, syn_derive::ToTokens)]
pub struct Property {
    pub key: CssIdent,
    pub token_colon: Token![:],
    #[to_tokens(|tokens, val| tokens.append_all(val))]
    #[parse(Value::parse_array_until_block_or_colon)]
    pub value: Vec<Value>,
    // Use semi inside property, because we also need to support nested blocks
    // Optional semi is only for last property
    pub token_semi: Option<Token![;]>,
}

#[derive(Clone, Debug, syn_derive::Parse, syn_derive::ToTokens)]
pub struct CssBlock {
    #[syn(braced)]
    pub brace_token: Brace,
    #[syn(in = brace_token)]
    #[to_tokens(|tokens, val| tokens.append_all(val))]
    #[parse(parse_to_eof)]
    pub contents: Vec<Property>,
}

#[derive(Clone, Debug, syn_derive::Parse, syn_derive::ToTokens)]
pub enum BlockOrSemicolon {
    #[parse(peek = Token![;])]
    Semicolon(Token![;]),
    Block(CssBlock),
}

#[derive(Clone, Debug, syn_derive::Parse, syn_derive::ToTokens)]
pub enum SelectorOp {
    #[parse(peek = Token![+])]
    NextSibling(Token![+]),
    #[parse(peek = Token![>])]
    Child(Token![>]),
    #[parse(peek = Token![||])]
    Column(Token![||]),
    #[parse(peek = Token![~])]
    SubSequent(Token![~]),
    #[parse(peek = Token![|])]
    Namespace(Token![|]),
    #[parse(peek = Token![,])]
    List(Token![,]),
    Descendant(/* whitespace */),
}

#[derive(Clone, Debug, syn_derive::Parse, syn_derive::ToTokens)]
pub struct SelectorArgs {
    #[syn(parenthesized)]
    paren_token: Paren,
    #[syn(in= paren_token)]
    #[to_tokens(|tokens, val| tokens.append_all(val))]
    #[parse(SelectorsFragment::parse_array_until_block)]
    data: Vec<SelectorsFragment>
}
impl SelectorArgs {
    fn parse_option(input: ParseStream) -> syn::Result<Option<Self>> {
        if input.peek(Paren) {
            Ok(Some(input.parse()?))
        } else {
            Ok(None)
        }
    }
}
#[derive(Clone, Debug, syn_derive::Parse, syn_derive::ToTokens)]
pub enum Selector {
    #[parse(peek = Token![&])]
    Nesting (Token![&]),
    #[parse(peek = Token![*])]
    Universal(Token![*]),
    #[parse(peek = Token![.])]
    Class {
        token_dot: Token![.],
        ident: CssIdent
    },
    #[parse(peek = Token![#])]
    Id {
        token_hash: Token![#],
        ident: CssIdent
    },
    #[parse(peek = Token![::])]
    PseudoElement {
        token_path: Token![::],
        ident: CssIdent,
        #[parse(SelectorArgs::parse_option)]
        args: Option<SelectorArgs>
        
    },
    #[parse(peek = Token![:])]
    PseudoClass {
        token_colon: Token![:],
        ident: CssIdent,
        #[parse(SelectorArgs::parse_option)]
        args: Option<SelectorArgs>
    },
    Type(CssIdent),
}

#[derive(Clone, Debug, syn_derive::ToTokens)]
pub enum SelectorsFragment {
    Selector(Selector),
    Operation(SelectorOp)
} 
impl SelectorsFragment {
    fn add_spaces(fragments: Vec<Self>) -> Vec<Self> {
        // dbg!(&fragments);

        dbg!(&fragments[0].span().end());
        let mut new_fragments = vec![];
        for chunk in fragments.windows(2) {

            dbg!(&chunk[0].span().end());
            new_fragments.push(chunk[0].clone());
            match chunk {
                [SelectorsFragment::Selector(_), SelectorsFragment::Selector(_)] => {
                    new_fragments.push(SelectorsFragment::Operation(SelectorOp::Descendant()))
                }
                _ => {}
            }
        }
        if let Some(v) = fragments.last() {
            new_fragments.push(v.clone())
        }
        new_fragments
    }
    fn parse_array_until_block(input: ParseStream) -> syn::Result<Vec<Self>> {
        let result = parse_array_terminated(input, 
            |p| p.peek(Brace) || p.peek(Token![;] ))?;
        let mut new_results = vec![];
        // add 
        for res in result {
            match res {
                SelectorsFragment::Selector(Selector::Class { token_dot, ident }) => {
                    let idents = ident.clone().split_ident()?;
                    let (ident, rest) = idents.split_first().expect("non empty");
                    new_results.push(SelectorsFragment::Selector(Selector::Class { token_dot, ident:ident.clone() }));
                    for ident in rest {
                        new_results.push(SelectorsFragment::Selector(Selector::Type(ident.clone())));
                    }
                }
                SelectorsFragment::Selector(Selector::Id { token_hash, ident }) => {
                    let idents = ident.clone().split_ident()?;
                    let (ident, rest) = idents.split_first().expect("non empty");
                    new_results.push(SelectorsFragment::Selector(Selector::Id { token_hash, ident:ident.clone() }));
                    for ident in rest {
                        new_results.push(SelectorsFragment::Selector(Selector::Type(ident.clone())));
                    }
                }
                SelectorsFragment::Selector(Selector::PseudoElement { token_path, ident, args: None }) => {
                    let idents = ident.clone().split_ident()?;
                    let (ident, rest) = idents.split_first().expect("non empty");
                    new_results.push(SelectorsFragment::Selector(Selector::PseudoElement { token_path, ident:ident.clone(), args: None }));
                    for ident in rest {
                        new_results.push(SelectorsFragment::Selector(Selector::Type(ident.clone())));
                    }
                }
                SelectorsFragment::Selector(Selector::PseudoClass { token_colon, ident, args: None }) => {
                    let idents = ident.clone().split_ident()?;
                    let (ident, rest) = idents.split_first().expect("non empty");
                    new_results.push(SelectorsFragment::Selector(Selector::PseudoClass { token_colon, ident:ident.clone(), args: None }));
                    for ident in rest {
                        new_results.push(SelectorsFragment::Selector(Selector::Type(ident.clone())));
                    }
                }
                SelectorsFragment::Selector(Selector::Type (ident)) => {
                    let idents = ident.clone().split_ident()?;
                    let (ident, rest) = idents.split_first().expect("non empty");
                    new_results.push(SelectorsFragment::Selector(Selector::Type ( ident.clone() )));
                    for ident in rest {
                        new_results.push(SelectorsFragment::Selector(Selector::Type(ident.clone())));
                    }
                }    
                res => new_results.push(res)
            }
        }
        
        Ok(Self::add_spaces(new_results))
    }
}

impl Parse for SelectorsFragment {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let fork = input.fork();
        if let Ok(selector) = fork.parse() {
            input.advance_to(&fork);
            return Ok(SelectorsFragment::Selector(selector))
        } else {
            Ok(SelectorsFragment::Operation(input.parse()?))
        }
    }
}


#[derive(Clone, Debug, syn_derive::Parse, syn_derive::ToTokens)]
pub struct QRule {
    #[to_tokens(|tokens, val| tokens.append_all(val))]
    #[parse(SelectorsFragment::parse_array_until_block)]
    pub selectors: Vec<SelectorsFragment>,
    pub block: CssBlock,
}
/// It's some kind of extensions for css.
/// 
#[derive(Clone, Debug, syn_derive::Parse, syn_derive::ToTokens)]
pub struct AtRule {
    pub token_at: Token![@],
    #[parse(parse_tts_until_block)]
    pub arguments: TokenStream,
    pub block: CssBlock,
}

#[derive(Clone, Debug, syn_derive::Parse, syn_derive::ToTokens)]
pub enum Rule {
    #[parse(peek = Token![@])]
    AtRule(AtRule),
    // Qualified rule
    QRule(QRule)
}

#[derive(Clone, Debug, syn_derive::Parse, syn_derive::ToTokens)]
pub struct StyleSheet {
    #[to_tokens(|tokens, val| tokens.append_all(val))]
    #[parse(parse_to_eof)]
    rules: Vec<Rule>,
}

pub fn parse2(tokens: TokenStream) -> syn::Result<StyleSheet> {
    syn::parse2(tokens)
}

pub fn parse2_with_macro_call(tokens: TokenStream) -> syn::Result<StyleSheet> {
    let mut tokens = tokens.into_iter();
    tokens.next(); // css
    tokens.next(); // ! 
    // { .. }
    if let Some(TokenTree::Group(g)) = tokens.next() {
        syn::parse2(g.stream())
    } else {
        panic!()
    }
}



impl Display for CssIdent {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        
        for fragment in &self.fragments {
            write!(f, "{}", fragment)?
        }
        Ok(())
    }
}

impl Display for IdentFragment {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            IdentFragment::Digit(d) => {
                write!(f, "{}", d)?
            },
            IdentFragment::Minus(_) => {
                write!(f, "-")?
            },
            IdentFragment::Ident(i) => {
                write!(f, "{}", i)?
            }
        }
        Ok(())
    }
}

impl Display for Dimension {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.number)
    }
}
impl Display for Sign {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Sign::Minus(_) => write!(f, "-"),
            Sign::Plus(_) => write!(f, "+"),
        }
    }
}
impl Display for DotNumber {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
       if let Some(sign) = &self.token_sign {
            write!(f, "{}", sign)?;
       }
       write!(f, ".")
    }
}

impl Display for Number {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if let Some(e) = &self.sign_dot {
            write!(f, "{}", e)?
        }
        write!(f, "{}", self.number)
        
    }
}

impl Display for Function {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}(", self.name)?;
        for arg in &self.args {
            write!(f, "{}", arg)?;
        }
        write!(f, ")")
    }
}

impl Display for PercentToken {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}%", self.number)
    }
}
impl Display for StringToken {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.literal)
    }
}
impl Display for Value {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Value::Dimension(d) => d.fmt(f),
            Value::Function(fnc) => fnc.fmt(f),
            Value::Ident(i) => i.fmt(f),
            Value::Number(n) => n.fmt(f),
            Value::Percent(p) => p.fmt(f),
            Value::String(p) => p.fmt(f),
            Value::Verbatim(v) => v.fmt(f)
        }
    }
}
impl Display for Property {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}: ", self.key)?;
        let mut first = true;
        for val in &self.value {
            if !first {
                write!(f, " ")?;
            }
            write!(f, "{}", val)?;
            first = false;
        }
        write!(f, ";")
    }
}

impl Display for CssBlock {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "{{")?;
        for element in &self.contents {
            writeln!(f, "{}", element)?
        }
        writeln!(f, "}}")
    }
}

impl Display for AtRule {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "@{} {}", self.arguments, self.block)
    }
}

impl Display for SelectorOp {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SelectorOp::Child(_) => write!(f, ">"),
            SelectorOp::Column(_) => write!(f, "||"),
            SelectorOp::List(_) => write!(f, ","),
            SelectorOp::Namespace(_) => write!(f, "|"),
            SelectorOp::NextSibling(_) => write!(f, "+"),
            SelectorOp::SubSequent(_) => write!(f, "~"),
            SelectorOp::Descendant() => write!(f, " "),
        }
    }
}

impl Display for Selector {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Selector::Class { ident, .. } => write!(f, ".{}", ident),
            Selector::Id { ident, .. } => write!(f, "#{}", ident),
            Selector::PseudoClass {  ident, .. } => write!(f, ":{}", ident),
            Selector::PseudoElement {  ident, .. } => write!(f, "::{}", ident),
            Selector::Nesting(_) => write!(f, "&"),
            Selector::Type(ident) => write!(f, "{}", ident),
            Selector::Universal(_) => write!(f, "*")
        }
    }
}

impl Display for SelectorsFragment {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SelectorsFragment::Operation(o) => o.fmt(f),
            SelectorsFragment::Selector(s) => s.fmt(f),
        }
    }
}
impl Display for QRule {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        for selector in &self.selectors{
            write!(f, "{}", selector)?;
        }
        write!(f, " {}", self.block)
    }
}

impl Display for Rule {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Rule::AtRule(a) => a.fmt(f),
            Rule::QRule(b) => b.fmt(f)
        }
    }
}
impl Display for StyleSheet {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        for rule in &self.rules {
            write!(f, "{}", rule)?
        }
        Ok(())
    }
}

#[cfg(test)]
mod test {
    use std::str::FromStr;

    use proc_macro2::TokenStream;
    use quote::quote;
    use syn::parse::ParseStream;

    use crate::{parse2, CssIdent, Property, primitive_tokens::Value};

    #[test] 
    fn test_ident() {
        let ident: CssIdent = syn::parse2(quote!{
            foo
        }).unwrap();

        assert_eq!(ident.to_string(), "foo");
        let ident: CssIdent = syn::parse2(quote!{
            foo-bar
        }).unwrap();

        assert_eq!(ident.to_string(), "foo-bar");
        let ident: CssIdent = syn::parse2(quote!{
            foo- bar
        }).unwrap();

        assert_eq!(ident.to_string(), "foo-bar");
        let ident: CssIdent = syn::parse2(quote!{
            --foo_bar
        }).unwrap();

        assert_eq!(ident.to_string(), "--foo_bar");

        use syn::parse::Parser as _;
        // let parser = move |input: ParseStream| Ok(self.parse_syn_stream(input));

        let parser = |input: ParseStream| {
            CssIdent::parse_array(input)
        };
        
        let idents = parser.parse2(quote!{
            --foo_--bar
        }).unwrap();
        
        assert_eq!(idents[0].to_string(), "--foo_");
        assert_eq!(idents[1].to_string(), "--bar");

        let idents = parser.parse2(quote!{
            --foo-----bar
        }).unwrap();
        
        assert_eq!(idents[0].to_string(), "--foo");
        assert_eq!(idents[1].to_string(), "-----bar");

        let idents = parser.parse2(quote!{
            --foo-bar
        }).unwrap();
        
        assert_eq!(idents[0].to_string(), "--foo-bar");
        //check that ident can't end with double minus
        let _ = parser.parse2(quote!{
            --foo--
        }).unwrap_err();

    }

    #[test] 
    fn test_property() {
        let property: Property = syn::parse2(quote!{
            foo: 32px;
        }).unwrap();

        assert_eq!(property.to_string(), "foo: 32px;");

    }

    #[test]
    fn test_values() {
        let value: Value = syn::parse2(quote!{
            foo
        }).unwrap();

        assert!(matches!(value, Value::Ident(_)));
        assert_eq!(value.to_string(), "foo");


        let value: Value = syn::parse2(quote!{
            --foo-bar
        }).unwrap();

        assert!(matches!(value, Value::Ident(_)));
        assert_eq!(value.to_string(), "--foo-bar");


        let value: Value = syn::parse2(quote!{
            var(--foo)
        }).unwrap();

        assert!(matches!(value, Value::Function(_)));
        assert_eq!(value.to_string(), "var(--foo)");
    }

    #[test]
    fn simple_parse() {
        // TODO: rewrite display implementation
        let css_str = 
r#".bar {
foo: 32;
}
div {
foo: "x";
}
#id {
x: y;
}
"#;
        let css = TokenStream::from_str(css_str).unwrap();
        let style = parse2(css).unwrap();

        assert_eq!(style.to_string(), css_str);
    }


    #[test] 
    fn stylers_example() {
        let css_str = 
        r##"button {
            background-color: green;
            border-radius: 8px;
            border-style: none;
            box-sizing: border-box;
            color: yellow;
            cursor: pointer;
            display: inline-block;
            font-family: r#"Haas Grot Text R Web"#, r#"Helvetica Neue"#, Helvetica, Arial, sans-serif;
            font-size: 14px;
            font-weight: 500;
            height: 40px;
            line-height: 20px;
            list-style: none;
            margin: 0;
            outline: none;
            padding: 10px 16px;
            position: relative;
            text-align: center;
            text-decoration: none;
            transition: color 100ms;
            vertical-align: baseline;
            user-select: none;
            -webkit-user-select: none;
        }
        button:hover {
            background-color: yellow;
            color: green;
        }
        "##;
                let css = TokenStream::from_str(css_str).unwrap();
                let style = parse2(css).unwrap();
                dbg!(&style);
        
    }
}