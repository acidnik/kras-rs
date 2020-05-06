// use std::fs::File;
// use std::io::{self, Lines};
// use std::path::PathBuf;
// use std::error;
use std::io::BufReader;
use std::io::BufRead;
// use std::fmt;
use std::str::{self, FromStr};
// use std::collections::HashMap;


extern crate clap;
use clap::{Arg, App};

extern crate fileinput;
use fileinput::FileInput;

extern crate pom;
use pom::parser::*;
use pom::char_class::*;
// use pom::Parser;

#[derive(Debug)]
enum KrasValue {
    Str(String),
    PairDelim(String),
    ListDelim(String),
    Delim(Box<KrasValue>),
    ListItem(Box<(KrasValue, Option<KrasValue>)>),
    List(Vec<KrasValue>),
    Tuple(Vec<KrasValue>),
    Ident(String),
    Num(f64),
}

fn pad(i: usize, l: usize) -> String {
    std::iter::repeat(" ").take(i*l).collect()
}

impl KrasValue {
    fn pretty(&self, ident: usize, level: usize) -> String {
        match self {
            KrasValue::Str(ref s) => s.to_string(),
            KrasValue::List(ref v) => "[\n".to_string() 
                + &v.iter().map(|x| x.pretty(ident, level+1)).collect::<String>() 
                + "\n]",
            KrasValue::PairDelim(ref s) => s.to_string(),
            KrasValue::ListDelim(ref s) => s.to_string(),
            KrasValue::ListItem(ref kv) =>  kv.0.pretty(ident, level) 
                // + " "
                + &kv.1.as_ref().map_or("".to_string(), |v| v.pretty(ident, level)),
            KrasValue::Num(n) => n.to_string(),
            KrasValue::Tuple(ref v) => "(".to_string() + &v.iter().map(|x| x.pretty(ident, level+1)).collect::<String>() + ")",
            _ => format!("<<<< ??? {:?} ??? >>>>", self),
        }
    }
}

fn space<'a>() -> Parser<'a, u8, ()> {
    one_of(b" \t\r\n").repeat(0..).discard()
}

fn ident<'a>() -> Parser<'a, u8, String> {
    let first = is_a(alpha);
    let rest = is_a(alphanum);
    let ident = first + rest.repeat(0..);
    ident.collect().convert(str::from_utf8).map(|s| s.to_string())
}

fn number<'a>() -> Parser<'a, u8, f64> {
    let integer = one_of(b"123456789") - one_of(b"0123456789").repeat(0..) | sym(b'0');
    let frac = sym(b'.') + one_of(b"0123456789").repeat(1..);
    let exp = one_of(b"eE") + one_of(b"+-").opt() + one_of(b"0123456789").repeat(1..);
    let number = sym(b'-').opt() + integer + frac.opt() + exp.opt();
    number.collect().convert(str::from_utf8).convert(|s|f64::from_str(&s))
}

fn string<'a>() -> Parser<'a, u8, String> {
    let special_char = sym(b'\\') | sym(b'/') | sym(b'"')
        | sym(b'b').map(|_|b'\x08') | sym(b'f').map(|_|b'\x0C')
        | sym(b'n').map(|_|b'\n') | sym(b'r').map(|_|b'\r') | sym(b't').map(|_|b'\t');
    let escape_sequence = sym(b'\\') * special_char;
    let string = sym(b'"') * (none_of(b"\\\"") | escape_sequence).repeat(0..) - sym(b'"');
    string.convert(String::from_utf8)
}

fn pair_delim<'a>() -> Parser<'a, u8, KrasValue> {
    let delim = space() * (seq(b":") | seq(b"=>")) - space();
    delim.collect().convert(std::str::from_utf8).map(|s| KrasValue::PairDelim(s.to_string())) 
}

fn array_delim<'a>() -> Parser<'a, u8, KrasValue> {
    let delim = space() * seq(b",") - space();
    delim.collect().convert(std::str::from_utf8).map(|s| KrasValue::ListDelim(s.to_string()))
}

fn list_item<'a>() -> Parser<'a, u8, KrasValue> {
    let delim = call(value) + (array_delim() | pair_delim()).opt();
    delim.map(|(a, b)| KrasValue::ListItem(Box::new((a,b))))
}

fn array<'a>() -> Parser<'a, u8, Vec<KrasValue>> {
    // let elems = list(call(list_item), delim);
    let elems = list_item().repeat(0..);
    sym(b'[') * space() * elems - sym(b']')
}

fn value<'a>() -> Parser<'a, u8, KrasValue> {
    (
        string().map(|s| KrasValue::Str(s))
        | number().map(|n| KrasValue::Num(n))
        | array().map(|arr| KrasValue::List(arr))
    ) - space()
}

fn kras<'a>() -> Parser<'a, u8, KrasValue> {
    space() * value() - end()
}

fn main() {
    let matches = App::new("Kras")
        .version("0.0.1")
        .author("Nikita Bilous <nikita@bilous.me>")
        .about("Detect, highlight and pretty print structured data")
        .arg(Arg::with_name("indent")
             .short("i")
             .long("indent")
             .help("indentation. 0 to disable (but still color output)")
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
        .arg(Arg::with_name("min_len")
            .short("m")
            .long("min_len")
            .help("minimal length of data to be formatted")
            .default_value("20")
        )
        .arg(Arg::with_name("input")
             .index(1)
             .multiple(true)
             .help("Input files or stdin")
        )
        .get_matches();
    let files = matches.values_of("input").map(|fs| fs.collect::<Vec<_>>()).unwrap_or(Vec::new());
    let input = FileInput::new(&files);
    let reader = BufReader::new(input);
    // let mut buf = Vec::new();
    for line in reader.lines() {
        match line {
            Ok(s) => {
                let buf = s.as_bytes().iter().map(|x| *x).collect::<Vec<u8>>();
                // let buf = r#"["a": "c", ["b"]]"#.as_bytes().iter().map(|x| *x).collect::<Vec<u8>>();
                // let buf = b"{}";
                let r = kras().parse(&buf);
                println!("{} ===>>> {:?}", s, r);
                if let Ok(r) = r {
                    println!("{}", r.pretty(4, 0))
                }
            }
            Err(err) => println!("{:?}", err),
        }
    }
}
