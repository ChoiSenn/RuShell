#[allow(unused_imports)]
use std::io::{self, Write};
use std::env;
use std::path::{Path, PathBuf};
use std::process::Command as ProcessCommand;
use std::fs::File;
use std::io::{stdout, stderr};
use std::process::Stdio;
use std::fs::OpenOptions;
use rustyline::completion::{Completer, Pair};
use rustyline::Context;
use rustyline::Result as RLResult;
use rustyline::history::DefaultHistory;
use rustyline::Editor;
use rustyline::error::ReadlineError;
use rustyline::Helper;
use rustyline::hint::Hinter;
use rustyline::highlight::Highlighter;
use rustyline::validate::Validator;

#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;
#[cfg(unix)]
use std::os::unix::process::CommandExt;

struct ShellHelper;

impl Helper for ShellHelper {}

impl Hinter for ShellHelper {
    type Hint = String;
}

impl Highlighter for ShellHelper {}

impl Validator for ShellHelper {}

impl Completer for ShellHelper {
    type Candidate = Pair;

    fn complete(
        &self,
        line: &str,
        pos: usize,
        _ctx: &Context<'_>,
    ) -> RLResult<(usize, Vec<Pair>)> {
        let input = &line[..pos];
        let tokens: Vec<&str> = input.split_whitespace().collect();

        let mut candidates = Vec::new();

        if tokens.len() <= 1 {
            let prefix = tokens.get(0).unwrap_or(&"");

            let builtins = ["cd", "pwd", "echo", "exit", "type"];

            for cmd in builtins {
                if cmd.starts_with(prefix) {
                    candidates.push(Pair {
                        display: cmd.to_string(),
                        replacement: format!("{} ", cmd),
                    });
                }
            }

            if let Some(path_var) = std::env::var_os("PATH") {
                for dir in std::env::split_paths(&path_var) {
                    if let Ok(entries) = std::fs::read_dir(dir) {
                        for entry in entries.flatten() {
                            let file_name = entry.file_name();
                            let file_str = file_name.to_string_lossy();

                            if file_str.starts_with(prefix) {
                                candidates.push(Pair {
                                    display: file_str.to_string(),
                                    replacement: format!("{} ", file_str),
                                });
                            }
                        }
                    }
                }
            }

            return Ok((0, candidates));
        }

        let last = tokens.last().unwrap_or(&"");

        if let Ok(entries) = std::fs::read_dir(".") {
            for entry in entries.flatten() {
                let file_name = entry.file_name();
                let file_str = file_name.to_string_lossy();

                if file_str.starts_with(last) {
                    candidates.push(Pair {
                        display: file_str.to_string(),
                        replacement: format!("{} ", candidates),
                    });
                }
            }
        }

        let start = input.rfind(' ').map(|i| i + 1).unwrap_or(0);
        Ok((start, candidates))
    }
}

// 명령 실행 환경 ()
struct ExecContext {
    stdout: Box<dyn Write>,
    stderr: Box<dyn Write>,
}

// 리다이렉트 대상 파일 경로 및 append 여부 (false: 덮어쓰기 / tru: 이어쓰기)
#[derive(Clone)]
struct Redirect {
    stdout: Option<(String, bool)>,
    stderr: Option<(String, bool)>,
}

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

