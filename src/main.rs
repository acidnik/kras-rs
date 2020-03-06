use std::fs::File;
use std::io::{self, Lines};
use std::path::PathBuf;
use std::error;
use std::io::BufReader;
use std::io::BufRead;


extern crate clap;
use clap::{Arg, App};

extern crate fileinput;
use fileinput::FileInput;

enum Token {
    SquareOpen,
    SquareClose,
    Str(String),
    // StringStart(String),
    // StringEnd(String),
    Unknown(String),
    Space(String),
}

struct Scanner {
    input: Vec<char>,
    tokens: Vec<String>,
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
        self.current < self.input.len()
    }
    
    fn scan_token(&mut self) {
        let c = self.advance();
        let token = match c {
            '[' => Token::SquareOpen,
            ']' => Token::SquareClose,
            '"' => self.string(),
            _ => Token::Unknown(c.to_string()),
        };
    }

    fn string(&mut self) -> Token {
        while self.peek() != '"' && ! self.at_end() {
            self.advance();
        }
        if self.at_end() {
            return Token::Unknown(self.curr_string())
        }
        Token::Str(self.curr_string())
    }

    fn curr_string(&self) -> String {
        self.input[self.start + 1 .. self.current - 1].collect()
    }

    fn advance(&mut self) -> char {
        self.current += 1;
        self.input[self.current-1]
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
    let files = matches.values_of("input").unwrap().collect::<Vec<_>>();
    let input = FileInput::new(&files);
    let mut reader = BufReader::new(input);
    for line in reader.lines() {
        match line {
            Ok(s) => {
                let mut s = Scanner::new(&s);
                s.scan();
                // for data in DetectData::new(&s) {
                //     let s = Scanner::new(data);
                // }
            }
            Err(err) => println!("{:?}", err),
        }
    }
}
