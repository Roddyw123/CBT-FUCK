mod bf2c;
use bf2c::bf2c::bf2cify;
use std::fs::File;
use std::io::{Read, Write};
use std::env;
use std::process;

fn main() {
    let args: Vec<String> = env::args().collect();

    // handle --help
    if args.len() == 2 && args[1] == "--help" {
        println!("Usage: {} [input.bf output.c] [options]", args[0]);
        println!("Defaults (if not provided):");
        println!("  input.bf  -> input file");
        println!("  output.c  -> output file");
        process::exit(0);
    }

    // handle input and output files
    let (input_file_path, output_file_path) = match args.len() {
        1 => ("src/bf.bf", "c.c"),                  // default when no args
        3 => (args[1].as_str(), args[2].as_str()),  // normal when two args
        _ => {
            eprintln!("Invalid usage, try --help");
            process::exit(-1);
        }
    };

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