// 인자 및 리다이렉션 분리
fn extract_redirect(tokens: Vec<String>) -> (Vec<String>, Option<Redirect>) {
    let mut args = Vec::new();
    let mut redirect = Redirect {
        stdout: None,
        stderr: None,
    };

    let mut has_redirect = false;

    let mut i = 0;
    while i < tokens.len() {
        // 토큰을 순회하며 리다이렉트 문구가 있는지 확인
        match tokens[i].as_str() {
            // stdout 리다이렉트 처리 
            ">" | "1>" => {  
                if i + 1 < tokens.len() {  // 다음 토큰에서 파일명 확인
                    redirect.stdout = Some((tokens[i + 1].clone(), false));
                    has_redirect = true;
                    i += 2;
                } else {
                    eprintln!("syntax error: no file after >");
                    break;
                }
            }
            // 표준 출력 처리 
            ">>" | "1>>" => {  
                if i + 1 < tokens.len() {
                    redirect.stdout = Some((tokens[i + 1].clone(), true));
                    has_redirect = true;
                    i += 2;
                } else {
                    eprintln!("syntax error: no file after >");
                    break;
                }
            }
            // 표준 오류 처리
            "2>" => {
                if i + 1 < tokens.len() {
                    redirect.stderr = Some((tokens[i + 1].clone(), false));
                    has_redirect = true;
                    i += 2;
                } else {
                        eprintln!("syntax error: no file after 2>");
                        break;
                }
            }
            // 표준 오류 추가
            "2>>" => {
                if i + 1 < tokens.len() {
                    redirect.stderr = Some((tokens[i + 1].clone(), true)); // append
                    has_redirect = true;
                    i += 2;
                } else {
                    eprintln!("syntax error: no file after 2>>");
                    break;
                }
            }
            // 일반 토큰일 시, args에 추가
            _ => {
                args.push(tokens[i].clone());
                i += 1;
            }
        }
    }

    // [인수 토큰들], Some(Redirect)
    if has_redirect {
        (args, Some(redirect))
    } else {
        (args, None)
    }
}

// 실행 환경 build
fn build_context(redirect: Option<Redirect>) -> ExecContext {
    let mut stdout: Box<dyn Write> = Box::new(stdout());
    let mut stderr: Box<dyn Write> = Box::new(stderr());

    if let Some(r) = redirect {
        if let Some((file, append)) = r.stdout {
            let f = if append {
                OpenOptions::new().create(true).append(true).open(file).unwrap()
            } else {
                File::create(file).unwrap()
            };
            stdout = Box::new(f);
        }
        if let Some((file, append)) = r.stderr {
            let f = if append {
                OpenOptions::new().create(true).append(true).open(file).unwrap()
            } else {
                File::create(file).unwrap()
            };
            stderr = Box::new(f);
        }
    }

    ExecContext { stdout, stderr }
}

