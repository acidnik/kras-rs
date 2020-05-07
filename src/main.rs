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

#[derive(Debug)]
enum KrasValue {
    Str(String),
    PairDelim(String),
    ListDelim(String),

    // pair delim | list_delim
    Delim(Box<KrasValue>),

    // value, delim
    ListItem(Box<(KrasValue, Option<KrasValue>)>),

    // open brace, values
    List((String, Vec<KrasValue>)),
    Ident(String),

    // ident, list
    Constructor(Box<(KrasValue, KrasValue)>),
    Num(f64),
}

struct PrettyPrint {
    indent: usize,
    sort: bool,
    color: bool,
    min_len: usize,
}

fn pad(i: usize, l: usize) -> String {
    std::iter::repeat(" ").take(i*l).collect()
}

fn close(s: &str) -> String {
    match s {
        "[" => "]",
        "{" => "}",
        "(" => ")",
        _ => panic!("parsed bad array"),
    }.to_string()
}

enum PrettyData {
    Leaf(String),
    Nested(Vec<PrettyData>),
}

impl PrettyPrint {
    fn new(indent: usize, sort: bool, color: bool, min_len: usize) -> Self {
        PrettyPrint {indent, sort, color, min_len}
    }

    fn indent_data(&self, p: &PrettyData, level: usize) -> String {
        use PrettyData::*;
        match p {
            Leaf(s) => pad(self.indent, level) + s,
            Nested(s) => pad(self.indent, level) + &s.iter().map(|x| self.indent_data(x, level+1)).collect::<String>(),
        }
    }

    fn pretty_vec(&self, v: &KrasValue) -> PrettyData {
        use PrettyData::*;
        match v {
            KrasValue::Str(_) => Leaf(self.pretty(&v)),
            KrasValue::List((ref s, ref v)) => Nested(vec![
                Leaf(s.to_string()),
                Nested(v.iter().map(|x| self.pretty_vec(&x)).collect::<Vec<PrettyData>>()),
                Leaf(close(s))
            ]),
            KrasValue::PairDelim(_) => Leaf(self.pretty(&v)),
            KrasValue::Ident(_) => Leaf(self.pretty(&v)),
            KrasValue::ListDelim(_) => Leaf(self.pretty(&v)),
            KrasValue::ListItem(ref kv) =>  Nested(vec![
                self.pretty_vec(&kv.0), // the item
                Leaf(kv.1.as_ref().map_or("".to_string(), |v| self.pretty(&v))) // the separator
            ]),
            KrasValue::Num(_) => Leaf(self.pretty(&v)),
            KrasValue::Constructor(ref nv) => Nested(vec![
                Leaf(self.pretty(&nv.0)), // ident
                self.pretty_vec(&nv.1) // args
            ]),
            _ => Leaf(format!("<<<< ??? {:?} ??? >>>>", v)),
        }
    }

    fn pretty(&self, v: &KrasValue) -> String {
        match v {
            KrasValue::Str(ref s) => r#"""#.to_string() + &s.to_string() + r#"""#,
            // KrasValue::List((ref s, ref v)) => s.to_string() + "\n"
            //     + &v.iter().map(|x| self.pretty(&x)).collect::<String>() 
            //     + "\n" + &close(s),
            KrasValue::PairDelim(ref s) => s.to_string(),
            KrasValue::Ident(ref s) => s.to_string(),
            KrasValue::ListDelim(ref s) => s.to_string(),
            // KrasValue::ListItem(ref kv) =>  self.pretty(&kv.0) 
            //     + &kv.1.as_ref().map_or("".to_string(), |v| self.pretty(&v)),
            KrasValue::Num(n) => n.to_string(),
            // KrasValue::Constructor(ref nv) => self.pretty(&nv.0) + &self.pretty(&nv.1),
            _ => format!("<<<< ??? {:?} ??? >>>>", v),
        }
    }

    fn pretty_indent(&self, v: &KrasValue) -> String {
        let data = self.pretty_vec(&v);
        self.indent_data(&data, 0)
    }
}

fn space<'a>() -> Parser<'a, u8, ()> {
    one_of(b" \t\r\n").repeat(0..).discard()
}

fn ident<'a>() -> Parser<'a, u8, String> {
    let first = is_a(alpha);
    let dot = sym(b'.');
    let rest = is_a(alphanum) | dot;
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
    let delim = call(inner_value) + (array_delim() | pair_delim()).opt();
    delim.map(|(a, b)| KrasValue::ListItem(Box::new((a,b))))
}

fn array<'a>() -> Parser<'a, u8, (String, Vec<KrasValue>)> {
    let arr = sym(b'[') + space() * list_item().repeat(0..) - sym(b']');
    let set = sym(b'{') + space() * list_item().repeat(0..) - sym(b'}');
    let tup = sym(b'(') + space() * list_item().repeat(0..) - sym(b')');
    (arr | set | tup).map(|(a, b)| (std::str::from_utf8(&[a]).unwrap_or("").to_string(), b))
}

fn constructor<'a>() -> Parser<'a, u8, KrasValue> {
    let res = ident() - space() + array();
    res.map(|(a, b)| KrasValue::Constructor(Box::new((KrasValue::Ident(a), KrasValue::List(b)))))
}

fn inner_value<'a>() -> Parser<'a, u8, KrasValue> {
    value() | ident().map(|s| KrasValue::Ident(s))
}

fn value<'a>() -> Parser<'a, u8, KrasValue> {
    (
        string().map(|s| KrasValue::Str(s))
        | number().map(|n| KrasValue::Num(n))
        | constructor()
        | array().map(|(s, arr)| KrasValue::List((s, arr)))
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
             .default_value("2")
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
    let indent = usize::from_str(matches.value_of("indent").unwrap()).unwrap();
    let min_len = usize::from_str(matches.value_of("min_len").unwrap()).unwrap();
    let files = matches.values_of("input").map(|fs| fs.collect::<Vec<_>>()).unwrap_or(Vec::new());
    let color = matches.is_present("color");
    let sort = matches.is_present("sort");
    let input = FileInput::new(&files);
    let reader = BufReader::new(input);
    let printer = PrettyPrint::new(indent, sort, color, min_len);
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
                    println!("{}", printer.pretty_indent(&r));
                }
            }
            Err(err) => println!("{:?}", err),
        }
    }
}
