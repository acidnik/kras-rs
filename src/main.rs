// disable some warnings for debug build
#![cfg_attr(
    debug_assertions,
    allow(dead_code, unused_imports, unused_mut, unused_variables, unreachable_code)
)]
#![allow(clippy::redundant_field_names)]

use std::{
    env,
    io::{BufRead, BufReader, Read, Write},
    thread, sync::{Arc, atomic::AtomicBool},
};
extern crate chrono;
#[macro_use]
extern crate log;
extern crate env_logger;

extern crate atty;

extern crate clap;

extern crate fileinput;
use clap::{Parser, ValueEnum};
use fileinput::FileInput;

extern crate crossbeam;

extern crate pom;

extern crate pretty;

extern crate num_cpus;

extern crate signal_hook;

use crossbeam::channel::bounded;
use pretty::termcolor::ColorChoice;

mod detect;
mod detect2;

mod pretty_value;

mod parse;
use parse::parse_str;

mod stopwatch;

mod printer;
use printer::Printer;
use signal_hook::consts::SIGPIPE;

#[derive(Clone, Debug, ValueEnum)]
enum ColorChoiceArg {
    Auto,
    Yes,
    No,
}

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Cli {
    #[arg(
        short = 'i',
        long,
        help="identation. 0 to disable (colorization is still performed)",
        default_value_t = 2
    )]
    indent: usize,

    #[arg(
        value_enum,
        short='c',
        long,
        help="colorize output",
        default_value_t = ColorChoiceArg::Auto,
    )]
    color: ColorChoiceArg,

    #[arg(
        short='C',
        long,
        help="alias for --color yes",
        default_value_t = false,
    )]
    force_color: bool,

    #[arg(
        short='s',
        long,
        help="sort keys",
        default_value_t = false,
    )]
    sort: bool,

    #[arg(
        short='r',
        long,
        help="try to parse nested strings",
        default_value_t = false,
    )]
    recursive: bool,

    #[arg(
        short='j',
        long,
        help="number of parallel jobs. Default is num_cpus",
        default_value_t = num_cpus::get(),
    )]
    jobs: usize,

    #[arg(
        short='w',
        long,
        help="maximum width of output",
        default_value_t = 80,
    )]
    width: usize,

    #[arg(
        short='m',
        long,
        help="look for data spannding several lines. This will read wholle input to memory",
        default_value_t = false,
    )]
    multiline: bool,

    #[arg(
        long,
        help="use more robust, but slower method to detect structured data",
        default_value_t = false,
    )]
    robust: bool,

    #[arg(
        long,
        help="debut mode",
        default_value_t = false,
    )]
    debug: bool,

    #[arg(
        index(1),
        help="Input files or stdin",
    )]
    input: Vec<String>,
}

fn main() {
    let signal_flag = Arc::new(AtomicBool::new(false));
    signal_hook::flag::register(SIGPIPE, Arc::clone(&signal_flag)).unwrap();

    let args = Cli::parse();

    init_logger(if args.debug {2} else {0});

    let min_len = if args.indent == 0 {
        std::usize::MAX
    }
    else {
        args.width
    };

    let color_choice = if args.force_color {
        ColorChoice::Always
    }
    else {
        match args.color {
            ColorChoiceArg::Yes => ColorChoice::Always,
            ColorChoiceArg::No => ColorChoice::Never,
            ColorChoiceArg::Auto => {
                if atty::is(atty::Stream::Stdout) {
                    ColorChoice::Auto
                }
                else {
                    ColorChoice::Never
                }
            }
        }
    };

    let jobs = if args.multiline {
        1
    }
    else {
        args.jobs
    };

    // ---- done parsing arguments. prepare to read from files

    let input = FileInput::new(&args.input);
    let mut reader = BufReader::new(input);

    // pipeline: input lines => input_sender => [worker] input_receiver => output_sender => [printer] output_receiver

    let (output_sender, output_receiver) = bounded(jobs * 128);
    let (input_sender, input_receiver) = bounded::<(usize, String)>(jobs * 128);

    let printer = Printer::new(output_receiver);

    (0..jobs).for_each(|_| {
        let input_receiver = input_receiver.clone();
        let output_sender = output_sender.clone();
        thread::spawn(move || {
            let signal_flag = Arc::new(AtomicBool::new(false));
            signal_hook::flag::register(SIGPIPE, Arc::clone(&signal_flag)).unwrap();
            while let Ok((i, s)) = input_receiver.recv() {
                let line = parse_str(&s, args.sort, args.recursive, args.robust);
                debug!("line = {:?}", line);
                let rendered_str = line.render(args.indent, min_len, color_choice);
                if let Err(err) = output_sender.send((i, rendered_str)) {
                    // likely a pipe is closed on us
                    debug!("send error: {}", err);
                    break;
                }
            }
        });
    });
    drop(input_receiver);

    if args.multiline {
        let mut buf = Vec::new();
        reader.read_to_end(&mut buf).unwrap();
        let input = std::str::from_utf8(&buf).unwrap().to_string();
        input_sender.send((0, input)).unwrap_or(());
        drop(input_sender);
    }
    else {
        for (i, line) in reader.lines().enumerate() {
            match line {
                Ok(s) => {
                    if let Err(err) = input_sender.send((i, s)) {
                        debug!("send error: {}", err);
                        break;
                    }
                },
                Err(err) => {
                    error!("{:?}", err);
                    // break;
                }
            }
        }
        drop(input_sender);
    }

    drop(output_sender);

    printer.join();
}

fn init_logger(level: usize) {
    let format = |buf: &mut env_logger::fmt::Formatter, record: &log::Record| {
        writeln!(
            buf,
            "[{date}] [{level}] {module} | {file}:{line} | {message}",
            date = chrono::Local::now().format("%H:%M:%S%.3f"),
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
