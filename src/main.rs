#[allow(unused_imports)]
use std::io::{self, Write};

#[derive(Debug)]
enum Command {
    Exit,
    Type,
    Echo,
}

impl Command {
    fn from_str(s: &str) -> Option<Self> {
        match s {
            "exit" => Some(Command::Exit),
            "type" => Some(Command::Type),
            "echo" => Some(Command::Echo),
            _ => None,
        }
    }
}

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

        // 입력 명령어에 따른 동작
        match Command::from_str(command) {
            Some(Command::Exit) => break,  // shell 종료
            Some(Command::Type) => type_command(&args),  // 내장 명령어/실행 파일/인식되지 않은 명령어인지 확인
            Some(Command::Echo) => echo_command(&args),  // 인자 출력
            None => println!("{}: command not found", command.trim()),
        }
    }
}

fn type_command(args: &[&str]) {
    if let Some(cmd) = args.first() {
        if Command::from_str(cmd).is_some() {
            println!("{} is a shell builtin", cmd);
        } else {
            println!("{}: not found", cmd);
        }
    }
}

fn echo_command(args: &[&str]) {
    println!("{}", args.join(" "));
}