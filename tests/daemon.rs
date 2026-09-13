//! Whole turns, driven from a WAV fixture with no router, no agent and no
//! speaker.
//!
//! `tests/turn.rs` proves the rules; this proves the wiring. Every boundary is
//! a fake that records what it was asked for, so the assertions are about what
//! the daemon actually did to the outside world: which audio reached the
//! transcriber, what text reached the agent, which cues reached the speaker.
//!
//! The fakes are deliberately able to block. Cancellation only means anything
//! while work is in flight, so the transcriber can be held open while the
//! fixture goes on producing chunks, which is how "the wake word cancels a
//! transcription" is tested rather than asserted.

mod support;

use maestro_voice::capture::WavSource;
use maestro_voice::runner::{Daemon, Note, Services, Stopped};
use maestro_voice::speak::Language;
use maestro_voice::tone::Cue;
use maestro_voice::turn::Delivery;
use std::sync::Arc;
use std::time::Duration;
use support::{Ears, Fakes, WAKE, Wakes, silence, speech};

/// Room tone, the wake phrase, a sentence, then quiet.
///
/// Assembled as samples rather than read from disk so each test states the
/// shape of its own turn; `a_real_recording_drives_a_whole_turn` covers the
/// committed fixture.
fn utterance() -> Vec<i16> {
    support::spoken_turn(20)
}

#[test]
fn a_whole_turn_reaches_the_transcriber_the_agent_and_the_speaker() {
    let fakes = Fakes::new();
    fakes.transcriber.answer("run the tests", Some("en"));
    fakes.reply("<speak>Two tests failed.</speak>");

    let outcome = fakes.run(&utterance(), Wakes::at(WAKE));

    assert_eq!(outcome, Stopped::AudioEnded);
    assert_eq!(
        fakes.courier.delivered(),
        vec!["run the tests".to_owned()],
        "the transcript must reach the agent exactly once"
    );
    assert_eq!(
        fakes.synthesizer.spoken(),
        vec![("Two tests failed.".to_owned(), "en".to_owned())],
        "the spoken block must be synthesized in the language it was asked in"
    );
    assert!(
        fakes.player.played_audio(),
        "and the samples must actually reach the speaker"
    );
    assert!(
        fakes.transcriber.warmed() >= 1,
        "the model must have been asked for at the wake, not at the transcription"
    );
}

#[test]
fn the_recording_carries_the_pre_roll_so_the_first_word_survives() {
    // The detector fires at the end of the phrase, so audio from before that
    // moment has to be in the recording or the first word is clipped.
    let fakes = Fakes::new();
    fakes.transcriber.answer("hello", Some("en"));

    // Room tone, then speech that begins BEFORE the wake word fires, which is
    // what happens when someone says "hey jarvis run the tests" in one breath.
    let mut audio = silence(support::LEAD);
    audio.extend(speech(8));
    let woke = support::LEAD + 8;
    audio.extend(speech(10));
    audio.extend(silence(20));
    fakes.run(&audio, Wakes::at(woke));

    let sent = fakes.transcriber.received();
    assert_eq!(sent.len(), 1, "one utterance, one transcription");
    let samples = support::samples_in_wav(&sent[0]);

    // After the wake there are 10 chunks of speech and the 10 quiet ones that
    // ended the turn. Anything beyond that came from before the wake fired.
    let after_the_wake = 20 * maestro_voice::capture::CHUNK_SAMPLES;
    assert!(
        samples > after_the_wake + 3000,
        "the recording must reach back before the wake: {samples} samples is no \
         more than the {after_the_wake} captured after it"
    );
}

#[test]
fn a_false_wake_sends_nothing_anywhere() {
    let fakes = Fakes::new();

    // The wake fires and nothing is said for longer than the lead-in.
    let outcome = fakes.run(&silence(60), Wakes::at(WAKE));

    assert_eq!(outcome, Stopped::AudioEnded);
    assert!(
        fakes.transcriber.received().is_empty(),
        "nothing was said, so nothing may be transcribed"
    );
    assert!(
        fakes.courier.delivered().is_empty(),
        "and nothing may reach the agent"
    );
    assert_eq!(
        fakes.player.cues(),
        vec![Cue::Listening],
        "only the acknowledgement, and no outcome cue: a false wake is silent"
    );
}

