use std::{iter::FromIterator, str::FromStr};

use pom::parser::*;

use crate::{detect::DetectDataIter, detect2::DetectDataV2, pretty_value::*, stopwatch::Stopwatch};

fn space<'a>() -> Parser<'a, char, ()> {
    one_of(" \t\r\n").repeat(0..).discard()
}

fn ident<'a>() -> Parser<'a, char, String> {
    let first = is_a(|c: char| c.is_alphabetic()) | one_of("_%$@\\/");

    fn alnum<'a>() -> Parser<'a, char, String> {
        let alnum = is_a(|c: char| c.is_alphanumeric()) | one_of("_%$@\\/");
        alnum.collect().map(String::from_iter)
    }
    let dot = sym('.') | sym(':') | sym('-');

    // [a-z] [a-z0-9]* ([:.]+[a-z0-9]+)*
    let ident = first + alnum().repeat(0..) + (dot.repeat(1..) + alnum().repeat(1..)).repeat(0..);

    ident.collect().map(String::from_iter) - space()
}

fn plain_number<'a>() -> Parser<'a, char, (f64, String)> {
    let integer = (one_of("123456789") - one_of("0123456789").repeat(0..)) | sym('0');
    let frac = sym('.') + one_of("0123456789").repeat(1..);
    let exp = one_of("eE") + one_of("+-").opt() + one_of("0123456789").repeat(1..);
    let number = sym('-').opt() + integer + frac.opt() + exp.opt();
    let repr = number.collect().map(String::from_iter);
    repr.convert(|s| f64::from_str(&s).map(|n| (n, s)))
}

fn hex_number<'a>() -> Parser<'a, char, (f64, String)> {
    let prefix = sym('0') + one_of("xX");
    let integer = one_of("0123456789abcdefABCDEF");
    let hex = (prefix + integer.repeat(1..)).collect().map(String::from_iter);
    hex.convert(|s| u64::from_str_radix(&s[2..], 16).map(|n| (n as f64, s)))
}

fn number<'a>() -> Parser<'a, char, (f64, String)> {
    hex_number() | plain_number()
}

fn x_char<'a>() -> Parser<'a, char, char> {
    // parse '\xFF'
    let hex = one_of("0123456789abcdefABCDEF");
    let ch = one_of("xX") * hex.repeat(2).map(String::from_iter);
    ch.convert(|s| u32::from_str_radix(&s, 16)).convert(|n| {
        std::char::from_u32(n)
            // .and_then(|x| if x.is_ascii_graphic() { Some(x) } else { None })
            .ok_or("not a valid unicode")
    })
}

fn json_unicode<'a>() -> Parser<'a, char, char> {
    let hex = one_of("0123456789abcdefABCDEF");
    let ch = sym('u') * hex.repeat(4).map(String::from_iter);
    ch.convert(|s| u32::from_str_radix(&s, 16))
        .convert(|n| std::char::from_u32(n).ok_or("not a valid unicode"))
}

fn special_char<'a>() -> Parser<'a, char, char> {
    json_unicode()
        | x_char()
        | sym('\\')
        | sym('/')
        | sym('"')
        | sym('\'')
        | sym('b').map(|_| '\x08')
        | sym('f').map(|_| '\x0C')
        | sym('n').map(|_| '\n')
        | sym('r').map(|_| '\r')
        | sym('t').map(|_| '\t')
}

fn qqstring<'a>() -> Parser<'a, char, (char, String)> {
    let escape_sequence = sym('\\') * special_char();
    let string = sym('"') + (none_of("\\\"") | escape_sequence).repeat(0..) - sym('"');
    string.map(|(a, b)| (a, b.iter().collect()))
}

fn qstring<'a>() -> Parser<'a, char, (char, String)> {
    let escape_sequence = sym('\\') * special_char();
    let string = sym('\'') + (none_of("\\\'") | escape_sequence).repeat(0..) - sym('\'');
    string.map(|(a, b)| (a, b.iter().collect()))
}

fn string<'a>() -> Parser<'a, char, KrasValue> {
    // TODO if \x | \u char is not printable - keep it as is ('\x00')

    fn alpha<'a>() -> Parser<'a, char, String> {
        let is_alpha = is_a(|c: char| c.is_alphabetic());
        is_alpha.repeat(0..).collect().map(String::from_iter)
    }

    let string = alpha() + (qqstring() | qstring());

    string.map(|(p, (q, s))| KrasValue::Str((q, p, s)))
}

