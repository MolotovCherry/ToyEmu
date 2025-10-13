use proc_macro::TokenStream;
use quote::quote;

#[proc_macro]
pub fn run(input: TokenStream) -> TokenStream {
    let mut source = String::new();
    let mut line = None;

    let mut new = true;
    let mut prev_was_dot = false;
    let mut iter = input.into_iter().peekable();
    while let Some(tree) = iter.next() {
        let span = tree.span();

        let line = line.get_or_insert(span.line());

        if *line < span.line() {
            source.push('\n');
            *line = span.line();
            new = true;
        }

        if let Some(mut s) = tree.span().source_text() {
            let next = iter
                .peek()
                .and_then(|t| t.span().source_text())
                .unwrap_or_default();

            #[allow(clippy::match_like_matches_macro)]
            let add_next = match &*next {
                ";" => true,
                _ => false,
            };

            #[allow(clippy::match_like_matches_macro)]
            let skip_next = match &*next {
                "." => true,
                _ => false,
            };

            if (s.contains([',', ';']) || add_next || new || prev_was_dot) && !skip_next {
                s.push(' ');
                prev_was_dot = false;
            }

            if s == "." {
                prev_was_dot = true;
            }

            source.push_str(&s);
        }

        new = false;
    }

    quote! {
        run(#source)
    }
    .into()
}
