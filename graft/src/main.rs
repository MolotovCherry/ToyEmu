use std::{env, fs};

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

    let data = match assemble(&input_file) {
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
