use std::fs::File;
use std::io::{self, Lines};
use std::path::PathBuf;
use std::error;
use std::io::BufReader;
use std::io::BufRead;

enum LinesReader {
    File(Lines<BufReader<File>>),
    Stdin(Lines<StdinLock>)
}

impl LinesReader {
    fn from_file(f: File) -> Self {
        File(BufReader::new(f).lines())
    }
    fn from_stdin() -> Self {
        Stdin(io.stdin().lock().lines())
    }
}

struct IterInput {
    input_files: Vec<PathBuf>,
    file: Option<LinesReader>,
    idx: usize,
    input_error: bool,
}

impl IterInput {
    fn new(input: &Vec<String>) -> Self {
        IterInput {
            input_files: input.iter().map(|f| PathBuf::from(f)).collect(),
            file: None,
            idx: 0,
            input_error: false,
        }
    }
    fn open_next(&mut self) -> Result<(), Box<dyn error::Error>> {
        if self.input_files.len() == 0 {
            // self.file = Some(Box::new(io::stdin().lock()).lines
            self.file = LinesReader::from_stdin()
            return Ok(());
        }
        let file = File::open(&self.input_files[self.idx]);
        if file.is_err() {
            self.input_error = true;
            return Err(Box::new(file.err().unwrap()))
        }
        self.file = Some(BufReader::new(file.unwrap()).lines());
        self.idx += 1;
        Ok(())
    }
    fn has_next_file(&self) -> bool {
        self.idx < self.input_files.len()
    }
}

impl Iterator for IterInput {
    type Item = Result<String, Box<dyn error::Error>>;
    fn next(&mut self) -> Option<Self::Item> {
        if self.input_error {
            return None
        }
        // if self.file.is_none() {
        //     if self.has_next_file() {
        //         match self.open_next() {
        //             Ok(()) => {},
        //             Err(err) => return Some(Err(err)),
        //         }
        //     }
        //     else {
        //         return None
        //     }
        // }
        loop {
            let line = self.file.as_mut().map(|l| l.next());
            if line.is_none() {
                if self.has_next_file() {
                    if let Err(err) = self.open_next() {
                        return Some(Err(err))
                    }
                }
                else {
                    break
                }
            }
        }
        

        return Some(Ok("".to_string()))
    }
}
