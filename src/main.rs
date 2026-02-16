#[allow(unused_imports)]
use std::io::{self, Write};

fn main() {
    print!("$ ");

    let command = "";
    io::stdin().read_line(&mut command).unwrap();
    
    //io::stdout().flush().unwrap();
    println!("{}: command not found", command.trim());
}
