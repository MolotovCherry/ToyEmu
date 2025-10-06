mod cpu;
mod emulator;
mod instruction;
mod memory;

pub type BitSize = u32;

fn main() {
    let a = vec![1, 2, 3, 4];
    println!("{a:?}");
}
