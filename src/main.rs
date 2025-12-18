mod bf2c;
use bf2c::bf2c::bf2cify;
use std::fs::File;
use std::io::{Read, Write};
use std::env;
use std::process;

fn main() {
    let args: Vec<String> = env::args().collect();

    if args.len() != 3 {
        eprintln!("Usage: {} <input.bf> <output.c>", args[0]);
        process::exit(1);
    }

    let mut input = File::open(&args[1])
        .expect("Unable to open input file");
    
    let mut contents = String::new();
    input.read_to_string(&mut contents)
        .expect("Failed to read input file");

    let result = bf2cify(contents)
        .expect("failed to bf2cify");

    let mut output = File::create(&args[2])
        .expect("could not create output file");

    output.write_all(result.as_bytes()).unwrap();
}
