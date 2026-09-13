//! Running the daemon: the intake listener, then the turn loop.
//!
//! Two threads and no more. The intake sits on a loopback socket waiting for
//! the agent's extension to post a finished turn, and forwards what it receives
//! into the daemon's own channel; everything else happens on the turn loop,
//! which is the thread that reads the microphone and must never be made to
//! wait. See `crate::runner`.

use std::process::ExitCode;
use std::sync::mpsc::Sender;
use std::thread;

use maestro_voice::config::Config;
use maestro_voice::intake::{Intake, Received};
use maestro_voice::runner::Note;

use crate::{assemble, ears, farewell, microphone};

/// Listen for the wake word and serve turns until the microphone gives up.
pub fn run(config: &Config) -> ExitCode {
    let (mut daemon, mut waker) = match assemble(config) {
        Ok(parts) => parts,
        Err(why) => {
            eprintln!("maestro-voice: {why}");
            return ExitCode::FAILURE;
        }
    };

    let intake = match Intake::bind(config.intake_port) {
        Ok(intake) => intake,
        Err(why) => {
            eprintln!(
                "maestro-voice: could not listen on port {} for finished turns: {why}",
                config.intake_port
            );
            eprintln!("maestro-voice: set 'intake_port' to a free one, or stop what holds it");
            return ExitCode::FAILURE;
        }
    };

    match intake.address() {
        Ok(address) => println!("maestro-voice: finished turns are posted to {address}"),
        Err(why) => eprintln!("maestro-voice: the intake could not name itself: {why}"),
    }
    listen_for_replies(intake, daemon.notifier());

    println!(
        "maestro-voice: listening. Say the wake phrase; transcripts go to '{}'.",
        config.agent
    );

    let mut source = microphone(config);
    let mut gate = ears();
    farewell(daemon.run(&mut source, waker.as_mut(), &mut gate))
}

/// Take finished turns off the socket and pass them to the daemon.
///
/// On its own thread because accepting blocks, and the turn loop cannot. A
/// caller that is refused or unreadable is an ordinary outcome and must not end
/// the loop: the daemon serves one agent all day, and a bad request must not
/// stop the next good one from arriving.
fn listen_for_replies(intake: Intake, post: Sender<Note>) {
    drop(thread::spawn(move || {
        loop {
            match intake.accept() {
                Ok(Received::Turn(turn)) => {
                    // A turn with no assistant text is still posted, and still
                    // worth passing on: a silent turn is a thing to count, not
                    // a thing to miss.
                    let message = turn.message().unwrap_or_default().to_owned();
                    if post.send(Note::Reply(message)).is_err() {
                        // The daemon has stopped; so should this.
                        return;
                    }
                }
                Ok(Received::Refused(why)) => {
                    eprintln!("maestro-voice: a caller was refused: {why:?}");
                }
                Err(why) => {
                    eprintln!("maestro-voice: the intake stopped accepting: {why}");
                    return;
                }
            }
        }
    }));
}
