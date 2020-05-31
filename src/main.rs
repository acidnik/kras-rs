// disable some warnings for debug build
#![cfg_attr(debug_assertions, allow(dead_code, unused_imports, unused_mut, unused_variables))]

use std::io::BufReader;
use std::io::BufRead;
use std::str::{self, FromStr};
use std::cmp::Ordering;
use std::iter::FromIterator;

extern crate clap;
use clap::{Arg, App};

extern crate fileinput;
use fileinput::FileInput;

extern crate pom;
use pom::parser::*;
use pom::char_class::*;

extern crate pretty;
use pretty::*;

extern crate termcolor;
use termcolor::{Color, ColorSpec};


#[derive(Debug, Clone)]
struct OrdF64(f64);

impl PartialEq for OrdF64 {
    fn eq(&self, other: &Self) -> bool {
        self.0 == other.0
    }
}

impl Eq for OrdF64 { }

impl Ord for OrdF64 {
    fn cmp(&self, other: &Self) -> Ordering {
        self.0.partial_cmp(&other.0).unwrap_or(Ordering::Equal)
    }
}

impl PartialOrd for OrdF64 {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        self.0.partial_cmp(&other.0)
    }
}

#[derive(Debug, Clone, Ord, Eq, PartialEq, PartialOrd)]
enum KrasValue {
    // quoted string
    Str((char, String)),

    // value, delim?
    ListItem((Box<KrasValue>, Option<String>)),

    // value, delim, value, delim2?
    Pair((Box<KrasValue>, String, Box<KrasValue>, Option<String>)),

    // open brace, ListItem | Pair, close
    List((String, Vec<KrasValue>, String)),

    // a literal identificator, including null, true, false and any var name
    Ident(String),

    // ident, list
    Constructor((Box<KrasValue>, Box<KrasValue>)),

    // a number
    Num(OrdF64),
}

impl KrasValue {
    fn postprocess(mut self, sort: bool) -> Self {
        // convert lists to dicts, sort
        match self {
            KrasValue::List((ref o, ref items, ref c)) => {
                // if list looks like python dict / js object / perl hash / etc: convert ListItems
                // to Pairs
                // {key1: val1, key2: val2} => [ (key1, :), (val1, ,), (key2, :), (val2, ())] => [ (key1, :, val1, ,), (key2, :, val2, ()) ]
                if items.len() % 2 == 0 {
                    let mut is_dict = true;
                    for (i, item) in items.iter().enumerate() {
                        if i % 2 == 0 {
                            // each even list item delimeter must be a dict separator
                            if let KrasValue::ListItem((_, Some(d))) = item {
                                if ! (d == "=>" || d == ":" || d == "=") {
                                    is_dict = false;
                                    break
                                }
                            }
                        }
                    }
                    if is_dict {
                        // TODO can it be done without clone?
                        let mut res = Vec::new();
                        for (i, kv) in items.chunks(2).enumerate() {
                            if let [k, v] = kv {
                                if let KrasValue::ListItem(k) = k {
                                    if let KrasValue::ListItem(v) = v {
                                        res.push(KrasValue::Pair(( 
                                            Box::new(k.0.clone().postprocess(sort)),
                                            k.1.clone().unwrap(),
                                            Box::new(v.0.clone().postprocess(sort)),
                                            v.1.clone()
                                        )))
                                    }
                                }
                            }
                        }
                        if sort {
                            res.sort();
                            self.fix_comma(&mut res);
                        }
                        self = KrasValue::List((o.to_string(), res, c.to_string()));
                    }
                }
            },
            KrasValue::Constructor(kv) => {
                let (ident, mut args) = kv;
                args = Box::new(args.postprocess(sort));
                self = KrasValue::Constructor((ident, args))
            }
            _ => {},
        }
        self
    }
    fn fix_comma(&self, list: &mut Vec<KrasValue>) {
        // {"2": 2, "1": 1} => sort => {"1": 1<no comma> "2": 2,<extra comma>} 
        // => fix => {"1": 1,<add comma> "2": 2<remove comma> } => {"1": 1, "2": 2}
        let len = list.len();
        for (i, mut item) in list.iter_mut().enumerate() {
            if let KrasValue::Pair((k, d, v, d2)) = item {
                match (d2.is_some(), i == len-1) {
                    (true, true) => {
                        let d2 = None;
                        *item = KrasValue::Pair((k.clone(), d.to_string(), v.clone(), d2))
                    },
                    (false, false) => {
                        let d2 = Some(",".to_string());
                        *item = KrasValue::Pair((k.clone(), d.to_string(), v.clone(), d2))
                    }
                    _ => {},
                }
            }
        }
    }
}

