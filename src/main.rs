use std::sync::{atomic::AtomicBool, atomic::Ordering, Arc};
use std::{thread, time::Duration};

use enigo::*;
use inputbot::KeybdKey::*;

fn main() {
    let stop = Arc::new(AtomicBool::new(false));

    {
        let stop = stop.clone();
        thread::spawn(move || {
            let _48_secs = Duration::from_secs(48);
            let mut enigo = Enigo::new();

            println!("sleeping for 5s, activate target window now...");
            thread::sleep(Duration::from_secs(5));


            println!("start pressing buttons");
            enigo.mouse_down(MouseButton::Left);

            loop {
                enigo.key_down(Key::Layout('d'));
                thread::sleep(_48_secs);
                enigo.key_up(Key::Layout('d'));

                for _ in 0..4 {
                    enigo.key_down(Key::Layout('s'));
                    thread::sleep(_48_secs);
                    enigo.key_up(Key::Layout('s'));

                    while stop.load(Ordering::Relaxed) {
                        thread::sleep(Duration::from_millis(100));
                    }

                    enigo.key_down(Key::Layout('d'));
                    thread::sleep(_48_secs);
                    enigo.key_up(Key::Layout('d'));

                    while stop.load(Ordering::Relaxed) {
                        thread::sleep(Duration::from_millis(100));
                    }
                }
                enigo.key_click(Key::Layout('g'));
            }
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
