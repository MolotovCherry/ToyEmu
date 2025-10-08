use std::{env, fs, path::Path};

use graft::assemble;

fn main() {
    let Some(input_file) = env::args().nth(1) else {
        println!("symasm <input.asm> <output>");
        return;
    };

    let Some(output_file) = env::args().nth(2) else {
        println!("symasm <input.asm> <output>");
        return;
    };

    let input_file = match fs::read_to_string(&input_file) {
        Ok(v) => v,
        Err(e) => {
            eprintln!("failed to read input file:\n{e}");
            return;
        }
    };

    let filename = Path::new(&input_file).file_name();
    let Some(filename) = filename else {
        eprintln!("failed to get input filename. did you input the correct path?");
        return;
    };
    let filename = &*filename.to_string_lossy();

    let data = match assemble(filename, &input_file, true) {
        Ok(bin) => bin,
        Err(e) => {
            eprintln!("{e}");
            return;
        }
    };

    if let Err(e) = fs::write(&output_file, data) {
        eprintln!("failed to save output file:\n{e}");
        return;
    }

    println!("saved to {output_file}");
}
