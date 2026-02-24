#[allow(unused_imports)]
use std::io::{self, Write};
use std::env;
use std::path::{Path, PathBuf};
use std::process::Command as ProcessCommand;

#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;
#[cfg(unix)]
use std::os::unix::process::CommandExt;

// 사용 가능한 문자열 목록
#[derive(Debug)]
enum Command {
    Exit,
    Type,
    Echo,
    Pwd,
    Cd,
}

// Command 메서드 정의
impl Command {
    // 문자열을 Command 타입으로 변환
    fn from_str(s: &str) -> Option<Self> {
        match s {
            "exit" => Some(Command::Exit),
            "type" => Some(Command::Type),
            "echo" => Some(Command::Echo),
            "pwd" => Some(Command::Pwd),
            "cd" => Some(Command::Cd),
            _ => None,
        }
    }
}

fn parse_args(input: &str) -> Vec<String> {
    // 상태 기반으로 명령어/인수 parser 처리
    #[derive(PartialEq)]
    enum State {
        Normal,
        InSingleQuote,
        InDoubleQuote,
        Backslash,
    }

    let mut args = Vec::new();
    let mut current = String::new();
    let mut state = State::Normal;

    for c in input.chars() {  // 문자 단위로 순회
        match state {  // 현 상태에 따라 다르게 처리
            // 기본 상태일때는 (', " 등에 둘러싸여있지 않을 때)
            State::Normal => match c {
                '\'' => state = State::InSingleQuote,  // '가 입력되면 InSingleQuote 상태로
                '"' => state = State::InDoubleQuote,  // "가 입력되면 InDoubleQuote 상태로
                ' ' => {  // 공백 입력 시, 토큰 종료. current를 args에 push
                    if !current.is_empty() {
                        args.push(std::mem::take(&mut current));
                    }
                }
                _ => current.push(c),  // 그 이외의 문자를 current에 모음
            },

            // '에 둘러싸여있는 상태일때는
            State::InSingleQuote => {
                if c == '\'' {  // '가 입력되면 닫혀지므로 Normal 상태로
                    state = State::Normal;
                } else {  // 이외에는 current에 문자를 모음
                    current.push(c);
                }
            },

            // "에 둘러싸여있는 상태 동일
            State::InDoubleQuote => {
                if c == '"' {
                    state = State::Normal;
                } else {
                    current.push(c);
                }
            },

            // 역슬래시가 존재하면 뒤의 문자에 대해 이스케이프 처리
            State::Backslash => {
                current.push(c);
                state = State::Normal;  // 뒤의 한 문자만 처리하고 상태 되돌림
            },
        }
    }
    // 모은 문자열 push
    if !current.is_empty() {
        args.push(current);
    }
    args
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
        let tokens = parse_args(input);

        if tokens.is_empty() {
            continue;
        }

        let command = &tokens[0];
        let args: Vec<&str> = tokens[1..].iter().map(|s| s.as_str()).collect();

        // 입력 명령어에 따른 동작
        match Command::from_str(command) {
            Some(Command::Exit) => break,  // shell 종료
            Some(Command::Type) => type_command(&args),  // 내장 명령어/실행 파일/인식되지 않은 명령어인지 확인
            Some(Command::Echo) => echo_command(&args),  // 인자 출력
            Some(Command::Pwd) => pwd_command(),  // 현재 디렉터리명 출력
            Some(Command::Cd) => cd_command(&args),  // 현재 디렉터리 이동
            None => external_command(command, &args)  // 내장 명령어가 아닌 경우, 외부 프로그램 실행 및 인수 전달
        }
    }
}

fn cd_command(args: &[&str]) {
    // 해당 디렉터리가 존재하면 그 디렉터리로 이동
    if let Some(dir) = args.first() {
        let path = Path::new(dir);

        // cd ~이면 홈 디렉터리로 이동
        if *dir == "~" {
            let home_path = if cfg!(unix) {
                env::var_os("HOME")
            } else {
                env::var_os("USERPROFILE")
            };

            if let Some(home_path) = home_path {
                if let Err(e) = env::set_current_dir(&home_path) {
                    println!("cd: {}", e);
                }
            }

            return;
        }

        if path.exists() && path.is_dir() {
            if let Err(e) = env::set_current_dir(path) {
                println!("Connot set path : {}", e);
            }
        } else {
            // 존재하지 않으면 오류 출력
            println!("cd: {}: No such file or directory", path.display());
        }
    }
}

fn pwd_command() {
    match env::current_dir() {
        Ok(path) => println!("{}", path.display()),
        Err(_e) => println!("Can't found path"),
    }
}

fn external_command(cmd: &str, args: &[&str]) {
    // 실행 가능하지 않다면 return
    if let Some(path) = find_in_path(cmd) {
        // 프로세스 생성. arg0은 명령어(프로그램명), 인수로 나머지 인수 그대로. spawn() 이용하여 프로세스 fork. 자식 프로세스에서 exec 수행.
        #[cfg(unix)]
        let mut child = match ProcessCommand::new(path).arg0(cmd).args(args).spawn() {
            Ok(child) => child,  // child 핸들 반환
            Err(_) => {
                println!("{}: command not found", cmd);
                return;
            }
        };
        #[cfg(windows)]
        let mut child = match ProcessCommand::new(path).arg(cmd).args(args).spawn() {
            Ok(child) => child,  // child 핸들 반환
            Err(_) => {
                println!("{}: command not found", cmd);
                return;
            }
        };
        let _ = child.wait();  // 자식 프로세스 종료 대기
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