#[test]
fn an_empty_transcript_never_reaches_the_agent() {
    let fakes = Fakes::new();
    // What a transcriber returns for a cough.
    fakes.transcriber.answer(" . ", Some("en"));

    fakes.run(&utterance(), Wakes::at(WAKE));

    assert_eq!(
        fakes.transcriber.received().len(),
        1,
        "the audio was still transcribed"
    );
    assert!(
        fakes.courier.delivered().is_empty(),
        "but an empty prompt must never wake the agent"
    );
    assert!(
        fakes.player.cues().contains(&Cue::Dismissed),
        "and the dismissal must be audible: {:?}",
        fakes.player.cues()
    );
}

#[test]
fn a_router_refusal_is_announced_as_the_refusal_pair() {
    let fakes = Fakes::new();
    fakes.transcriber.refuse();

    fakes.run(&utterance(), Wakes::at(WAKE));

    assert!(
        fakes.player.cues().contains(&Cue::Refused),
        "a refusal must be audible, because speech is what is unavailable: {:?}",
        fakes.player.cues()
    );
    assert!(fakes.courier.delivered().is_empty());
}

#[test]
fn an_agent_sitting_on_a_question_is_announced_and_the_words_are_kept() {
    let fakes = Fakes::new();
    fakes.transcriber.answer("run the tests", Some("en"));
    fakes.courier.refuse(Delivery::Blocked);

    fakes.run(&utterance(), Wakes::at(WAKE));

    assert_eq!(
        fakes.courier.delivered(),
        vec!["run the tests".to_owned()],
        "the attempt must still be recorded, so nothing said is lost"
    );
    assert!(
        fakes.player.cues().contains(&Cue::Blocked),
        "being blocked must be audible: {:?}",
        fakes.player.cues()
    );
}

#[test]
fn a_reply_that_cannot_be_synthesized_is_one_quiet_tone() {
    let fakes = Fakes::new();
    fakes.transcriber.answer("run the tests", Some("en"));
    fakes.synthesizer.fail();
    fakes.reply("<speak>Two tests failed.</speak>");

    fakes.run(&utterance(), Wakes::at(WAKE));

    assert!(
        fakes.player.cues().contains(&Cue::Mute),
        "the answer is on screen, so this is quieter than a refusal: {:?}",
        fakes.player.cues()
    );
}

#[test]
fn a_turn_with_no_spoken_block_says_nothing_and_breaks_nothing() {
    let fakes = Fakes::new();
    fakes.transcriber.answer("run the tests", Some("en"));
    fakes.reply("I ran them. Two failed. No block here.");

    fakes.run(&utterance(), Wakes::at(WAKE));

    assert!(
        fakes.synthesizer.spoken().is_empty(),
        "a reply with no block is silent by design"
    );
    assert!(
        !fakes.player.cues().contains(&Cue::Mute),
        "and it is not a failure, so it is not announced: {:?}",
        fakes.player.cues()
    );
}

#[test]
fn the_wake_word_stops_a_reply_that_is_playing() {
    let fakes = Fakes::new();
    fakes.transcriber.answer("run the tests", Some("en"));
    fakes.reply("<speak>A long answer that is still being read out.</speak>");
    // Hold the speaker open so the second wake lands during playback.
    fakes.player.hold();

    let mut audio = utterance();
    audio.extend(silence(6));
    let woke_again = audio.len() / maestro_voice::capture::CHUNK_SAMPLES;
    audio.extend(speech(10));
    audio.extend(silence(20));

    fakes.run(&audio, Wakes::at_each(&[WAKE, woke_again]));

    assert!(
        fakes.player.hushed() >= 1,
        "the wake word must interrupt the speaker rather than talk over it"
    );
}

#[test]
fn the_wake_word_abandons_a_transcription_and_only_the_newest_is_delivered() {
    let fakes = Fakes::new();
    fakes
        .transcriber
        .answers(&["the first thing", "the second thing"], Some("en"));
    // Hold the first transcription open so the second wake lands during it.
    fakes.transcriber.hold();

    let mut audio = utterance();
    let woke_again = audio.len() / maestro_voice::capture::CHUNK_SAMPLES;
    audio.extend(speech(10));
    audio.extend(silence(20));

    fakes.run(&audio, Wakes::at_each(&[WAKE, woke_again]));

    // Counting deliveries is not enough: the machine's phase guard holds the
    // count at one even when the wrong transcript is the one that gets through.
    // What matters is WHICH utterance was delivered.
    assert_eq!(
        fakes.courier.delivered(),
        vec!["the second thing".to_owned()],
        "the abandoned utterance must not be what reaches the agent"
    );
}

