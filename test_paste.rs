use enigo::{Direction, Enigo, Key, Keyboard, Settings};
use std::process::Command;
use std::thread;
use std::time::Duration;

fn main() {
    println!("Testing paste with enigo...");
    let _ = Command::new("xclip")
        .args(["-selection", "clipboard"])
        .spawn();
    
    thread::sleep(Duration::from_millis(100));
    let mut enigo = Enigo::new(&Settings::default()).unwrap();
    enigo.key(Key::Control, Direction::Press).unwrap();
    enigo.key(Key::Unicode('v'), Direction::Click).unwrap();
    enigo.key(Key::Control, Direction::Release).unwrap();
    println!("Paste keystroke sent successfully via Enigo!");
}
