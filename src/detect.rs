use std::{
    cmp::Ordering,
    collections::{BinaryHeap, HashMap},
};

use crate::stopwatch::Stopwatch;

/*
Some explanation on how it works:
Scan input. Skip strings and escape chars

if the char is one of ( [ { - increment per-char counter and all-counter
the per-char cnt, all cnt and the char itself make the "signature". Put it to the map:
(signature) => position of the char

if the char is on of ) ] } - decr per-char and all-char counters. If both counters are 0 -
we found a valid sequence. Yield it. Otherwise we look for the same signature:

            v -- we a here
input = [ [ ]
signatures:
(0 0 [) => 0
(1 1 [) => 1

current signature is also (1 1 [)
so the current best candidate for valid data starts at position 1

put the (start, end, length) of candidate to priority queue, sorted by length

At the end, the longest candidate is yielded

This is O(nlogn) at worst case


*/

// open pos, close pos, len
#[derive(Debug)]
struct CharPosition(usize, usize, usize);

impl Ord for CharPosition {
    fn cmp(&self, other: &CharPosition) -> Ordering {
        (usize::MAX - self.0, self.2).cmp(&(usize::MAX - other.0, other.2))
    }
}

impl PartialEq for CharPosition {
    fn eq(&self, other: &Self) -> bool {
        self.2 == other.2
    }
}

impl Eq for CharPosition {}

impl PartialOrd for CharPosition {
    fn partial_cmp(&self, other: &CharPosition) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

pub struct DetectDataIter<'a> {
    input: &'a [char],
    start: usize,
    // queue of best possible matches
    pq:    BinaryHeap<CharPosition>,
}

impl<'a> DetectDataIter<'a> {
    pub fn new(input: &'a [char]) -> Self {
        DetectDataIter {
            input,
            start: 0,
            pq: BinaryHeap::<CharPosition>::new(),
        }
    }
}

pub fn is_open(c: char) -> bool {
    c == '(' || c == '[' || c == '{' || c == '<'
}

pub fn is_close(c: char) -> bool {
    c == ')' || c == ']' || c == '}' || c == '>'
}

pub fn get_open(c: char) -> char {
    match c {
        ')' => '(',
        ']' => '[',
        '}' => '{',
        '>' => '<',
        _ => panic!("wrong close char {c:?}"),
    }
}

#[allow(dead_code)]
fn get_close(c: char) -> char {
    match c {
        '(' => ')',
        '[' => ']',
        '{' => '}',
        '<' => '>',
        _ => panic!("wrong open char {c:?}"),
    }
}

