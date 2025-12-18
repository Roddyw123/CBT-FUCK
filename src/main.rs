mod bf2c;
use bf2c::bf2c::bf2cify;
use std::fs;
use std::fs::File;
use std::io::Write;

fn main() {
    println!("Hello, world!");
    let contents = fs::read_to_string("src/bf.bf").expect("Unable to read file");
    let result = bf2cify(contents).expect("failed to bf2cify");
    let mut file = File::create("c.c").unwrap();
    file.write_all(result.as_ref()).unwrap();
}
