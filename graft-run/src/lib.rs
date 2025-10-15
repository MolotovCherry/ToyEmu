use proc_macro::TokenStream;
use quote::{ToTokens, quote};
use syn::{ExprClosure, Ident, Token, parse::Parse, parse_macro_input};

fn tokens_to_source(input: TokenStream) -> String {
    let mut source = String::new();
    let mut line = None;

    let mut new_line = true;
    let mut last_col = 0;
    for tree in input {
        let span = tree.span();

        let line = line.get_or_insert(span.line());

        if *line < span.line() {
            let how_many = span.line() - *line;
            let s = "\n".repeat(how_many);
            source.push_str(&s);
            *line = span.line();
            last_col = 0;
            new_line = true;
        }

        if let Some(s) = span.source_text() {
            let how_many = if new_line {
                new_line = false;
                span.column().saturating_sub(1)
            } else {
                span.column() - last_col
            };

            let spaces = " ".repeat(how_many);
            source.push_str(&spaces);
            source.push_str(&s);
        }

        last_col = span.end().column();
    }

    source
}

#[proc_macro]
pub fn try_run(input: TokenStream) -> TokenStream {
    let source = tokens_to_source(input);

    quote! {
        try_run(#source)
    }
    .into()
}

#[proc_macro]
pub fn run(input: TokenStream) -> TokenStream {
    let source = tokens_to_source(input);

    quote! {
        try_run(#source).unwrap()
    }
    .into()
}

#[allow(clippy::large_enum_variant)]
enum ClosureOrIdent {
    Closure(ExprClosure),
    Ident(Ident),
}

impl ToTokens for ClosureOrIdent {
    fn to_tokens(&self, tokens: &mut proc_macro2::TokenStream) {
        match self {
            ClosureOrIdent::Closure(expr_closure) => expr_closure.to_tokens(tokens),
            ClosureOrIdent::Ident(ident) => ident.to_tokens(tokens),
        }
    }
}

struct RunWithInput {
    expr: ClosureOrIdent,
    rest: proc_macro2::TokenStream,
}

impl Parse for RunWithInput {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        let expr = input
            .parse::<ExprClosure>()
            .map(ClosureOrIdent::Closure)
            .or_else(|_| input.parse::<Ident>().map(ClosureOrIdent::Ident))?;

        let _comma: Token![,] = input.parse()?;
        let rest: proc_macro2::TokenStream = input.parse()?;
        Ok(RunWithInput { expr, rest })
    }
}

#[proc_macro]
pub fn try_run_with(input: TokenStream) -> TokenStream {
    let RunWithInput { expr, rest } = parse_macro_input!(input as RunWithInput);
    let source = tokens_to_source(rest.into());

    quote! {
        try_run_with(#expr, #source)
    }
    .into()
}

#[proc_macro]
pub fn run_with(input: TokenStream) -> TokenStream {
    let RunWithInput { expr, rest } = parse_macro_input!(input as RunWithInput);
    let source = tokens_to_source(rest.into());

    quote! {
        try_run_with(#expr, #source).unwrap()
    }
    .into()
}