impl<'a> Iterator for DetectDataIter<'a> {
    type Item = (usize, &'a [char]); // start, chunk

    fn next(&mut self) -> Option<Self::Item> {
        if self.start >= self.input.len() {
            // trace!("out: {} >= {}", self.start, self.input.len());
            return None;
        }
        let _stopwatch = Stopwatch::new("detect", 0);
        // balance (open-close) of each bracket
        let mut cnt_each = HashMap::<char, isize>::new();
        // balance of all open-close brackets
        let mut all_cnt = 0;
        // signature and position of signature
        let mut sign_pos = HashMap::<(isize, isize, char), usize>::new(); // (cnt, all_cnt, open_char) => pos
        let mut escape = false;
        let mut str_char: Option<char> = None;
        // trace!("begin {}", self.start);

        for (idx, c) in self.input[self.start..].iter().enumerate() {
            let idx = idx + self.start;
            let c = *c;
            if str_char.is_some() && c == '\\' {
                escape = true;
                continue;
            }
            if escape {
                escape = false;
                continue;
            }
            if c == '\'' || c == '"' {
                if let Some(sc) = str_char {
                    if sc == c {
                        str_char = None;
                    }
                }
                else {
                    str_char = Some(c)
                }
            }
            if str_char.is_some() {
                continue;
            }
            if is_open(c) {
                let cnt = cnt_each.entry(c).or_insert(0);
                sign_pos.insert((*cnt, all_cnt, c), idx);
                *cnt += 1;
                all_cnt += 1;
                // trace!("open {:?}: {} {} {:?}", c, all_cnt, cnt, sign_pos);
            }
            else if is_close(c) && (c != '>' || idx == 0 || self.input[idx - 1] != '=') {
                // hack: do not treat '=>' as a part of a bracket sequence
                let op = get_open(c);
                // trace!("cnt_each = {:?}; all_cnt = {}", cnt_each, all_cnt);
                let cnt = cnt_each.entry(op).or_insert(0);
                all_cnt -= 1;
                *cnt -= 1;
                if let Some(pos) = sign_pos.get(&(*cnt, all_cnt, op)) {
                    self.pq.push(CharPosition(*pos, idx, idx - pos));
                }
                // trace!("at close {:?} ||: get {:?} {:?}", c, (*cnt, all_cnt), sign_pos);
            }
        }
        // trace!("at end: {:?}", self.pq);
        // reached the end of str
        while let Some(pos) = self.pq.pop() {
            // trace!("at end: pop {:?}", pos);
            let (start, end) = (pos.0, pos.1);
            if start < self.start {
                // trace!("at end: pos {:?} SKIP", pos);
                continue;
            }
            self.start = end + 1;
            return Some((start, &self.input[start..end + 1]));
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
            // fuck
            // ("[1, 2, 3] {[} (4, 5, 6) ]", vec![(0, "[1, 2, 3]"), (14, "(4, 5, 6)")]),
            (r#"[ "\"]" ]"#, vec![(0, r#"[ "\"]" ]"#)]),
            (r#"[[ "\"]" ]"#, vec![(1, r#"[ "\"]" ]"#)]),
        ];
        for (t, res) in cases {
            let input = t.chars().collect::<Vec<_>>();
            let d = DetectDataIter::new(&input).collect::<Vec<_>>();
            // trace!("test: {} -> {:?}", t, d);
            assert_eq!(d.len(), res.len());
            for ((tn, t), (rn, r)) in res.iter().zip(d.iter()) {
                assert_eq!(t.to_string(), String::from_iter(r.iter()));
                assert_eq!(tn, rn);
            }
        }
    }

    fn dumb(s: &str) -> Vec<String> {
        // trace!("begin dumb {}", s);
        let mut res = Vec::new();
        let s = s.chars().collect::<Vec<_>>();
        // 'start: for i in 0..s.len() {
        let mut i = 0;
        'start: while i < s.len() {
            let a = s[i];
            if !is_open(a) {
                i += 1;
                continue;
            }
            let mut q = VecDeque::<char>::new();
            q.push_back(a);
            for j in i + 1..s.len() {
                let b = s[j];
                if is_open(b) {
                    q.push_back(b);
                    continue;
                }
                // b is close
                if q.is_empty() {
                    i += 1;
                    continue 'start;
                }
                let x = q.pop_back().unwrap();
                if get_close(x) != b {
                    i += 1;
                    continue 'start;
                }

                if q.is_empty() {
                    let r = String::from_iter(s[i..=j].iter());
                    i = j + 1;
                    res.push(r);
                    continue 'start;
                }
            }
            i += 1
        }
        res
    }

    // gotta be sure
    // #[test]
    fn test_dumb() {
        init();

        let tests = [
            ("[{}]", vec![("[{}]")]),
            ("[]", vec![("[]")]),
            ("[[]", vec![("[]")]),
            ("[{[{}]]", vec![("[{}]")]),
            ("[[]][{}]", vec![("[[]]"), ("[{}]")]),
            ("{>}", vec![]),
            (")[{}]", vec![("[{}]")]),
            ("{}{}", vec![("{}"), ("{}")]),
            ("[}{]", vec![]),
        ];
        for (t, exp) in tests {
            let res = dumb(t);
            assert_eq!(res, exp);
        }
    }
}
