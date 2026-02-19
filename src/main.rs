#[allow(unused_imports)]
use std::io::{self, Write};
use std::env;
use std::path::{Path, PathBuf};
use std::process::Command as ProcessCommand;

#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;

// 사용 가능한 문자열 목록
#[derive(Debug)]
enum Command {
    Exit,
    Type,
    Echo,
}

// Command 메서드 정의
impl Command {
    // 문자열을 Command 타입으로 변환
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
            None => external_command(command, &args)  // 내장 명령어가 아닌 경우, 외부 프로그램 실행 및 인수 전달
        }
    }
}

fn external_command(cmd: &str, args: &[&str]) {
    // 실행 가능하지 않다면 return
    if let Some(path) = find_in_path(cmd) {
        let mut child = match ProcessCommand::new(path).args(args).spawn() {
            Ok(child) => child,
            Err(_) => {
                println!("{}: command not found", cmd);
                return;
            }
        };
        let _ = child.wait();
    } else {
        println!("{}: command not found", cmd);
    }
}

fn type_command(args: &[&str]) {
    if let Some(cmd) = args.first() {
        // 내장 명령일 경우, 보고하고 중지
        if Command::from_str(cmd).is_some() {
            println!("{} is a shell builtin", cmd);
        } else if let Some(path) = find_in_path(cmd) {  // 내장 명령어가 아닌 경우, PATH를 참조하여 파일 전체 경로 반환
            println!("{} is {}", cmd, path.display());
        } else {
            println!("{}: not found", cmd);
        }
    }
}

fn echo_command(args: &[&str]) {
    println!("{}", args.join(" "));
}

fn find_in_path(cmd:&str) -> Option<PathBuf> {
    let path = Path::new(cmd);
    // 경로 직접 호출 (/ 포함 시, PATH를 보지 않음)
    if path.is_absolute() || cmd.contains('/') {
        if is_executable(path) {
            return Some(path.to_path_buf());
        }
        return None;
    }
    // PATH 환경변수 검색
    if let Some(path_var) = env::var_os("PATH") {
        for dir in env::split_paths(&path_var) {
            let full_path = dir.join(cmd);
            if is_executable(&full_path) {
                return Some(full_path);
            }
        }
    }
    None
}

fn is_executable(path:&Path) -> bool {
    if !path.is_file() {
        return false;
    }

    #[cfg(unix)]
    {
        if let Ok(metadata) = path.metadata() {
        let mode = metadata.permissions().mode();
        return mode & 0o111 != 0;
        }
        false
    }

    #[cfg(windows)]
    {
        // Windows는 실행 비트 개념 X
        return true;
    }
}