#[test]
fn the_daemon_still_hears_a_wake_word_while_the_agent_is_working() {
    // The agent takes its time and never answers. The daemon must not wait.
    let fakes = Fakes::new();
    fakes.transcriber.answer("the first thing", Some("en"));
    fakes.courier.hold();

    let mut audio = utterance();
    let woke_again = audio.len() / maestro_voice::capture::CHUNK_SAMPLES;
    audio.extend(speech(10));
    audio.extend(silence(20));

    fakes.run(&audio, Wakes::at_each(&[WAKE, woke_again]));

    assert_eq!(
        fakes.transcriber.received().len(),
        2,
        "the second utterance must be heard and transcribed while the agent is \
         still holding the first"
    );
}

#[test]
fn capture_that_gives_up_announces_itself_and_the_daemon_stops() {
    let fakes = Fakes::new();

    let outcome = fakes.run_failing();

    assert_eq!(outcome, Stopped::CaptureGaveUp);
    assert!(
        fakes.player.cues().contains(&Cue::Stopped),
        "a microphone that stopped must say so: {:?}",
        fakes.player.cues()
    );
}

#[test]
fn a_real_recording_drives_a_whole_turn() {
    // The committed corpus, through the same path, so the wiring is exercised
    // against audio nobody assembled for the occasion.
    let path = support::fixture("wake_hey_jarvis_en.wav");
    let source = WavSource::open(&path).expect("the committed fixture must open");
    let audio = source.samples().to_vec();

    let fakes = Fakes::new();
    fakes
        .transcriber
        .answer("hey jarvis run the tests", Some("en"));
    fakes.reply("<speak>Running them now.</speak>");

    let mut padded = silence(support::LEAD);
    padded.extend(audio);
    padded.extend(silence(20));
    fakes.run(&padded, Wakes::at(WAKE));

    assert_eq!(
        fakes.courier.delivered(),
        vec!["hey jarvis run the tests".to_owned()]
    );
}

#[test]
fn the_fallback_language_answers_when_transcription_declines_to_say() {
    let fakes = Fakes::new();
    fakes.transcriber.answer("run the tests", None);
    fakes.reply("<speak>Done.</speak>");

    fakes.run(&utterance(), Wakes::at(WAKE));

    assert_eq!(
        fakes.synthesizer.spoken(),
        vec![("Done.".to_owned(), "en".to_owned())],
        "a detector that declines must not stop the answer being spoken"
    );
}

#[test]
fn a_french_utterance_is_answered_in_french() {
    let fakes = Fakes::new();
    fakes.transcriber.answer("roule les tests", Some("fr"));
    fakes.reply("<speak>Deux tests ont echoue.</speak>");

    fakes.run(&utterance(), Wakes::at(WAKE));

    assert_eq!(
        fakes.synthesizer.spoken(),
        vec![("Deux tests ont echoue.".to_owned(), "fr".to_owned())],
        "the language detected on the way in must choose the voice on the way out"
    );
}

/// The daemon must be constructible with the ordinary public surface, or the
/// fakes above are testing a shape nobody can build.
#[test]
fn a_daemon_can_be_built_from_the_public_surface_alone() {
    let fakes = Fakes::new();
    let services = Services {
        transcriber: Arc::clone(&fakes.transcriber) as Arc<_>,
        courier: Arc::clone(&fakes.courier) as Arc<_>,
        synthesizer: Arc::clone(&fakes.synthesizer) as Arc<_>,
        player: Arc::clone(&fakes.player) as Arc<_>,
    };
    let daemon = Daemon::new(
        support::machine(),
        services,
        Duration::from_millis(300),
        Language::new("en").expect("a language tag"),
    );

    let notifier: std::sync::mpsc::Sender<Note> = daemon.notifier();
    drop(notifier);
    let mut source = WavSource::from_bytes(&support::wav(&silence(2))).expect("a WAV");
    let mut wakes = Wakes::never();
    let mut ears = Ears::default();
    let mut daemon = daemon;

    assert_eq!(
        daemon.run(&mut source, &mut wakes, &mut ears),
        Stopped::AudioEnded
    );
}
