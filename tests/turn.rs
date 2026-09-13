//! The turn lifecycle, driven event by event with no device present.
//!
//! These are the rules that decide what the daemon does, and they are the ones
//! most likely to be wrong in a way nobody notices: a wake that leaves the
//! machine recording forever, a refusal that announces nothing, a reply that
//! plays over the person still talking. All of it is reachable here because the
//! machine takes classified events and returns actions, and touches neither the
//! microphone nor the network.

use maestro_voice::endpoint::Rule;
use maestro_voice::tone::Cue;
use maestro_voice::turn::{Action, Delivery, Event, Fault, Machine, Phase};
use std::time::Duration;

const FRAME: Duration = Duration::from_millis(80);

fn machine() -> Machine {
    Machine::new(Rule::new(
        Duration::from_millis(800),
        Duration::from_secs(3),
        Duration::from_secs(30),
    ))
}

/// One chunk of silence that did not wake anything.
const fn quiet() -> Event {
    Event::Chunk {
        woke: false,
        speech: false,
        frame: FRAME,
    }
}

/// One chunk of speech.
const fn speech() -> Event {
    Event::Chunk {
        woke: false,
        speech: true,
        frame: FRAME,
    }
}

/// The chunk the wake word fired on.
const fn wake() -> Event {
    Event::Chunk {
        woke: true,
        speech: false,
        frame: FRAME,
    }
}

/// Feed one event whose actions are setup rather than the point of the test.
fn step(machine: &mut Machine, event: Event) {
    drop(machine.observe(event));
}

/// Feed events and collect every action, in order.
fn drive(machine: &mut Machine, events: &[Event]) -> Vec<Action> {
    let mut seen = Vec::new();
    for event in events {
        seen.extend(machine.observe(*event));
    }
    seen
}

/// Wake, speak for `chunks` frames, then go quiet long enough to end the turn.
fn utterance(chunks: usize) -> Vec<Event> {
    let mut events = vec![wake()];
    events.extend(std::iter::repeat_n(speech(), chunks));
    events.extend(std::iter::repeat_n(quiet(), 11));
    events
}

/// Reach [`Phase::Working`] with a finished utterance behind it.
fn working() -> Machine {
    let mut machine = machine();
    drop(drive(&mut machine, &utterance(10)));
    assert_eq!(machine.phase(), Phase::Working, "setup must reach Working");
    machine
}

#[test]
fn the_wake_word_warms_the_transcriber_before_the_sentence_is_finished() {
    // The whole latency argument for this design rests on the load overlapping
    // the speaking, so the warm must be asked for at the wake and not at the
    // transcription.
    let mut machine = machine();
    let actions = machine.observe(wake());

    assert!(
        actions.contains(&Action::Warm),
        "the wake must ask for the model while there is still speech to come, \
         got {actions:?}"
    );
    assert!(
        actions.contains(&Action::Record),
        "and must start recording at the same moment, got {actions:?}"
    );
    assert_eq!(
        actions.first(),
        Some(&Action::Cue(Cue::Listening)),
        "the cue comes first so the speaker hears it immediately: {actions:?}"
    );
    assert_eq!(machine.phase(), Phase::Listening);
}

#[test]
fn a_finished_sentence_is_transcribed_then_delivered() {
    let mut machine = machine();
    let actions = drive(&mut machine, &utterance(10));

    assert!(
        actions.contains(&Action::Transcribe),
        "trailing silence must end the utterance: {actions:?}"
    );
    assert_eq!(machine.phase(), Phase::Working);

    let after = machine.observe(Event::Transcribed { empty: false });
    assert_eq!(
        after,
        vec![Action::Deliver],
        "a transcript goes to the agent"
    );

    let settled = machine.observe(Event::Delivered(Delivery::Sent));
    assert_eq!(settled, vec![], "a delivered turn announces nothing");
}

