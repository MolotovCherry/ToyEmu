use std::io::BufWriter;

use customasm::{asm, diagn, util};

static SPEC: &str = include_str!(r"../spec.asm");

#[derive(Debug, thiserror::Error)]
pub enum AsmError {
    #[error("failed to flush BufWriter")]
    BufWriter,
    #[error("non utf8 data found in error output")]
    NonUtf8,
    #[error("{0}")]
    Error(String),
    #[error("No output. assembled output is None")]
    NoOutput,
}

pub fn assemble(asm: &str) -> Result<Vec<u8>, AsmError> {
    // quite sad we have to leak this cause of the api
    #[rustfmt::skip]
    let input_file = format!(r#"
#include "spec.asm"

{asm}
    "#).leak().trim();

    let input_filename = "<input>.asm";

    let mut report = diagn::Report::new();
    let mut fileserver = util::FileServerReal::new();
    fileserver.add("spec.asm", SPEC);
    fileserver.add(input_filename, input_file);

    let opts = asm::AssemblyOptions::new();

    let assembly = asm::assemble(&mut report, &opts, &mut fileserver, &[input_filename]);

    let data = assembly.output.map(|o| o.format_binary());

    if report.has_errors() {
        let mut errors = BufWriter::new(Vec::new());
        report.print_all(&mut errors, &fileserver, true);

        let Ok(errors) = errors.into_inner() else {
            return Err(AsmError::BufWriter);
        };

        let Ok(errors) = String::from_utf8(errors) else {
            return Err(AsmError::NonUtf8);
        };

        return Err(AsmError::Error(errors));
    }

    data.ok_or(AsmError::NoOutput)
}