fn pair_delim<'a>() -> Parser<'a, char, String> {
    let delim = space() * (seq(&[':']) | seq(&['=', '>']) | seq(&['='])) - space();
    delim.map(String::from_iter)
}

fn array_delim<'a>() -> Parser<'a, char, String> {
    let delim = space() * one_of(",;") - space();
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
    let ang = sym('<') + space() * list_item().repeat(0..) + sym('>');
    (arr | set | tup | ang).map(|((a, b), c)| (a.to_string(), b, c.to_string()))
}

fn constructor<'a>() -> Parser<'a, char, KrasValue> {
    let res = ident() - space() + array();
    res.map(|(a, b)| KrasValue::Constructor((Box::new(KrasValue::Ident(a)), Box::new(KrasValue::List(b)))))
}

fn inner_value<'a>() -> Parser<'a, char, KrasValue> {
    value() | ident().map(KrasValue::Ident)
}

fn value<'a>() -> Parser<'a, char, KrasValue> {
    (string()
        | number().map(|(n, r)| KrasValue::Num(OrdF64(n, r)))
        | constructor()
        | array().map(|(s, arr, c)| KrasValue::List((s, arr, c))))
        - space()
}

pub fn kras<'a>() -> Parser<'a, char, KrasValue> {
    space() * value() - end()
}

// bool sort, bool robust
struct RecursiveStringParser(bool, bool);

impl KrasVisitor for RecursiveStringParser {
    fn visit_str(&self, val: &mut KrasValue) {
        if let KrasValue::Str((_, _, ref s)) = val {
            let mut inner = parse_str(s, self.0, true, self.1);
            debug!("rec parse: {:?}", inner);
            if let KrasValue::RawList(ref mut items) = inner {
                if items.is_empty() {
                    // parset Str("") as RawList([])
                    // do nothing and keep the val as is
                }
                else if items.len() == 1 {
                    let single_val = items.pop().unwrap();
                    if let KrasValue::RawStr(_) = single_val {
                        // parsed Str("a b c") as RawStr("a b c").
                        // Do nothing and let `val` be
                    }
                    else {
                        *val = single_val
                    }
                }
                else {
                    *val = inner
                }
            }
        }
    }
}

pub fn parse_str(s: &str, sort: bool, recursive: bool, robust: bool) -> KrasValue {
    let mut res = Vec::new();
    let buf = s.chars().collect::<Vec<_>>();
    let mut start = 0;
    let iter: Box<dyn Iterator<Item = (usize, &[char])>> = if robust {
        Box::new(DetectDataV2::new(&buf))
    }
    else {
        Box::new(DetectDataIter::new(&buf))
    };
    for (pos, data) in iter {
        debug!("DETECT: {}", String::from_iter(data));
        let mut stopwatch = Stopwatch::new("parse", 0);
        let r = kras().parse(data);
        stopwatch.stop();
        if let Ok(mut r) = r {
            debug!("PARSED: {:?}", r);
            if pos > start {
                res.push(KrasValue::RawStr(String::from_iter(buf[start..pos].iter())));
            }
            start = pos + data.len();
            let mut stopwatch = Stopwatch::new("postprocess", 0);
            r.postprocess(sort);
            if recursive {
                let rec_parser = RecursiveStringParser(sort, robust);
                r.visit(&rec_parser)
            }
            stopwatch.stop();
            debug!("POSTPROC: {:?}", r);
            res.push(r);
        }
        else {
            // TODO what will happend here?
            debug!("parse error {:?}", r);
        }
    }
    if start < buf.len() {
        res.push(KrasValue::RawStr(String::from_iter(buf[start..].iter())));
    }
    KrasValue::RawList(res)
}

#[cfg(test)]
mod test {
    use super::*;

    fn check_single_value(input: &str, expected: &KrasValue) {
        check_single_value_with(input, expected, |a, b| a == b)
    }