#[test]
fn the_daemon_hears_a_wake_word_while_the_agent_is_working() {
    // The point of returning to idle after delivery: a turn that took the
    // daemon deaf for the length of the agent's work could not be corrected.
    let mut machine = working();
    step(&mut machine, Event::Transcribed { empty: false });
    step(&mut machine, Event::Delivered(Delivery::Sent));

    assert_eq!(
        machine.phase(),
        Phase::Idle,
        "the agent may work for minutes; the daemon must not wait on it"
    );

    let actions = machine.observe(wake());
    assert!(
        actions.contains(&Action::Record),
        "a second utterance must be heard while the first is still being worked \
         on: {actions:?}"
    );
}

#[test]
fn a_false_wake_plays_nothing_and_delivers_nothing() {
    let mut machine = machine();
    // The wake fires, then nothing is said for longer than the lead-in.
    let mut events = vec![wake()];
    events.extend(std::iter::repeat_n(quiet(), 40));
    let actions = drive(&mut machine, &events);

    assert_eq!(
        &actions[3..],
        [] as [Action; 0],
        "a wake nothing followed must cost nothing and say nothing beyond the \
         acknowledgement it already gave: {actions:?}"
    );
    assert!(
        !actions.contains(&Action::Transcribe),
        "nothing was said, so there is nothing to transcribe: {actions:?}"
    );
    assert_eq!(machine.phase(), Phase::Idle);
}

#[test]
fn an_empty_transcript_is_dismissed_rather_than_delivered() {
    let mut machine = working();

    let actions = machine.observe(Event::Transcribed { empty: true });

    assert_eq!(
        actions,
        vec![Action::Cue(Cue::Dismissed)],
        "an empty prompt must never reach the agent"
    );
    assert_eq!(machine.phase(), Phase::Idle);
}

#[test]
fn a_refused_transcription_announces_the_refusal_pair() {
    for fault in [Fault::Refused, Fault::Unreachable] {
        let mut machine = working();

        let actions = machine.observe(Event::NotTranscribed(fault));

        assert_eq!(
            actions,
            vec![Action::Cue(Cue::Refused)],
            "{fault:?} must be announced, since speech is not available to say it"
        );
        assert_eq!(machine.phase(), Phase::Idle);
    }
}

#[test]
fn an_agent_that_cannot_be_prompted_announces_three_tones() {
    for refusal in [Delivery::Blocked, Delivery::Stalled] {
        let mut machine = working();
        step(&mut machine, Event::Transcribed { empty: false });

        let actions = machine.observe(Event::Delivered(refusal));

        assert_eq!(
            actions,
            vec![Action::Cue(Cue::Blocked)],
            "{refusal:?} must be audible: the words went nowhere"
        );
        assert_eq!(machine.phase(), Phase::Idle);
    }
}

#[test]
fn a_reply_that_cannot_be_spoken_is_one_quiet_tone() {
    let mut machine = machine();
    step(&mut machine, Event::ReplyReady);
    assert_eq!(machine.phase(), Phase::Speaking);

    let actions = machine.observe(Event::NotSpoken);

    assert_eq!(
        actions,
        vec![Action::Cue(Cue::Mute)],
        "the answer is on screen, so this is smaller than a refusal"
    );
    assert_eq!(machine.phase(), Phase::Idle);
}

#[test]
fn a_spoken_reply_returns_the_daemon_to_listening() {
    let mut machine = machine();
    step(&mut machine, Event::ReplyReady);

    let actions = machine.observe(Event::Spoke);

    assert_eq!(actions, vec![], "a reply read out needs no announcement");
    assert_eq!(machine.phase(), Phase::Idle);
}

#[test]
fn the_wake_word_stops_a_reply_that_is_playing() {
    let mut machine = machine();
    step(&mut machine, Event::ReplyReady);
    assert_eq!(machine.phase(), Phase::Speaking);

    let actions = machine.observe(wake());

    assert_eq!(
        actions.first(),
        Some(&Action::Hush),
        "playback must stop before anything else, or it talks over you: {actions:?}"
    );
    assert!(actions.contains(&Action::Record));
    assert_eq!(machine.phase(), Phase::Listening);
}

