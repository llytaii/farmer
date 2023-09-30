use std::thread;
use std::cell::RefCell;
use std::time::Duration;
use std::sync::{atomic::AtomicBool, atomic::Ordering, Arc};

use enigo::*;
use stopwatch::Stopwatch;
use inputbot::KeybdKey::*;

#[derive(Debug)]
enum WorkStep {
    Mouse(enigo::MouseButton),
    Key(enigo::Key),
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
    let stop = Arc::new(AtomicBool::new(true));

    {
        let stop = stop.clone();
        thread::spawn(move || {
            let duration = Duration::from_secs(48);
            let mut farmer = Farmer::new();
            println!("adding work..");
            farmer.add_permanent_work(WorkStep::Mouse(MouseButton::Left));
            farmer.add_cyclic_work(WorkStep::Key(Key::Layout('d')), duration);
            for _ in 0..4 {
                farmer.add_cyclic_work(WorkStep::Key(Key::Layout('s')), duration);
                farmer.add_cyclic_work(WorkStep::Key(Key::Layout('d')), duration);
            }
            farmer.add_cyclic_work(WorkStep::Key(Key::Layout('g')), Duration::from_secs(0));

            farmer.work(stop);
        });
    }

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
