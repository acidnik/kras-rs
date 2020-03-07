use std::fs::File;
use std::io::{self, Lines};
use std::path::PathBuf;
use std::error;
use std::io::BufReader;
use std::io::BufRead;
use std::fmt;


extern crate clap;
use clap::{Arg, App};

extern crate fileinput;
use fileinput::FileInput;

#[derive(Debug)]
enum Token {
    SquareOpen,
    SquareClose,
    Str(String),
    Colon,
    FatComma, // =>
    Comma,
    Semicolon,
    Equal,
    BraceOpen,
    BraceClose,
    BracketOpen,
    BracketClose,
    // StringStart(String),
    // StringEnd(String),
    Unknown(String),
    Space(String),
}

impl Token {
    fn to_string(&self) -> String {
        match self {
            Token::SquareOpen => "[",
            Token::SquareClose => "]",
            Token::Str(ref s) => s,
            Token::Colon => ":",
            Token::FatComma => "=>",
            Token::Comma => ",",
            Token::Semicolon => ";",
            Token::Equal => "=",
            Token::BraceOpen => "{",
            Token::BraceClose => "}",
            Token::BracketOpen => "(",
            Token::BracketClose => ")",
            Token::Unknown(ref s) => s,
            Token::Space(ref s) => s,
        }.to_string()
    }
}

impl fmt::Display for Token {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::result::Result<(), std::fmt::Error> {
        write!(f, "{}", self.to_string())
    }
}

struct Scanner {
    input: Vec<char>,
    tokens: Vec<Token>,
    current: usize,
    start: usize,
}

// http://craftinginterpreters.com/scanning.html
impl Scanner {
    fn new(s: &str) -> Self {
        Scanner {
            input: s.chars().collect(),
            tokens: Vec::new(),
            current: 0,
            start: 0,
        }
    }

    fn scan(&mut self) {
        self.current = 0;
        while ! self.at_end() {
            self.start = self.current;
            self.scan_token();
        }
    }

    fn at_end(&self) -> bool {
        self.current >= self.input.len()
    }
    
    fn scan_token(&mut self) {
        if self.at_end() {
            return
        }
        let c = self.advance();
        let token = match c {
            '[' => Token::SquareOpen,
            ']' => Token::SquareClose,
            '{' => Token::BraceOpen,
            '}' => Token::BraceClose,
            '(' => Token::BracketOpen,
            ')' => Token::BracketClose,
            ':' => Token::Colon,
            ',' => Token::Comma,
            ';' => Token::Semicolon,
            '=' => {
                if self.check_match('>') {
                    Token::FatComma
                }
                else {
                    Token::Equal
                }
            },
            '"' => self.string(),
            _ if c.is_whitespace() => self.space(),
            _ => Token::Unknown(c.to_string()),
        };
        self.tokens.push(token);
    }

    fn space(&mut self) -> Token {
        while ! self.at_end() && self.peek().is_whitespace() {
            self.advance();
        }
        Token::Space(self.curr_string())
    }

    fn string(&mut self) -> Token {
        while ! self.at_end() {
            let c = self.advance();
            if c == '\\' && ! self.at_end() {
                self.advance();
                continue
            }
            if c == '"' {
                break
            }
        }
        if self.at_end() {
            return Token::Unknown(self.curr_string())
        }
        Token::Str(self.curr_string())
    }

    fn curr_string(&self) -> String {
        self.input[self.start .. self.current].iter().collect()
    }

    fn check_match(&mut self, c: char) -> bool {
        if self.at_end() {
            false
        }
        else {
            if self.input[self.current] == c {
                self.current += 1;
                true
            }
            else {
                false
            }
        }
    }

    fn advance(&mut self) -> char {
        // println!("{:?} [{}]", self.input, self.current);
        self.current += 1;
        self.input[self.current-1]
    }

    fn peek(&self) -> char {
        self.input[self.current]
    }
}

struct Parser {

}

impl Parser {
    fn new() -> Self {
        Parser {}
    }
    fn parse(&self, tokens: &Vec<Token>) {
        for token in tokens {

        }
    }
}


fn main() {
    let matches = App::new("Kras")
        .version("0.0.1")
        .author("Nikita Bilous <nikita@bilous.me>")
        .about("Detect, highlight and pretty print structured data")
        .arg(Arg::with_name("indent")
             .short("i")
             .long("indent")
             .help("indentation. 0 to disable")
             .default_value("4")
        )
        .arg(Arg::with_name("color")
            .short("c")
            .long("color")
            .help("colorize. On by default if output is tty")
        )
        .arg(Arg::with_name("sort")
            .short("s")
            .long("sort")
            .help("sort keys")
        )
        .arg(Arg::with_name("input")
             .index(1)
             .multiple(true)
        )
        .get_matches();
    let files = matches.values_of("input").map(|fs| fs.collect::<Vec<_>>()).unwrap_or(Vec::new());
    // let files = if matches.is_present("input"){
    //     matches.values_of("input").unwrap().collect::<Vec<_>>()
    // }
    // else {
    //     vec![]
    // };
    let input = FileInput::new(&files);
    let mut reader = BufReader::new(input);
    for line in reader.lines() {
        match line {
            Ok(s) => {
                let mut scanner = Scanner::new(&s);
                println!("{}", s);
                scanner.scan();
                for t in scanner.tokens {
                    print!("{}", t);
                }
                println!("");
                // for data in DetectData::new(&s) {
                //     let s = Scanner::new(data);
                // }
            }
            Err(err) => println!("{:?}", err),
        }
    }
}
