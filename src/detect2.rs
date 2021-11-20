use std::collections::VecDeque;

// use std::iter::FromIterator;
use crate::{
    detect::{get_open, is_close, is_open},
    stopwatch::Stopwatch,
};

pub struct DetectDataV2<'a> {
    input: &'a [char],
    start: usize,
    // pos: Vec<usize>,
}

impl<'a> DetectDataV2<'a> {
    pub fn new(input: &'a [char]) -> Self {
        DetectDataV2 {
            input: input,
            start: 0,
            // pos: Vec::new(),
        }
    }
}

impl<'a> Iterator for DetectDataV2<'a> {
    type Item = (usize, &'a [char]); // start, chunk

    fn next(&mut self) -> Option<Self::Item> {
        let mut i = self.start;
        let mut str_char: Option<char> = None;
        let _stopwatch = Stopwatch::new("detect", 0);
        'start: while i < self.input.len() {
            let a = self.input[i];
            if a == '\'' || a == '"' {
                if let Some(sc) = str_char {
                    if sc == a {
                        str_char = None;
                    }
                }
                else {
                    str_char = Some(a)
                }
            }
            if str_char.is_some() {
                i += 1;
                continue;
            }

            if a == '>' && i > 0 && self.input[i - 1] == '=' {
                i += 1;
                continue;
            }

            if !is_open(a) {
                i += 1;
                continue;
            }
            let mut stack = VecDeque::<char>::new();
            stack.push_back(a);
            let mut str_char: Option<char> = None;
            for j in i + 1..self.input.len() {
                let b = self.input[j];
                if b == '\'' || b == '"' {
                    if let Some(sc) = str_char {
                        if sc == b {
                            str_char = None;
                        }
                    }
                    else {
                        str_char = Some(b)
                    }
                }
                if str_char.is_some() {
                    continue;
                }

                if b == '>' && j > 0 && self.input[j - 1] == '=' {
                    continue;
                }

                if is_open(b) {
                    stack.push_back(b);
                    continue;
                }
                if !is_close(b) {
                    continue;
                }
                // b is close
                if stack.is_empty() {
                    i += 1;
                    continue 'start;
                }
                let x = stack.pop_back().unwrap();
                if x != get_open(b) {
                    i += 1;
                    continue 'start;
                }

                if stack.is_empty() {
                    self.start = j + 1;
                    return Some((i, &self.input[i..j + 1]));
                }
            }
            i += 1
        }
        None
    }
}

#[cfg(test)]
mod test {
    fn init() {
        let _ = env_logger::builder().is_test(true).try_init();
    }

    use std::{
        collections::{HashSet, VecDeque},
        iter::FromIterator,
    };

    use permutator::XPermutationIterator;
    use rand::prelude::*;

    use super::*;

    #[test]
    fn test_detect() -> () {
        init();
        let cases = vec![
            ("[{}]", vec![(0, "[{}]")]),
            ("[]", vec![(0, "[]")]),
            ("[[]", vec![(1, "[]")]),
            ("[{ [{}] ]", vec![(3, "[{}]")]),
            ("[[]]  [{}]", vec![(0, "[[]]"), (6, "[{}]")]),
            ("(1, 2, '{')", vec![(0, "(1, 2, '{')")]),
            ("[']']", vec![(0, "[']']")]),
            ("'[]'", vec![]),
            ("{a=>b}", vec![(0, "{a=>b}")]),
            ("<class 'str'>", vec![(0, "<class 'str'>")]),
            ("", vec![]),
            (") [{}]", vec![(2, "[{}]")]),
            (r#"[ "]" ]"#, vec![(0, r#"[ "]" ]"#)]),
            (
                r#""a": {"b": 1 }, "c": {"d": "e", }"#,
                vec![(5, r#"{"b": 1 }"#), (21, r#"{"d": "e", }"#)],
            ),
            ("{}{a:b}", vec![(0, "{}"), (2, "{a:b}")]),
            ("[1, 2, 3] {[} (4, 5, 6) ]", vec![(0, "[1, 2, 3]"), (14, "(4, 5, 6)")]),
        ];
        for (t, res) in cases {
            let input = t.chars().collect::<Vec<_>>();
            let d = DetectDataV2::new(&input).collect::<Vec<_>>();
            // trace!("test: {} -> {:?}", t, d);
            assert_eq!(d.len(), res.len());
            for ((tn, t), (rn, r)) in res.iter().zip(d.iter()) {
                assert_eq!(t.to_string(), String::from_iter(r.iter()));
                assert_eq!(tn, rn);
            }
        }
    }
}
