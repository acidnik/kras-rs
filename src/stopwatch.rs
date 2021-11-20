use std::time;

pub struct Stopwatch {
    start:    time::Instant,
    name:     String,
    treshold: usize,
    stopped:  bool,
}

impl Stopwatch {
    pub fn new<S: Into<String>>(name: S, treshold_ms: usize) -> Self {
        let name = name.into();
        Stopwatch {
            name,
            start: time::Instant::now(),
            treshold: treshold_ms * 1_000_000,
            stopped: false,
        }
    }
    pub fn stop(&mut self) {
        if self.stopped {
            return;
        }
        self.stopped = true;
        let elapsed = self.start.elapsed();
        if elapsed.as_secs() == 0 && elapsed.subsec_nanos() < (self.treshold as u32) {
            return;
        }
        debug!("<STOPWATCH> {}: {:?}", self.name, elapsed.as_nanos());
    }
}

impl Drop for Stopwatch {
    fn drop(&mut self) {
        self.stop();
    }
}
