use std::{env, fs, io::BufWriter, path::Path};

use customasm::{asm, diagn, util};

static SPEC: &str = include_str!(r"../spec.asm");

fn main() {
    let Some(input_file) = env::args().nth(1) else {
        println!("symasm <input.asm> <output>");
        return;
    };

    let Some(output_file) = env::args().nth(2) else {
        println!("symasm <input.asm> <output>");
        return;
    };

    let input_filename = Path::new(&input_file).file_name();
    let Some(input_filename) = input_filename else {
        eprintln!("failed to get input filename. did you input the correct path?");
        return;
    };
    let input_filename = &*input_filename.to_string_lossy();

    let input_file = match fs::read_to_string(&input_file) {
        Ok(v) => v,
        Err(e) => {
            eprintln!("failed to read input file:\n{e}");
            return;
        }
    };

    // quite sad we have to leak this cause of the api
    #[rustfmt::skip]
    let input_file = format!(r#"
#include "spec.asm"

{input_file}
    "#).leak().trim();

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
            eprintln!("failed to flush error BufWriter");
            return;
        };

        let Ok(errors) = str::from_utf8(&errors) else {
            eprintln!("error wasn't utf8?\n{errors:?}");
            return;
        };

        eprint!("{}", errors.trim());
        return;
    }

    if let Some(data) = data {
        if let Err(e) = fs::write(&output_file, data) {
            eprintln!("failed to save output file:\n{e}");
            return;
        }
    } else {
        eprintln!("nothing was saved to file");
        return;
    }

    println!("saved to {output_file}");
}
