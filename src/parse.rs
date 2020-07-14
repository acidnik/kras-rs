use std::iter::FromIterator;
use std::str::FromStr;

use pom::parser::*;

use crate::pretty_value::*;

fn space<'a>() -> Parser<'a, char, ()> {
    one_of(" \t\r\n").repeat(0..).discard()
}

fn ident<'a>() -> Parser<'a, char, String> { 
    let first = is_a(|c: char| c.is_alphabetic()) | one_of("_%$@\\");

    fn alnum<'a>() -> Parser<'a, char, String> {
        let alnum = is_a(|c: char| c.is_alphanumeric()) | one_of("_%$@\\");
        alnum.collect().map(String::from_iter)
    }
    let dot = sym('.') | sym(':') | sym('-');

    // [a-z] [a-z0-9]* ([:.]+[a-z0-9]+)*
    let ident = first + alnum().repeat(0..) + (dot.repeat(1..) + alnum().repeat(1..)).repeat(0..) - space();
    
    // wtf: -space() not working here (got consumed), so trim_end
    ident.collect().map(String::from_iter).map(|s| s.trim_end().to_string())
}

fn number<'a>() -> Parser<'a, char, (f64, String)> {
    let integer = (one_of("123456789") - one_of("0123456789").repeat(0..)) | sym('0');
    let frac = sym('.') + one_of("0123456789").repeat(1..);
    let exp = one_of("eE") + one_of("+-").opt() + one_of("0123456789").repeat(1..);
    let number = sym('-').opt() + integer + frac.opt() + exp.opt();
    let repr = number.collect().map(String::from_iter);
    repr.convert(|s| f64::from_str(&s).and_then(|n| Ok((n, s))) )
}

fn json_unicode<'a>() -> Parser<'a, char, char> {
    let hex = one_of("0123456789abcdefABCDEF");
    let ch = sym('u') * hex.repeat(1..).map(String::from_iter);
    ch.convert(|s| u32::from_str_radix(&s, 16)).convert(|n| std::char::from_u32(n).ok_or("not a valid unicode"))
}

fn special_char<'a>() -> Parser<'a, char, char> {
    json_unicode() | sym('\\') | sym('/') | sym('"')
        | sym('b').map(|_|'\x08') | sym('f').map(|_|'\x0C')
        | sym('n').map(|_|'\n') | sym('r').map(|_|'\r') | sym('t').map(|_|'\t')
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
    let string = qqstring() | qstring();

    string.map(|(a, b)| KrasValue::Str((a, b)))
}

fn pair_delim<'a>() -> Parser<'a, char, String> {
    let delim = space() * (seq(&[':']) | seq(&['=','>']) | seq(&['='])) - space();
    delim.map(String::from_iter)
}

fn array_delim<'a>() -> Parser<'a, char, String> {
    let delim = space() * sym(',') - space();
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
    value() | ident().map(KrasValue::Ident)
}

fn value<'a>() -> Parser<'a, char, KrasValue> {
    (
        string()
        | number().map(|(n, r)| KrasValue::Num(OrdF64(n, r)))
        | constructor()
        | array().map(|(s, arr, c)| KrasValue::List((s, arr, c)))
    ) - space()
}

pub fn kras<'a>() -> Parser<'a, char, KrasValue> {
    space() * value() - end()
}
