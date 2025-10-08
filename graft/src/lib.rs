use std::{fmt::Debug, io::BufWriter};

use customasm::{asm, diagn, util};

static SPEC: &str = include_str!(r"../spec.asm");

#[derive(thiserror::Error)]
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

impl Debug for AsmError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{self}")
    }
}

pub fn assemble(filename: &str, asm: &str, error_colors: bool) -> Result<Vec<u8>, AsmError> {
    // quite sad we have to leak this cause of the api
    #[rustfmt::skip]
    let input_file = format!(r#"
#include "spec.asm"

{asm}
    "#).leak().trim();

    let mut report = diagn::Report::new();
    let mut fileserver = util::FileServerReal::new();
    fileserver.add("spec.asm", SPEC);
    fileserver.add(filename, input_file);

    let opts = asm::AssemblyOptions::new();

    let assembly = asm::assemble(&mut report, &opts, &mut fileserver, &[filename]);

    let data = assembly.output.map(|o| o.format_binary());

    if report.has_errors() {
        let mut errors = BufWriter::new(Vec::new());
        report.print_all(&mut errors, &fileserver, error_colors);

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
