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
        let mut input = String::new();
        io::stdin().read_line(&mut input).unwrap();
        let input = input.trim();

        if input.is_empty() {
            continue;
        }

        // 입력 값을 공백 기준으로 분해하여 &str 슬라이스 벡터 생성
        let mut input_command = input.split_whitespace();
        let command = input_command.next().unwrap();
        let args: Vec<&str> = input_command.collect();

        // exit 입력 시, 종료
        match command {
            "exit" => break,
            "echo" => echo_command(&args),
            _ => println!("{}: command not found", command.trim()),
        }
    }
}

fn echo_command(args: &[&str]) {
    println!("{}", args.join(" "));
}