impl KrasValue {
    fn kv_spaces(&self, d: String) -> RcDoc<()> {
        // '=>' - spaces around
        // ':' - spaces to the right ': '
        // '=' - no spaces
        // ',' - ', '
        let ds: &str = &d;
        match ds {
            "=>" => RcDoc::space().append(RcDoc::text(d)).append(RcDoc::space()),
            ":"  => RcDoc::text(d).append(RcDoc::space()),
            "="  => RcDoc::text(d),
            ","  => RcDoc::text(d).append(RcDoc::space()),
            _ => panic!(format!("unexpected kv delim ['{}']", d)),
        }
    }

    fn to_doc(&self, indent: usize) -> RcDoc<()> {
        let nest = indent as isize; // why tf _i_size?
        match self {
            // TODO quotes
            KrasValue::Str((q, s)) => RcDoc::as_string(q.to_string() + s + &q.to_string()),
            KrasValue::Ident(s) => RcDoc::as_string(s),
            KrasValue::List((op, it, cl)) => {
                RcDoc::text(op)
                    .append(RcDoc::nil()
                        .append(RcDoc::line_())
                        .nest(nest)
                        .append(RcDoc::intersperse(it.iter().map(|x| x.to_doc(indent)), RcDoc::softline_())
                        .nest(nest)
                        .append(Doc::line_()))
                        .group()
                    )
                    .append(cl)
            },
            KrasValue::Pair((k, d, v, d2)) => {
                RcDoc::nil()
                    .append(
                        RcDoc::nil()
                        // key
                        .append(k.to_doc(indent))//.append(Doc::line_())
                        // kv delim
                        .append(self.kv_spaces(d.to_string()))
                        .group()
                    )
                    .append(
                        // value
                        RcDoc::nil()
                        .append(v.to_doc(indent))
                        // list delim
                        .append(d2.clone().map_or(RcDoc::nil(), |d| self.kv_spaces(d.to_string())))
                        .group()
                        .append(Doc::line_())
                    )
            },
            KrasValue::ListItem((v, d)) => {
                RcDoc::nil()
                    .append(v.to_doc(indent))
                    .append(d.clone().map_or(RcDoc::nil(), |d| self.kv_spaces(d.to_string())))
            }
            KrasValue::Num(OrdF64(n)) => RcDoc::as_string(n),
            KrasValue::Constructor((id, args)) => {
                RcDoc::nil()
                    .append(id.to_doc(indent))
                    .append(args.to_doc(indent))
                    .group()
            }
        }
    }
}


fn space<'a>() -> Parser<'a, char, ()> {
    one_of(" \t\r\n").repeat(0..).discard()
}

fn ident<'a>() -> Parser<'a, char, String> {
    let first = is_a(|c: char| c.is_alphabetic());
    let dot = sym('.');
    let rest = is_a(|c: char| c.is_alphanumeric()) | dot;
    let ident = first + rest.repeat(0..);
    ident.collect().map(String::from_iter)
}

fn number<'a>() -> Parser<'a, char, f64> {
    let integer = one_of("123456789") - one_of("0123456789").repeat(0..) | sym('0');
    let frac = sym('.') + one_of("0123456789").repeat(1..);
    let exp = one_of("eE") + one_of("+-").opt() + one_of("0123456789").repeat(1..);
    let number = sym('-').opt() + integer + frac.opt() + exp.opt();
    number.collect().map(String::from_iter).convert(|s| f64::from_str(&s))
}

fn qqstring<'a>() -> Parser<'a, char, (char, String)> {
    let special_char = sym('\\') | sym('/') | sym('"')
        | sym('b').map(|_|'\x08') | sym('f').map(|_|'\x0C')
        | sym('n').map(|_|'\n') | sym('r').map(|_|'\r') | sym('t').map(|_|'\t');
    let escape_sequence = sym('\\') * special_char;
    let string = sym('"') + (none_of("\\\"") | escape_sequence).repeat(0..) - sym('"');
    string.map(|(a, b)| (a, b.iter().collect()))
}

