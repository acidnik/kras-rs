use std::cmp::Ordering;

use pretty::*;
use pretty::termcolor::{Color, ColorSpec};

#[derive(Debug, Clone)]
pub struct OrdF64(pub f64);

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
pub enum KrasValue {
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
    pub fn postprocess(&mut self, sort: bool) {
        // convert lists to dicts, sort
        match self {
            KrasValue::List((ref o, ref mut items, ref c)) => {
                // if list looks like python dict / js object / perl hash / etc: convert ListItems
                // to Pairs
                // {key1: val1, key2: val2} => [ (key1, :), (val1, ,), (key2, :), (val2, ())] => [ (key1, :, val1, ,), (key2, :, val2, ()) ]
                let mut is_dict = true;
                for (i, item) in items.iter_mut().enumerate() {
                    item.postprocess(sort);
                    if i % 2 == 0 {
                        // each even list item delimeter must be a dict separator
                        if let KrasValue::ListItem((_, d)) = item {
                            is_dict = match d {
                                Some(d) => {
                                    d == "=>" || d == ":" || d == "="
                                },
                                None => {
                                    false
                                },
                            };
                            if !is_dict {
                                break
                            }
                        }
                    }
                }
                if is_dict {
                    // TODO can it be done without clone?
                    let mut res = Vec::new();
                    for kv in items.chunks_mut(2) {
                        if let [k, v] = kv {
                            if let KrasValue::ListItem(ref mut k) = k {
                                if let KrasValue::ListItem(ref mut v) = v {
                                    res.push(KrasValue::Pair(( 
                                        k.0.clone(),
                                        k.1.clone().unwrap(),
                                        v.0.clone(),
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
                    *self = KrasValue::List((o.to_string(), res, c.to_string()));
                }
            },
            KrasValue::Constructor(ref mut kv) => {
                let (ref mut ident, ref mut args) = kv;
                args.postprocess(sort);
                *self = KrasValue::Constructor((ident.clone(), args.clone()))
            }
            KrasValue::ListItem((ref mut val, _)) => {
                val.postprocess(sort)
            }
            _ => {},
        }
    }
    fn fix_comma(&self, list: &mut Vec<KrasValue>) {
        // {"2": 2, "1": 1} => sort => {"1": 1<no comma> "2": 2,<extra comma>} 
        // => fix => {"1": 1,<add comma> "2": 2<remove comma> } => {"1": 1, "2": 2}
        let len = list.len();
        for (i, item) in list.iter_mut().enumerate() {
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
    fn kv_spaces(&self, d: String) -> RcDoc<ColorSpec> {
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

    pub fn to_doc(&self, indent: usize, is_key: bool) -> RcDoc<ColorSpec> {
        let nest = indent as isize; // why tf _i_size?
        match self {
            KrasValue::Str((q, s)) => RcDoc::as_string(q.to_string() + s + &q.to_string())
                .annotate(ColorSpec::new().set_fg(Some(Color::Red)).set_bold(is_key).clone()),
            KrasValue::Ident(s) => RcDoc::as_string(s)
            // KrasValue::Ident(s) => RcDoc::as_string(format!("<{}>",s ))
                .annotate(ColorSpec::new().set_fg(Some(Color::Blue)).clone()),
            KrasValue::List((op, it, cl)) => {
                RcDoc::text(op)
                    .annotate(ColorSpec::new().set_bold(true).clone())
                    .append(RcDoc::nil()
                        .append(RcDoc::line_())
                        .nest(nest)
                        .append(RcDoc::intersperse(it.iter().map(|x| x.to_doc(indent, false)), RcDoc::line_())
                            .nest(nest)
                            .append(Doc::line_())
                        )
                        .group()
                    )
                    .append(RcDoc::nil()
                        .append(cl)
                        .annotate(ColorSpec::new().set_bold(true).clone())
                    )
            }
            KrasValue::Pair((k, d, v, d2)) => {
                RcDoc::nil()
                    .append(
                        RcDoc::nil()
                        // key
                        .append(k.to_doc(indent, true))
                        // kv delim
                        .append(self.kv_spaces(d.to_string()))
                        .group()
                    )
                    .append(RcDoc::softline_())
                    .nest(nest)
                    .append(
                        // value
                        RcDoc::nil()
                        .append(v.to_doc(indent, false))
                        // list delim
                        .append(d2.clone().map_or(RcDoc::nil(), |d| self.kv_spaces(d)))
                        .group()
                    )
            }.group(),
            KrasValue::ListItem((v, d)) => {
                RcDoc::nil()
                    .append(v.to_doc(indent, false))
                    .append(d.clone().map_or(RcDoc::nil(), |d| self.kv_spaces(d)))
            }
            KrasValue::Num(OrdF64(n)) => RcDoc::as_string(n),
            KrasValue::Constructor((id, args)) => {
                RcDoc::nil()
                    .append(id.to_doc(indent, false))
                    .append(args.to_doc(indent, false))
                    .group()
            }
        }.group()
    }
}