fn parse_args(input: &str) -> Vec<String> {
    // 상태 기반으로 명령어/인수 parser 처리
    #[derive(PartialEq)]
    enum State {
        Normal,
        InSingleQuote,
        InDoubleQuote,
    }

    let mut args = Vec::new();
    let mut current = String::new();
    let mut state = State::Normal;
    let mut escaped = false;

    for c in input.chars() {  // 문자 단위로 순회
        // escape 처리
        if escaped {
            match state {
                // 기본 상태일때는 모든 문자 escape
                State::Normal => current.push(c),
                // " 안에서는 특정 문자만 escape
                State::InDoubleQuote => {
                    match c {
                        '"' | '\\' => current.push(c),
                        _ => {
                            current.push('\\');
                            current.push(c);
                        }
                    }
                }
                State::InSingleQuote => current.push(c),
            }
            escaped = false;
            continue;
        }

        match state {  // 현 상태에 따라 다르게 처리
            // 기본 상태일때는 (', " 등에 둘러싸여있지 않을 때)
            State::Normal => match c {
                '\'' => state = State::InSingleQuote,  // '가 입력되면 InSingleQuote 상태로
                '"' => state = State::InDoubleQuote,  // "가 입력되면 InDoubleQuote 상태로
                '\\' => escaped = true,
                ' ' => {  // 공백 입력 시, 토큰 종료. current를 args에 push
                    if !current.is_empty() {
                        args.push(std::mem::take(&mut current));
                    }
                }
                _ => current.push(c),
            },

            // '에 둘러싸여있는 상태일때는
            State::InSingleQuote => {
                if c == '\'' {  // '가 입력되면 닫혀지므로 Normal 상태로
                    state = State::Normal
                } else {  // 이외에는 current에 문자를 모음
                    current.push(c);
                }
            },

            // "에 둘러싸여있는 상태 동일
            State::InDoubleQuote => match c {
                '"' => state = State::Normal,
                '\\' => escaped = true,
                _ => current.push(c),
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
    let mut rl = Editor::<ShellHelper, DefaultHistory>::new().unwrap();
    rl.set_helper(Some(ShellHelper));

    // shell은 계속 반복되어야 하니까...
    // while true 하지 말고 loop를 쓰렴 더 짧으니까
    loop {
        // 프롬프트 시작
        let line = rl.readline("$ ");

        let input = match line {
            Ok(line) => {
                rl.add_history_entry(line.as_str()).unwrap();
                line
            }
            Err(ReadlineError::Interrupted) => continue, // Ctrl-C
            Err(ReadlineError::Eof) => break,            // Ctrl-D
            Err(err) => {
                eprintln!("Error: {:?}", err);
                break;
            }
        };

        let input = input.trim();
        if input.is_empty() {
            continue;
        }

        // 입력 값을 공백 기준으로 분해하여 &str 슬라이스 벡터 생성
        let tokens = parse_args(input);

        if tokens.is_empty() {
            continue;
        }

        // 리다이렉트 여부 확인
        let (tokens, redirect) = extract_redirect(tokens);

        if tokens.is_empty() {
            continue;
        }

        let command = &tokens[0];
        let args: Vec<&str> = tokens[1..].iter().map(|s| s.as_str()).collect();

        // context 생성
        let mut ctx = build_context(redirect.clone());

        // 입력 명령어에 따른 동작
        match Command::from_str(command) {
            Some(Command::Exit) => break,  // shell 종료
            Some(Command::Type) => type_command(&args, &mut ctx),  // 내장 명령어/실행 파일/인식되지 않은 명령어인지 확인
            Some(Command::Echo) => echo_command(&args, &mut ctx),  // 인자 출력
            Some(Command::Pwd) => pwd_command(&mut ctx),  // 현재 디렉터리명 출력
            Some(Command::Cd) => cd_command(&args),  // 현재 디렉터리 이동
            None => external_command(command, &args, redirect)  // 내장 명령어가 아닌 경우, 외부 프로그램 실행 및 인수 전달
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

fn pwd_command(ctx: &mut ExecContext) {
    match env::current_dir() {
        Ok(path) => writeln!(ctx.stdout, "{}", path.display()).unwrap(),
        Err(_) => writeln!(ctx.stdout, "Can't found path").unwrap(),
    }
}

fn external_command(cmd: &str, args: &[&str], redirect: Option<Redirect>) {
    // 실행 가능하지 않다면 return
    if let Some(path) = find_in_path(cmd) {
        let mut command = ProcessCommand::new(&path);
        command.args(args);

        // stdout / stderr 연결
        if let Some(r) = redirect {
            if let Some((file, append)) = r.stdout {
                let f = if append {
                    OpenOptions::new().create(true).append(true).open(file).unwrap()
                } else {
                    File::create(file).unwrap()
                };
                command.stdout(Stdio::from(f));
            }
            if let Some((file, append)) = r.stderr {
                let f = if append {
                    OpenOptions::new().create(true).append(true).open(file).unwrap()
                } else {
                    File::create(file).unwrap()
                };
                command.stderr(Stdio::from(f));
            }
        }

        // 프로세스 생성. arg0은 명령어(프로그램명), 인수로 나머지 인수 그대로. spawn() 이용하여 프로세스 fork. 자식 프로세스에서 exec 수행.
        #[cfg(unix)]
        {
            command.arg0(cmd);
        }

        let mut child = match command.spawn() {
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

fn type_command(args: &[&str], ctx: &mut ExecContext) {
    if let Some(cmd) = args.first() {
        // 내장 명령일 경우, 보고하고 중지
        if Command::from_str(cmd).is_some() {
            writeln!(ctx.stdout, "{} is a shell builtin", cmd).unwrap();
        } else if let Some(path) = find_in_path(cmd) {  // 내장 명령어가 아닌 경우, PATH를 참조하여 파일 전체 경로 반환
            writeln!(ctx.stdout, "{} is {}", cmd, path.display()).unwrap();
        } else {
            writeln!(ctx.stdout, "{}: not found", cmd).unwrap();
        }
    }
}

fn echo_command(args: &[&str], ctx: &mut ExecContext) {
    writeln!(ctx.stdout, "{}", args.join(" ")).unwrap();
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