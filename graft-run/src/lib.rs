use proc_macro::TokenStream;
use quote::quote;

#[proc_macro]
pub fn run(input: TokenStream) -> TokenStream {
    let mut source = String::new();
    let mut line = None;

    let mut start_col = 0;
    let mut first = true;
    let mut new_line = true;
    let mut last_col = 0;
    for tree in input {
        let span = tree.span();

        if first {
            start_col = span.column();
            last_col = span.end().column();
            first = false;
        }

        let line = line.get_or_insert(span.line());

        if *line < span.line() {
            let how_many = span.line() - *line;
            let s = "\n".repeat(how_many);
            source.push_str(&s);
            *line = span.line();
            new_line = true;
        }

        if let Some(s) = span.source_text() {
            let how_many = if new_line {
                let column = span.column();
                column - start_col
            } else if last_col < span.column() {
                span.column() - last_col
            } else {
                0
            };

            let spaces = " ".repeat(how_many);

            source.push_str(&format!("{spaces}{s}"));
        }

        last_col = span.end().column();
        new_line = false;
    }

    quote! {
        run(#source)
    }
    .into()
}
