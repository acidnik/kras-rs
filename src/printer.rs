use std::{cmp::Reverse, collections::BinaryHeap, io::Write, thread::JoinHandle};

use crossbeam::channel::Receiver;

/*
This object consumes lines from several threads. Each message is a
(line_number, String)
Messages are consumed in random order and put to a priority queue
When the line number at top of the queue equals to next_line_num -
it is popped from the queue and printed
*/

pub struct Printer {
    thread: JoinHandle<()>,
}

impl Printer {
    pub fn new(receiver: Receiver<(usize, String)>) -> Self {
        // just for statistics: how many lines max were in the queue?
        let mut max_qlen = 0;
        // number of the next line to print
        let mut next_line_num = 0;
        // a storage for lines that are to be printed
        let mut output_queue = BinaryHeap::<Reverse<(usize, String)>>::new();
        let thread = std::thread::spawn(move || {
            let stdout = std::io::stdout();
            let mut stdout = stdout.lock();
            'recv: while let Ok((i, line)) = receiver.recv() {
                max_qlen = usize::max(max_qlen, output_queue.len());
                if i == next_line_num {
                    if let Err(err) = writeln!(stdout, "{line}") {
                        if err.kind() == std::io::ErrorKind::BrokenPipe {
                            debug!("write error {err}");
                        }
                        else {
                            error!("write error {err}");
                        }
                        break 'recv;
                    }
                    next_line_num += 1;
                }
                else {
                    output_queue.push(Reverse((i, line)));
                }

                if let Some(Reverse((i, line))) = output_queue.peek() {
                    if *i == next_line_num {
                        if let Err(err) = writeln!(stdout, "{line}") {
                            if err.kind() == std::io::ErrorKind::BrokenPipe {
                                debug!("write error {err}");
                            }
                            else {
                                error!("write error {err}");
                            }
                            break 'recv;
                        }
                        next_line_num += 1;
                        output_queue.pop();
                    }
                }
            }
            while let Some(Reverse((_, line))) = output_queue.pop() {
                if let Err(err) = writeln!(stdout, "{line}") {
                    if err.kind() == std::io::ErrorKind::BrokenPipe {
                        debug!("write error {err}");
                    }
                    else {
                        error!("write error {err}");
                    }
                    break;
                }
            }
            debug!("max queue len = {max_qlen}");
        });
        Printer { thread: thread }
    }

    pub fn join(self) {
        self.thread.join().expect("join failed")
    }
}