#[test]
fn the_wake_word_abandons_a_transcription_in_flight() {
    let mut machine = working();

    let actions = machine.observe(wake());

    assert_eq!(
        actions.first(),
        Some(&Action::Abandon),
        "newest wins: the old utterance must not arrive after the new one: \
         {actions:?}"
    );
    assert_eq!(machine.phase(), Phase::Listening);
}

#[test]
fn a_transcript_that_arrives_after_being_abandoned_is_ignored() {
    // The runner cancels by generation, but the machine must not act on a
    // result it already gave up on either, or one utterance delivers twice.
    let mut machine = working();
    step(&mut machine, wake());

    let actions = machine.observe(Event::Transcribed { empty: false });

    assert_eq!(
        actions,
        vec![],
        "the abandoned utterance must not be delivered: {actions:?}"
    );
    assert_eq!(
        machine.phase(),
        Phase::Listening,
        "still recording the new one"
    );
}

#[test]
fn a_reply_arriving_while_someone_is_talking_waits_rather_than_talking_over_them() {
    // Playing here would put the daemon's own voice into the microphone it is
    // recording from, which corrupts the utterance and can retrigger the wake.
    let mut machine = machine();
    step(&mut machine, wake());

    let actions = machine.observe(Event::ReplyReady);

    assert_eq!(
        actions,
        vec![],
        "a reply must not play into an open microphone: {actions:?}"
    );
    assert_eq!(machine.phase(), Phase::Listening);
}

#[test]
fn a_reply_held_back_is_played_once_the_daemon_is_idle_again() {
    // Held, not dropped: the answer is the whole point of the turn.
    let mut machine = machine();
    step(&mut machine, wake());
    step(&mut machine, Event::ReplyReady);

    let events: Vec<Event> = std::iter::repeat_n(quiet(), 40).collect();
    let actions = drive(&mut machine, &events);

    assert!(
        actions.contains(&Action::Play),
        "the held reply must be spoken when the microphone closes: {actions:?}"
    );
    assert_eq!(machine.phase(), Phase::Speaking);
}

#[test]
fn a_monologue_is_transcribed_at_the_cap_rather_than_discarded() {
    let mut machine = machine();
    let mut events = vec![wake()];
    // Well past the thirty-second cap with no pause at all.
    events.extend(std::iter::repeat_n(speech(), 400));
    let actions = drive(&mut machine, &events);

    assert!(
        actions.contains(&Action::Transcribe),
        "a microphone that never goes quiet must still produce a turn: {actions:?}"
    );
}

#[test]
fn capture_giving_up_announces_itself_and_halts() {
    let mut machine = machine();
    let actions = machine.observe(Event::CaptureStopped);

    assert_eq!(
        actions,
        vec![Action::Cue(Cue::Stopped), Action::Halt],
        "a microphone that stopped must say so before the daemon exits"
    );
}

#[test]
fn a_second_wake_while_recording_restarts_the_utterance() {
    let mut machine = machine();
    step(&mut machine, wake());
    drop(drive(&mut machine, &[speech(), speech()]));

    let actions = machine.observe(wake());

    assert!(
        actions.contains(&Action::Record),
        "saying the phrase again starts over rather than appending: {actions:?}"
    );
    assert!(
        !actions.contains(&Action::Abandon),
        "nothing was in flight, so there is nothing to abandon: {actions:?}"
    );
    assert_eq!(machine.phase(), Phase::Listening);
}

#[test]
fn audio_arriving_while_idle_changes_nothing() {
    // The daemon hears the room all day. Only the wake word may start a turn.
    let mut machine = machine();

    let actions = drive(&mut machine, &[speech(), quiet(), speech()]);

    assert_eq!(
        actions,
        vec![],
        "speech without the wake phrase must be ignored: {actions:?}"
    );
    assert_eq!(machine.phase(), Phase::Idle);
}
