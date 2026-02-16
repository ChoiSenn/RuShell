#[allow(unused_imports)]
use std::io::{self, Write};

fn main() {
    // 프롬프트 시작
    print!("$ ");
    io::stdout().flush().unwrap();

    // 입력 값 읽어와 저장 (command)
    let mut command = String::new();
    io::stdin().read_line(&mut command).unwrap();

    // 처리 및 출력
    println!("{}: command not found", command.trim());
}
