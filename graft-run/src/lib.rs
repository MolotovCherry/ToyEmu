use proc_macro::TokenStream;
use quote::quote;

#[proc_macro]
pub fn run(input: TokenStream) -> TokenStream {
    let mut source = String::new();
    let mut line = None;

    let mut iter = input.into_iter().peekable();
    while let Some(tree) = iter.next() {
        let span = tree.span();

        let line = line.get_or_insert(span.line());

        if *line < span.line() {
            source.push('\n');
            *line = span.line();
        }

        if let Some(mut s) = tree.span().source_text() {
            if iter
                .peek()
                .and_then(|t| t.span().source_text())
                .unwrap_or_default()
                != ","
            {
                s.push(' ');
            }

            source.push_str(&s);
        }
    }

    quote! {
        run(#source)
    }
    .into()
}