    // fn check_single_value_with(input: &str, expected: &KrasValue, cmp_with: Fn(&KrasValue, &KrasValue) -> bool) { // ? this looks like the same thing but does not compiles
    fn check_single_value_with<F>(input: &str, expected: &KrasValue, cmp_with: F)
    where
        F: Fn(&KrasValue, &KrasValue) -> bool,
    {
        let input = input.chars().collect::<Vec<_>>();
        let res = kras().parse(&input);
        if let Ok(KrasValue::List((_, ref res, _))) = res {
            if let Some(KrasValue::ListItem((item, _))) = res.get(0) {
                assert!(cmp_with(&item, &expected), "{:?} != {:?}", **item, *expected);
                return;
            }
        }
        assert!(false, "{:?} != {:?}", res, expected);
    }

    #[test]
    fn test_parse_str() {
        let tests = vec![
            (
                r#"{x:""}"#,
                KrasValue::RawList(vec![
                    KrasValue::List((
                        "{".to_string(),
                        vec![KrasValue::Pair((
                            Box::new(KrasValue::Ident("x".to_string())),
                            ":".to_string(),
                            Box::new(KrasValue::Str(('"', "".to_string(), "".to_string()))),
                            None,
                        ))],
                        "}".to_string(),
                    )),
                ]),
            ),
        ];
        for (s, exp) in tests {
            let res = parse_str(s, true, true, false);
            assert_eq!(res, exp);
        }
    }

    #[test]
    fn test_kras() -> () {
        let tests = vec![
            ("{}", KrasValue::List(("{".to_string(), vec![], "}".to_string()))),
            (
                "{a=>b}",
                KrasValue::List((
                    "{".to_string(),
                    vec![
                        KrasValue::ListItem((Box::new(KrasValue::Ident("a".to_string())), Some("=>".to_string()))),
                        KrasValue::ListItem((Box::new(KrasValue::Ident("b".to_string())), None)),
                    ],
                    "}".to_string(),
                )),
            ),
            (r#"b''"#, KrasValue::Str(('\'', "b".to_string(), "".to_string()))),
            (
                "{a=>b''}",
                KrasValue::List((
                    "{".to_string(),
                    vec![
                        KrasValue::ListItem((Box::new(KrasValue::Ident("a".to_string())), Some("=>".to_string()))),
                        KrasValue::ListItem((Box::new(KrasValue::Str(('\'', "b".to_string(), "".to_string()))), None)),
                    ],
                    "}".to_string(),
                )),
            ),
        ];
        for (input, expected) in tests {
            let input = input.chars().collect::<Vec<_>>();
            let res = kras().parse(&input).unwrap();
            assert_eq!(res, expected);
        }
    }

    #[test]
    fn test_unicode() {
        let tests = vec![
            (r#"["\u044f"]"#, KrasValue::Str(('"', "".to_string(), "я".to_string()))),
            (
                r#"["\u044f2"]"#,
                KrasValue::Str(('"', "".to_string(), "я2".to_string())),
            ),
        ];
        for (input, expected) in tests {
            check_single_value(&input, &expected);
        }
    }

    #[test]
    fn test_numbers() {
        let tests = vec![
            ("[1]", KrasValue::Num(OrdF64(1.0, "1".to_string()))),
            ("[123]", KrasValue::Num(OrdF64(123.0, "123".to_string()))),
            ("[0.123]", KrasValue::Num(OrdF64(0.123, "0.123".to_string()))),
            ("[0x1]", KrasValue::Num(OrdF64(1.0, "0x1".to_string()))),
            (
                "[0xdeadbeef]",
                KrasValue::Num(OrdF64(3735928559.0, "0xdeadbeef".to_string())),
            ),
            (
                "[0x7f1bcd0b0d40]",
                KrasValue::Num(OrdF64(139757380898112.0, "0x7f1bcd0b0d40".to_string())),
            ),
        ];
        for (input, expected) in tests {
            check_single_value_with(&input, &expected, |a, b| {
                if let KrasValue::Num(OrdF64(fa, sa)) = a {
                    let radix = if sa.len() >= 2 && sa.chars().skip(2).next().unwrap() == 'x' {
                        16
                    }
                    else {
                        10
                    };
                    if radix == 16 {
                        assert_eq!(u64::from_str_radix(&sa, radix).unwrap() as f64, *fa);
                    }
                    if let KrasValue::Num(OrdF64(fb, sb)) = b {
                        return fa == fb && sa == sb;
                    }
                }
                assert!(false, "invalid types: {:?} {:?}", a, b);
                false
            });
        }
    }
}
