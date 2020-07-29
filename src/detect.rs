use std::cmp::Ordering;
use std::collections::{HashMap, BinaryHeap};

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
        self.2.cmp(&other.2)
    }
}

impl PartialEq for CharPosition {
    fn eq(&self, other: &Self) -> bool {
        self.2 == other.2
    }
}

impl Eq for CharPosition {
}

impl PartialOrd for CharPosition {
    fn partial_cmp(&self, other: &CharPosition) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

pub struct DetectDataIter<'a> {
    input: &'a [char],
    start: usize
}

impl<'a> DetectDataIter<'a> {
    pub fn new(input: &'a [char]) -> Self {
        DetectDataIter {
            input,
            start: 0,
        }
    }
}


fn is_open(c: char) -> bool {
    c == '(' || c == '[' || c == '{' || c == '<'
}

fn is_close(c: char) -> bool {
    c == ')' || c == ']' || c == '}' || c == '>'
}

fn get_open(c: char) -> char {
    match c {
        ')' => '(',
        ']' => '[',
        '}' => '{',
        '>' => '<',
        _ => panic!(format!("wrong close char {:?}", c))
    }
}

#[allow(dead_code)]
fn get_close(c: char) -> char {
    match c {
        '(' => ')',
        '[' => ']',
        '{' => '}',
        '<' => '>',
        _ => panic!(format!("wrong open char {:?}", c))
    }
}

impl<'a> Iterator for DetectDataIter<'a> {
    type Item = (usize, &'a [char]); // start, chunk

    fn next(&mut self) -> Option<Self::Item> {
        if self.start >= self.input.len() {
            return None
        }
        let _stopwatch = Stopwatch::new("detect", 0);
        // balance (open-close) of each bracket
        let mut cnt_each = HashMap::<char, isize>::new();
        // balance of all open-close brackets
        let mut all_cnt = 0;
        // signature and position of signature
        let mut sign_pos = HashMap::<(isize, isize, char), usize>::new(); // (cnt, all_cnt, open_char) => pos
        // queue of best possible matches
        let mut pq = BinaryHeap::<CharPosition>::new();
        let mut escape = false;
        let mut str_char: Option<char> = None;
        // println!("begin {}", self.start);
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
                let _prev = sign_pos.insert((*cnt, all_cnt, c), idx);
                // if ! prev.is_none() {
                //     println!("sing = {:?}", sign_pos);
                //     println!("seen {:?} = {:?} at {}", (*cnt, all_cnt, c), prev, idx);
                //     panic!("")
                // }
                *cnt += 1;
                all_cnt += 1;
            }
            else if is_close(c) {
                let op = get_open(c);
                // println!("cnt_each = {:?}; all_cnt = {}", cnt_each, all_cnt);
                let cnt = cnt_each.entry(op).or_insert(0);
                all_cnt -= 1;
                *cnt -= 1;
                if *cnt == 0 && all_cnt == 0 {
                    if let Some(pos) = sign_pos.get(&(*cnt, all_cnt, op)) {
                        self.start = idx + 1;
                        // println!("end1 {}", idx+1);
                        return Some((*pos, &self.input[*pos .. idx+1]));
                    }
                }
                // println!("at close || {:?} ||: get {:?} {:?}", self.input, (*cnt, all_cnt), sign_pos);
                if let Some(pos) = sign_pos.get(&(*cnt, all_cnt, op)) {
                    pq.push(CharPosition(*pos, idx, idx-pos));
                }
            }
        }
        // println!("at end: {:?}", pq);
        // reached the end of str
        if let Some(CharPosition(start, end, _)) = pq.pop() {
            self.start = end+1;
            // println!("end2 {}", end+1);
            Some((start, &self.input[start .. end+1]))
        }
        else {
            None
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use std::iter::FromIterator;
    use rand::prelude::*;
    #[test]
    fn test_detect() -> () {
        let cases = vec![
            ("[]", vec![(0, "[]")]),
            ("[[]", vec![(1, "[]")]),
            ("[{ [{}] ]", vec![(3, "[{}]")]),
            ("[[]]  [{}]", vec![(0, "[[]]"), (6, "[{}]")]),
            ("(1, 2, '{')", vec![(0, "(1, 2, '{')")]),
            ("[']']", vec![(0, "[']']")]),
            ("'[]'", vec![]),
            // ("", vec![]),
            // ("({{[[{{[{}[{(({}{()()[[(((([([[][](){}()[]])]))))]]}))}]]}}]](){(]][[", vec![(0, "")]),
            // ("([}]{[]()([]{[{}(({}{{}[[]{({[[{[([[{}({}()([(([]))]))]])]}]]})}]}))]})}({(}", vec![(10, "{({[[{[([[{}({}()([(([]))]))]])]}]]})}")]),
        ];
        for (input, res) in cases {
            let input = input.chars().collect::<Vec<_>>();
            println!(">> '{}'", String::from_iter(input.iter()));
            let d = DetectDataIter::new(&input).collect::<Vec<_>>();
            println!("{:?}", d);
            assert_eq!(d.len(), res.len());
            for ((tn, t), (rn, r)) in res.iter().zip(d.iter()) {
                assert_eq!(tn, rn);
                assert_eq!(t.to_string(), String::from_iter(r.iter()));
            }
        }
    }

    fn rnd_chars() -> Vec<char> {
        let mut rng = thread_rng();
        let num = rng.gen_range(0, 100);
        let rc = vec!['(', '[', '{', ')', ']', '}'];
        let mut res = Vec::new();
        for _ in 0..num {
            res.push(rc[rng.gen_range(0, rc.len())])
        }
        res
    }

    fn valid_chars() -> Vec<char> {
        let mut rng = thread_rng();
        let mut res = Vec::new();
        let mut stack = Vec::new();
        let rc = vec!['(', '[', '{'];
        for _ in 0..rng.gen_range(1, 500) {
            let c = rc[rng.gen_range(0, 3)];
            res.push(c); // [{{}}{}
            stack.push(c);
            if rng.gen_range(0, 100) < 30 {
                let op = stack.pop().unwrap();
                res.push(get_close(op))
            }
        }
        while let Some(op) = stack.pop() {
            res.push(get_close(op))
        }
        res
    }

    #[test]
    fn test_fuzzy() -> () {
        let iters = 1000;
        for _ in 0..iters {
            let mut pre_chars = rnd_chars();
            let mut mid_chars = valid_chars();
            let mut last_chars = rnd_chars();
            let mut input = Vec::new();
            input.append(&mut pre_chars);
            input.append(&mut mid_chars);
            input.append(&mut last_chars);
            println!("BEGIN {:?}", String::from_iter(input.iter()));
            let d = DetectDataIter::new(&input).collect::<Vec<_>>();
            assert!(d.len() >= 1);
        }
    }
}

