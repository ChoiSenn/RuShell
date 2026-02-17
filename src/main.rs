#[allow(unused_imports)]
use std::io::{self, Write};

fn main() {
    // shell은 계속 반복되어야 하니까...
    // while true 하지 말고 loop를 쓰렴 더 짧으니까
    loop {
        // 프롬프트 시작
        print!("$ ");
        io::stdout().flush().unwrap();

        // 입력 값 읽어와 저장 (command)
        let mut command = String::new();
        io::stdin().read_line(&mut command).unwrap();

        let command = command.trim();

        // exit 입력 시, 종료
        match command {
            "exit" => break,
            _ => println!("{}: command not found", command.trim()),
        }
    }
}
