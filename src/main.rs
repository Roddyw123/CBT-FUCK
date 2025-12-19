mod bf2c;
use bf2c::bf2c::{bf2cify, bf2cify_without_verification};
use std::fs::File;
use std::io::{Read, Write};
use std::env;
use std::process;

fn main() {
    let args: Vec<String> = env::args().skip(1).collect();

    let mut input_file = None;
    let mut output_file = None;
    let mut verify = true;

    for arg in &args {
        if arg.starts_with("--") {
            match arg.as_str() {
                "--no-verify" => verify = false,
                "--help"      => {
                    println!("Usage: bf2c [input.bf output.c] [options]");
                    println!();
                    println!("Options:");
                    println!("  --help       Show this help message");
                    println!("  --no-verify  Skip loop verification during parsing");
                    println!();
                    println!("Defaults:");
                    println!("  input:       src/bf.bf");
                    println!("  output:      c.c");
                    return;
                }
                _             => {
                    eprintln!("Unknown flag: {}", arg);
                    process::exit(1);
                }
            }
        } else {
            if input_file.is_none() {
                input_file = Some(arg.clone());
            } else if output_file.is_none() {
                output_file = Some(arg.clone());
            } else {
                eprintln!("Too many files (expected at most 2)");
                process::exit(-1);
            }
        }
    }

    let input_file = input_file.unwrap_or("src/bf.bf".to_string());
    let output_file = output_file.unwrap_or("c.c".to_string());

    let mut input = File::open(input_file)
        .expect("Unable to open input file");
    
    let mut contents = String::new();
    input.read_to_string(&mut contents)
        .expect("Failed to read input file");

    let result = if verify{
        bf2cify(contents)
    } else {
        bf2cify_without_verification(contents)
    }
        .expect("failed to bf2cify");

    let mut output = File::create(output_file)
        .expect("could not create output file");

    output.write_all(result.as_bytes()).unwrap();
}
