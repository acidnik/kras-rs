use std::cmp::Ordering;
use std::collections::{HashMap, BinaryHeap};

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
    input: &'a Vec<char>,
    start: usize
}

impl<'a> DetectDataIter<'a> {
    pub fn new(input: &'a Vec<char>) -> Self {
        DetectDataIter {
            input: input,
            start: 0,
        }
    }
}

/*

input = "[{ [{}] ]"
stack = [
// pos char level all_level
    0 [ 0 0
    1 { 0 1
    2 [ 1 2
    3 { 1 3
    4 } 1 3
    5 ] 1 2
    6 ] 0 1

при каждой закрывающейся скобке можно попытаться найти соотв открывающую.
Если пришли к 0 0 - выдаем кусок
Иначе нужно попробовать пропустить символ либо с начала, либо с конца

Гипотеза:
Валидная строка та, где числа одинаковые

Будем писать засечки, чтобы потом быстро найти наибольшую строку.
в конце мы должны найти самую длинную строку, путь это будет prio queue.
Это значит что при закрытии мы должны найти самую дальнюю строку с той же сигнатурой. Может ли быть несколько одинаковых значений внутри строки, кроме откр-закр?
Пусть нет, тогда имеем отобр:
(a, b) => pos
при закрывании берем старый пос (если есть) и пишем в prq (len, start, end)
В конце, если не нашли 00 - отдаем из верха, гарантируем наилучший результат
Для проверки: сгенерить мусор вначале, ок, мусор. Проверить, что нашли ок или больше

[[]
 0 [ 0 0
 1 [ 1 1
 2 ] 1 1

[]]
 0 [ 0 0
 1 ] 0 0
 2 ] -1 -1

{[]
 0 { 0 0
 1 [ 0 1
 2 ] 0 1

[}]
 0 [ 0 0
 1 ] 0 0
 2 } -1 -1

*/

fn is_open(c: char) -> bool {
    c == '(' || c == '[' || c == '{'
}

fn is_close(c: char) -> bool {
    c == ')' || c == ']' || c == '}'
}

fn get_open(c: char) -> char {
    match c {
        ')' => '(',
        ']' => '[',
        '}' => '{',
        _ => panic!(format!("wrong close char {:?}", c))
    }
}

fn get_close(c: char) -> char {
    match c {
        '(' => ')',
        '[' => ']',
        '{' => '}',
        _ => panic!(format!("wrong close char {:?}", c))
    }
}

impl<'a> Iterator for DetectDataIter<'a> {
    type Item = (usize, &'a [char]); // start, chunk

    fn next(&mut self) -> Option<Self::Item> {
        if self.start >= self.input.len() {
            return None
        }
        let mut cnt_each = HashMap::<char, isize>::new();
        let mut all_cnt = 0;
        let mut sign_pos = HashMap::<(isize, isize, char), usize>::new(); // (cnt, all_cnt, open_char) => pos
        let mut pq = BinaryHeap::<CharPosition>::new();
        let mut escape = false;
        let mut str_char: Option<char> = None;
        // println!("begin {}", self.start);
        for (idx, c) in self.input[self.start..].iter().enumerate() {
            let idx = idx + self.start;
            let c = *c;
            if c == '\\' {
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
                let prev = sign_pos.insert((*cnt, all_cnt, c), idx);
                // assert_eq!(prev, None);
                if ! prev.is_none() {
                    // println!("seen {:?} = {:?} at {}", (*cnt, all_cnt, c), prev, idx);
                    // panic!("")
                }
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
            return Some((start, &self.input[start .. end+1]));
        }
        else {
            None
        }
    }
}

#[cfg(test)]
mod test {
    // use super::DetectDataIter;
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
            // ("({{[[{{[{}[{(({}{()()[[(((([([[][](){}()[]])]))))]]}))}]]}}]](){(]][[", vec![(0, "")]),
            // ("([}]{[]()([]{[{}(({}{{}[[]{({[[{[([[{}({}()([(([]))]))]])]}]]})}]}))]})}({(}", vec![(10, "{({[[{[([[{}({}()([(([]))]))]])]}]]})}")]),
        ];
        for (input, res) in cases {
            let input = input.chars().collect::<Vec<_>>();
            println!(">> {}", String::from_iter(input.iter()));
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

