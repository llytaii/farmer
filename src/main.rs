use std::cell::RefCell;
use std::env;
use std::fs::File;
use std::io::Read;
use std::sync::{atomic::AtomicBool, atomic::Ordering, Arc};
use std::thread;
use std::time::Duration;

use enigo::*;
use inputbot::KeybdKey::*;
use serde::{Deserialize, Serialize};
use stopwatch::Stopwatch;

#[derive(Deserialize, Serialize, Debug, Clone, Copy)]
enum WorkStep {
    Mouse(enigo::MouseButton),
    Key(enigo::Key),
}

#[derive(Deserialize, Serialize, Debug)]
struct Work {
    permanent_work: Vec<WorkStep>,
    cyclic_work: Vec<(WorkStep, u64)>,
}

struct Farmer {
    permanent_work: Vec<WorkStep>,
    cyclic_work: Vec<(WorkStep, Duration)>,
    enigo: RefCell<Enigo>,
    last_stop_signal: bool,
}

impl Farmer {
    fn start_work_step(&self, input: &WorkStep) {
        println!("start work step {:?}", input);
        let mut enigo = self.enigo.borrow_mut();
        match input {
            WorkStep::Mouse(button) => enigo.mouse_down(button.clone()),
            WorkStep::Key(key) => enigo.key_down(key.clone()),
        }
    }

    fn stop_work_step(&self, input: &WorkStep) {
        println!("stop work step {:?}", input);
        let mut enigo = self.enigo.borrow_mut();
        match input {
            WorkStep::Mouse(button) => enigo.mouse_up(button.clone()),
            WorkStep::Key(key) => enigo.key_up(key.clone()),
        }
    }

    fn start_permanent_work(&self) {
        for work_step in &self.permanent_work {
            self.start_work_step(&work_step);
        }
    }

    fn stop_permanent_work(&self) {
        for work_step in &self.permanent_work {
            self.stop_work_step(&work_step);
        }
    }

    /// starts working, pass stop signal as true to start with pausing.
    fn work(&mut self, stop_signal: Arc<AtomicBool>) {
        if stop_signal.load(Ordering::Relaxed) {
            println!("to start press the continue / stop button");
            while stop_signal.load(Ordering::Relaxed) {
                thread::sleep(Duration::from_millis(100));
            }
        }

        self.start_permanent_work();

        loop {
            for (work_step, duration) in &self.cyclic_work {
                let mut stopwatch = Stopwatch::start_new();
                self.start_work_step(&work_step);

                while let Some(_) = duration.checked_sub(stopwatch.elapsed()) {
                    let stop_signal = stop_signal.load(Ordering::Relaxed);

                    if stop_signal != self.last_stop_signal {
                        if stop_signal {
                            self.stop_permanent_work();
                            self.stop_work_step(&work_step);
                            stopwatch.stop();
                        } else {
                            self.start_permanent_work();
                            self.start_work_step(&work_step);
                            stopwatch.start();
                        }
                    }

                    self.last_stop_signal = stop_signal;
                    thread::sleep(Duration::from_millis(100));
                }
                self.stop_work_step(&work_step);
            }
        }
    }

    fn add_permanent_work(&mut self, work_step: WorkStep) {
        self.permanent_work.push(work_step);
    }

    fn add_cyclic_work(&mut self, work_step: WorkStep, duration: Duration) {
        self.cyclic_work.push((work_step, duration));
    }

    fn set_work(&mut self, work: Work) {
        self.permanent_work = work.permanent_work;
        for (work_step, duration) in work.cyclic_work {
            self.cyclic_work
                .push((work_step, Duration::from_secs(duration)));
        }
    }

    fn get_work(&self) -> Work {
        let mut work = Work {
            permanent_work: self.permanent_work.clone(),
            cyclic_work: Vec::new(),
        };

        for (work_step, duration) in &self.cyclic_work {
            work.cyclic_work
                .push((work_step.clone(), duration.as_secs()));
        }

        work
    }

    fn new() -> Farmer {
        Farmer {
            permanent_work: Vec::new(),
            cyclic_work: Vec::new(),
            enigo: RefCell::new(Enigo::new()),
            last_stop_signal: false,
        }
    }
}

fn main() {
    let args: Vec<String> = env::args().collect();

    if args.len() != 2 {
        eprintln!(
            "Usage: {} <filename.json>",
            std::path::Path::new(&args[0])
                .file_name()
                .unwrap()
                .to_str()
                .unwrap()
        );
        std::process::exit(1);
    }

    let filename = &args[1];

    let mut file = match File::open(filename) {
        Ok(file) => file,
        Err(err) => {
            eprintln!("error opening file: {}", err);
            std::process::exit(1);
        }
    };

    let mut content = String::new();
    if let Err(err) = file.read_to_string(&mut content) {
        eprintln!("error reading file: {}", err);
        std::process::exit(1);
    }

    let result: Result<Work, serde_json::Error> = serde_json::from_str(&content);

    let work = match result {
        Ok(parsed_data) => {
            let pretty_json = serde_json::to_string_pretty(&parsed_data);
            match pretty_json {
                Ok(json_str) => println!("Parsed data:\n{}", json_str),
                Err(err) => eprintln!("Error serializing data to pretty JSON: {}", err),
            }
            parsed_data
        }
        Err(err) => {
            eprintln!("error deserializing json: {}", err);
            std::process::exit(1);
        }
    };

    let stop = Arc::new(AtomicBool::new(true));

    println!("");
    println!("starting execution..");

    // start worker thread
    {
        let stop = stop.clone();
        thread::spawn(move || {
            let mut farmer = Farmer::new();
            farmer.set_work(work);
            farmer.work(stop);
        });
    }

    // start callback thread
    {
        let stop = stop.clone();
        Numpad0Key.bind(move || {
            if Numpad0Key.is_pressed() {
                if stop.load(Ordering::Relaxed) {
                    println!("continuing execution");
                    stop.store(false, Ordering::Relaxed);
                } else {
                    println!("stopping execution");
                    stop.store(true, Ordering::Relaxed);
                }
            }
        });
    }

    inputbot::handle_input_events();
}
