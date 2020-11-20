// disable some warnings for debug build
#![cfg_attr(debug_assertions, allow(dead_code, unused_imports, unused_mut, unused_variables, unreachable_code))]

use std::io::BufReader;
use std::io::BufRead;
use std::iter::FromIterator;
use std::str::FromStr;
use std::io::Write;
use std::collections::BinaryHeap;
use std::cmp::Reverse;
use std::thread;

use std::env;
extern crate chrono;
#[macro_use]
extern crate log;
extern crate env_logger;

extern crate atty;

extern crate clap;
use clap::{Arg, App};

extern crate fileinput;
use fileinput::FileInput;

extern crate crossbeam;

extern crate pom;

extern crate pretty;

extern crate num_cpus;

use crossbeam::crossbeam_channel::bounded;

use pretty::termcolor::ColorChoice;

mod detect;
use detect::*;

mod pretty_value;

mod parse;
use parse::kras;

mod stopwatch;
use stopwatch::Stopwatch;


fn main() {
    let matches = App::new("Kras")
        .version("0.1.0")
        .author("Nikita Bilous <nikita@bilous.me>")
        .about("Detect, highlight and pretty print almost any structured data inside plain text")
        .arg(Arg::with_name("indent")
             .short("i")
             .long("indent")
             .help("indentation. 0 to disable (colorization is stil performed)")
             .default_value("2")
        )
        .arg(Arg::with_name("color")
            .short("c")
            .long("color")
            .default_value("auto")
            .possible_values(&["yes", "no", "auto"])
            .help("colorize output")
        )
        .arg(Arg::with_name("force_color")
             .short("C")
             .long("force-color")
             .help("alias for --color yes")
         )
        .arg(Arg::with_name("sort")
            .short("s")
            .long("sort")
            .help("sort keys")
        )
        .arg(Arg::with_name("jobs")
            .short("j")
            .takes_value(true)
            .help("number of parallel jobs. Default is num_cpus")
        )
        .arg(Arg::with_name("width")
            .short("w")
            .long("width")
            .help("maximum width of output")
            .default_value("80")
        )
        .arg(Arg::with_name("debug")
            .long("debug")
            .help("debug mode")
        )
        .arg(Arg::with_name("input")
             .index(1)
             .multiple(true)
             .help("Input files or stdin")
        )
        .get_matches();
    init_logger(if matches.is_present("debug") {2} else {0} );
    let indent = usize::from_str(matches.value_of("indent").unwrap()).unwrap();
    let min_len = if indent == 0 { std::usize::MAX } else { usize::from_str(matches.value_of("width").unwrap()).unwrap() };
    let files = matches.values_of("input").map(|fs| fs.collect::<Vec<_>>()).unwrap_or_default();
    let color_choice =
        if matches.is_present("force_color") {
            ColorChoice::Always
        }
        else { 
            match matches.value_of("color").unwrap() {
            "yes" => ColorChoice::Always,
            "no" => ColorChoice::Never,
            "auto" => if atty::is(atty::Stream::Stdout) {
                    ColorChoice::Auto
                }
                else {
                    ColorChoice::Never
                }
            _ => unreachable!(),
        }
    };
    let sort = matches.is_present("sort");
    let jobs = match matches.value_of("jobs") {
        Some(v) => usize::from_str(v).unwrap(),
        None => num_cpus::get(),
    };

    let input = FileInput::new(&files);
    let reader = BufReader::new(input);
   
    // input lines => input_sender => [worker] input_receiver => output_sender => [printer] output_receiver 

    let (output_sender, output_receiver) = bounded(jobs*128);
    let (input_sender, input_receiver) = bounded::<(usize, String)>(jobs*128);

    let mut next_line_num = 0;
    let mut output_queue = BinaryHeap::<Reverse<(usize, String)>>::new();
    let mut max_qlen = 0;
    let printer = thread::spawn(move || {
        while let Ok((i, line)) = output_receiver.recv() {
            max_qlen = usize::max(max_qlen, output_queue.len());
            if i == next_line_num {
                println!("{}", line);
                next_line_num += 1;
            }
            else {
                output_queue.push(Reverse((i, line)));
            }

            if let Some(Reverse((i, line))) = output_queue.peek() {
                if *i == next_line_num {
                    println!("{}", line);
                    next_line_num += 1;
                    output_queue.pop();
                }
            }
        }
        while let Some(Reverse((_, line))) = output_queue.pop() {
            println!("{}", line);
        }
        debug!("max queue len = {max_qlen}", max_qlen=max_qlen)
    });

    (0..jobs).map(|_| {
        let input_receiver = input_receiver.clone();
        let output_sender = output_sender.clone();
        thread::spawn(move || {
            while let Ok((i, s)) = input_receiver.recv() {
                // TODO add flag to skip comments?
                // if s.starts_with("//") {
                //     continue
                // }
                let mut res = Vec::new();
                let buf = s.chars().collect::<Vec<_>>();
                let mut start = 0;
                for (pos, data) in DetectDataIter::new(&buf) {
                    debug!("DETECT: {}", String::from_iter(data));
                    let mut stopwatch = Stopwatch::new("parse", 0);
                    let r = kras().parse(data);
                    stopwatch.stop();
                    if let Ok(mut r) = r {
                        debug!("PARSED: {:?}", r);
                        res.push(String::from_iter(buf[start..pos].iter()));
                        start = pos + data.len();
                        let mut stopwatch = Stopwatch::new("postprocess", 0);
                        r.postprocess(sort);
                        stopwatch.stop();
                        debug!("POSTPROC: {:?}", r);
                        let mut stopwatch = Stopwatch::new("pretty", 0);
                        let doc = r.to_doc(indent, false);
                        stopwatch.stop();
                        let mut buffer = match color_choice {
                            ColorChoice::Always | ColorChoice::Auto => termcolor::Buffer::ansi(),
                            ColorChoice::Never => termcolor::Buffer::no_color(),
                            _ => termcolor::Buffer::no_color(),
                        };
                        doc.render_colored(min_len, &mut buffer).unwrap();
                        res.push(std::str::from_utf8(buffer.as_slice()).unwrap().to_string());
                    }
                    else {
                        debug!("parse error {:?}", r);
                    }
                }
                res.push(String::from_iter(buf[start..].iter()));
                output_sender.send((i, res.join(""))).expect("send");
            }
        })
    }).for_each(drop);
    
    reader.lines().enumerate().map(move |(i, line)| {
        match line {
            Ok(s) => {
                input_sender.send((i, s)).expect("input send");
            }
            Err(err) => {
                error!("{:?}", err);
            }
        }
    }).for_each(drop);
    
    drop(output_sender);
    
    printer.join().expect("printer join");
}

fn init_logger(level: usize) {
    let format = |buf: &mut env_logger::fmt::Formatter, record: &log::Record| {
        writeln!(buf,
            "[{date}] [{level}] {module} | {file}:{line} | {message}",
            date = chrono::Local::now().format("%H:%M:%S%.3f").to_string(),
            level = record.level(),
            module = record.module_path().unwrap_or_default(),
            file = record.file().unwrap_or_default(),
            line = record.line().unwrap_or_default(),
            message = record.args()
        )
    };
    let mut builder = env_logger::Builder::new();
    builder.format(format).filter(None, log::LevelFilter::Info);

    if level == 0 && env::var("RUST_LOG").is_ok() {
        builder.parse_filters(&env::var("RUST_LOG").unwrap());
    }
    else if level == 1 {
        builder.parse_filters("debug");
    }
    else if level >= 2 {
        builder.parse_filters("trace");
    }

    builder.init()
}
