// disable some warnings for debug build
#![cfg_attr(debug_assertions, allow(dead_code, unused_imports, unused_mut, unused_variables))]

use std::io::BufReader;
use std::io::BufRead;
use std::iter::FromIterator;
use std::str::{self, FromStr};

extern crate atty;

extern crate clap;
use clap::{Arg, App};

extern crate fileinput;
use fileinput::FileInput;

extern crate pom;

extern crate pretty;
use pretty::*;

use pretty::termcolor::{Color, ColorChoice, ColorSpec, StandardStream};

mod detect;
use detect::*;

mod pretty_value;
use pretty_value::*;

mod parse;
use parse::kras;


fn main() {
    let matches = App::new("Kras")
        .version("0.1.0")
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
            .default_value("auto")
            .possible_values(&["yes", "no", "auto"])
            .help("colorize output")
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
            .default_value("80")
        )
        .arg(Arg::with_name("input")
             .index(1)
             .multiple(true)
             .help("Input files or stdin")
        )
        .get_matches();
    let indent = usize::from_str(matches.value_of("indent").unwrap()).unwrap();
    let min_len = if indent == 0 { usize::MAX } else { usize::from_str(matches.value_of("min_len").unwrap()).unwrap() };
    let files = matches.values_of("input").map(|fs| fs.collect::<Vec<_>>()).unwrap_or_default();
    let color_choice = match matches.value_of("color").unwrap() {
        "yes" => ColorChoice::Always,
        "no" => ColorChoice::Never,
        "auto" => if atty::is(atty::Stream::Stdout) {
                ColorChoice::Auto
            }
            else {
                ColorChoice::Never
            }
        _ => unreachable!(),
    };
    let sort = matches.is_present("sort");
    let input = FileInput::new(&files);
    let reader = BufReader::new(input);
    for line in reader.lines() {
        match line {
            Ok(s) => {
                // TODO add flag to skip comments?
                // if s.starts_with("//") {
                //     continue
                // }
                let buf = s.chars().collect::<Vec<_>>();
                let mut start = 0;
                for (pos, data) in DetectDataIter::new(&buf) {
                    // println!("detect //////// {} //////////", String::from_iter(data));
                    let r = kras().parse(data);
                    if let Ok(mut r) = r {
                        print!("{}", String::from_iter(buf[start..pos].iter()));
                        start = pos + data.len();
                        r = r.postprocess(sort);
                        // println!("{} ===>>> {:?}", s, r);
                        r.to_doc(indent).render_colored(min_len, StandardStream::stdout(color_choice)).unwrap();
                    }
                    else {
                        // println!("err {:?}", r);
                    }
                }
                println!("{}", String::from_iter(buf[start..].iter()));
            }
            Err(err) => println!("{:?}", err),
        }
    }
}