fn qstring<'a>() -> Parser<'a, char, (char, String)> {
    let special_char = sym('\\') | sym('/') | sym('"')
        | sym('b').map(|_|'\x08') | sym('f').map(|_|'\x0C')
        | sym('n').map(|_|'\n') | sym('r').map(|_|'\r') | sym('t').map(|_|'\t');
    let escape_sequence = sym('\\') * special_char;
    let string = sym('\'') + (none_of("\\\'") | escape_sequence).repeat(0..) - sym('\'');
    string.map(|(a, b)| (a, b.iter().collect()))
}

fn string<'a>() -> Parser<'a, char, KrasValue> {
    let string = qqstring() | qstring();

    string.map(|(a, b)| KrasValue::Str((a, b)))
}

fn pair_delim<'a>() -> Parser<'a, char, String> {
    let delim = space() * (seq(&[':']) | seq(&['=','>']) | seq(&['='])) - space();
    // delim.collect().map(String::from_iter)
    delim.map(String::from_iter)
}

fn array_delim<'a>() -> Parser<'a, char, String> {
    let delim = space() * sym(',') - space();
    // delim.map(String::from_iter)
    delim.map(|c| c.to_string())
}

fn list_item<'a>() -> Parser<'a, char, KrasValue> {
    let delim = call(inner_value) + (array_delim() | pair_delim()).opt();
    delim.map(|(a, b)| KrasValue::ListItem((Box::new(a), b)))
}

fn array<'a>() -> Parser<'a, char, (String, Vec<KrasValue>, String)> {
    // parse array | set | dict | tuple | etc
    // (a=>b, c => d) => [ (a=>) (b,) (c=>) (d) ]
    let arr = sym('[') + space() * list_item().repeat(0..) + sym(']');
    let set = sym('{') + space() * list_item().repeat(0..) + sym('}');
    let tup = sym('(') + space() * list_item().repeat(0..) + sym(')');
    (arr | set | tup).map(|((a, b), c)| (a.to_string(), b, c.to_string() ))
}

fn constructor<'a>() -> Parser<'a, char, KrasValue> {
    let res = ident() - space() + array();
    res.map(|(a, b)| KrasValue::Constructor((Box::new(KrasValue::Ident(a)), Box::new(KrasValue::List(b)))))
}

fn inner_value<'a>() -> Parser<'a, char, KrasValue> {
    value() | ident().map(|s| KrasValue::Ident(s))
}

fn value<'a>() -> Parser<'a, char, KrasValue> {
    (
        string()
        | number().map(|n| KrasValue::Num(OrdF64(n)))
        | constructor()
        | array().map(|(s, arr, c)| KrasValue::List((s, arr, c)))
    ) - space()
}

fn kras<'a>() -> Parser<'a, char, KrasValue> {
    space() * value() - end()
}

fn main() {
    // TODO control trailing comma: add | remove | keep (!sort)
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
    // let mut buf = Vec::new();
    for line in reader.lines() {
        match line {
            Ok(s) => {
                // FIXME
                // TODO add flag to skip comments?
                if s.starts_with("//") {
                    continue
                }
                let buf = s.chars().collect::<Vec<_>>();
                // let buf = s.as_bytes().iter().map(|x| *x).collect::<Vec<u8>>();
                // let buf = r#"["a": "c", ["b"]]"#.as_bytes().iter().map(|x| *x).collect::<Vec<u8>>();
                // let buf = b"{}";
                let mut r = kras().parse(&buf);
                println!("{} ===>>> {:?}", s, r);
                if let Ok(mut r) = r {
                    r = r.postprocess(sort);
                    let mut res = Vec::new();
                    r.to_doc(indent).render(min_len, &mut res).unwrap();
                    let pretty = String::from_utf8(res).unwrap();
                    // println!("{} => {:?} => {}", s, r, pretty)
                    println!("{} =>\n{}", s, pretty)
                }
                else {
                    println!("{} =>\n{:?}", s, r.err());
                }
            }
            Err(err) => println!("{:?}", err),
        }
    }
}
