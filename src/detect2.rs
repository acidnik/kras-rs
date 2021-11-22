use std::collections::VecDeque;

use crate::{
    detect::{get_open, is_close, is_open},
    stopwatch::Stopwatch,
};

pub struct DetectDataV2<'a> {
    input: &'a [char],
    start: usize,
}

impl<'a> DetectDataV2<'a> {
    pub fn new(input: &'a [char]) -> Self {
        DetectDataV2 {
            input: input,
            start: 0,
        }
    }

    fn next_char(&mut self, idx: usize) -> Option<(usize, char)> {
        // move to the next char, skipping all non-relevant chars
        // returns index of next char and the char itself
        let mut str_char: Option<char> = None;
        let mut escape = false;
        for i in idx..self.input.len() {
            let a = self.input[i];

            if str_char.is_some() && a == '\\' {
                escape = true;
                continue;
            }
            if escape {
                escape = false;
                continue;
            }

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
                continue;
            }

            if a == '>' && i > 0 && self.input[i - 1] == '=' {
                // hack for =>
                continue;
            }

            if !(is_open(a) || is_close(a)) {
                continue;
            }

            return Some((i, a));
        }
        None
    }
}

impl<'a> Iterator for DetectDataV2<'a> {
    type Item = (usize, &'a [char]); // start, chunk

    fn next(&mut self) -> Option<Self::Item> {
        let _stopwatch = Stopwatch::new("detect", 0);

        let mut i = self.start;

        'start: while i < self.input.len() {
            let next = self.next_char(i);
            // trace!("i={} => {:?}", i, next);
            if next.is_none() {
                return None;
            }
            let (next_i, a) = next.unwrap();
            i = next_i;

            if !is_open(a) {
                i += 1;
                continue;
            }
            let mut stack = VecDeque::<char>::new();
            stack.push_back(a);
            let mut j = i + 1;
            while j < self.input.len() {
                let next = self.next_char(j);
                // trace!("j={} => {:?}", j, next);
                if next.is_none() {
                    break;
                }
                let (next_j, b) = next.unwrap();
                j = next_j;

                if is_open(b) {
                    stack.push_back(b);
                    j += 1;
                    continue;
                }
                // b is close
                if stack.is_empty() {
                    i += 1;
                    continue 'start;
                }
                let x = stack.pop_back().unwrap();
                // trace!("pop {} <> {}", x, b);
                if x != get_open(b) {
                    i += 1;
                    continue 'start;
                }

                if stack.is_empty() {
                    self.start = j + 1;
                    return Some((i, &self.input[i..j + 1]));
                }
                j += 1
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
            (r#"[ "\"]" ]"#, vec![(0, r#"[ "\"]" ]"#)]),
            (r#"[[ "\"]" ]"#, vec![(1, r#"[ "\"]" ]"#)]),
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
