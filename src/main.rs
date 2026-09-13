//! The `maestro-voice` binary.
//!
//! A skeleton: the turn loop is not wired yet. It reports that plainly rather
//! than starting something that cannot work, because a daemon that appears to
//! listen and does not is worse than one that says it is not ready.

fn main() {
    println!("maestro-voice {}", env!("CARGO_PKG_VERSION"));
    println!("The turn loop is not wired yet: no microphone is opened and no model is called.");
}
