mod bf2c;
use bf2c::bf2c::bf2cify;
use std::fs::File;
use std::io::{Read, Write};
use std::env;
use std::process;

fn main() {
    let args: Vec<String> = env::args().collect();

    let input_file_path = args.get(1).map(|s| s.as_str()).unwrap_or("src/bf.bf");
    let output_file_path = args.get(2).map(|s| s.as_str()).unwrap_or("c.c");

    let mut input = File::open(input_file_path)
        .expect("Unable to open input file");
    
    let mut contents = String::new();
    input.read_to_string(&mut contents)
        .expect("Failed to read input file");

    let result = bf2cify(contents)
        .expect("failed to bf2cify");

    let mut output = File::create(output_file_path)
        .expect("could not create output file");

    output.write_all(result.as_bytes()).unwrap();
}
