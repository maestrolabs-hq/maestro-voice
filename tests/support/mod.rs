//! Stand-ins for the four services, and the audio to drive them with.
//!
//! Each fake records what it was asked to do, so a test asserts about the
//! daemon's effect on the outside world rather than about its internals. Each
//! can also be told to take a moment, which is what makes cancellation
//! testable: work has to still be in flight for a wake word to cancel it.
//!
//! The speech gate is the real one rather than a fake. It is cheap, it is pure,
//! and a fake would only prove that the daemon believes whatever it is told.

use maestro_voice::capture::{CHUNK_SAMPLES, Error as CaptureError, Source, WavSource};
use maestro_voice::endpoint::Rule;
use maestro_voice::runner::{Daemon, Note, Services, Stopped};
use maestro_voice::service::{Courier, Player, Synthesizer, Transcriber, Transcript, Waker};
use maestro_voice::speak::Language;
use maestro_voice::tone::Cue;
use maestro_voice::turn::{Delivery, Fault, Machine};
use std::path::PathBuf;
use std::sync::mpsc::Sender;
use std::sync::{Arc, Mutex};
use std::time::Duration;

/// Long enough for the audio loop to get through a file's remaining chunks,
/// short enough that a suite of these still runs in a moment.
const MOMENT: Duration = Duration::from_millis(300);

/// The endpointing rule every test here uses.
pub fn machine() -> Machine {
    Machine::new(Rule::new(
        Duration::from_millis(800),
        Duration::from_secs(3),
        Duration::from_secs(30),
    ))
}

/// `chunks` worth of something loud enough to be speech.
pub fn speech(chunks: usize) -> Vec<i16> {
    (0..chunks * CHUNK_SAMPLES)
        .map(|n| if n % 2 == 0 { 6000 } else { -6000 })
        .collect()
}

/// `chunks` worth of digital silence.
pub fn silence(chunks: usize) -> Vec<i16> {
    vec![0; chunks * CHUNK_SAMPLES]
}

/// Chunks of room tone that every recording starts with.
///
/// Not padding. The speech gate learns its floor from the quietest thing it
/// heard recently, so audio that opens at full speaking level teaches it that
/// speech *is* the floor and only the first chunk registers. A real microphone
/// always delivers some room first; a fixture that does not is testing a
/// situation that cannot happen. Long enough to fill the gate's window.
pub const LEAD: usize = 50;

/// Which chunk the wake phrase lands on: the first one after the room tone.
pub const WAKE: usize = LEAD;

/// Room tone, then `words` chunks of speech, then enough quiet to end the turn.
pub fn spoken_turn(words: usize) -> Vec<i16> {
    let mut audio = silence(LEAD);
    audio.extend(speech(words));
    audio.extend(silence(20));
    audio
}

/// Samples as a WAV file, using the daemon's own encoder.
pub fn wav(samples: &[i16]) -> Vec<u8> {
    maestro_voice::runner::wav::encode(samples)
}

/// How many samples a WAV file holds.
pub fn samples_in_wav(bytes: &[u8]) -> usize {
    WavSource::from_bytes(bytes)
        .expect("the daemon must produce a readable WAV")
        .samples()
        .len()
}

/// A file from the committed wake corpus.
pub fn fixture(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/wake/corpus")
        .join(name)
}

/// The real speech gate.
pub type Ears = maestro_voice::voice::Gate;

/// A wake detector that fires on the chunks it was told to.
pub struct Wakes {
    at: Vec<usize>,
    seen: usize,
}

impl Wakes {
    /// Fires once, on chunk `n`.
    pub fn at(n: usize) -> Self {
        Self::at_each(&[n])
    }

    /// Fires on each of `chunks`.
    pub fn at_each(chunks: &[usize]) -> Self {
        Self {
            at: chunks.to_vec(),
            seen: 0,
        }
    }

    /// Never fires.
    pub fn never() -> Self {
        Self::at_each(&[])
    }
}

impl Waker for Wakes {
    fn woke(&mut self, _chunk: &[i16]) -> bool {
        let fires = self.at.contains(&self.seen);
        self.seen += 1;
        fires
    }
}

/// A source that fails at once, standing in for capture that has given up.
struct Broken;

impl Source for Broken {
    fn next_chunk(&mut self) -> Result<Option<Vec<i16>>, CaptureError> {
        Err(CaptureError::Stopped(
            "capture failed 3 times in a row and has stopped".to_owned(),
        ))
    }
}

/// What the transcriber will say, and how slowly.
#[derive(Default)]
struct Answer {
    /// One entry per call, so successive utterances can be told apart. The last
    /// is repeated once exhausted.
    texts: Vec<String>,
    language: Option<String>,
    refuse: bool,
    slow: bool,
}

/// A transcriber that answers whatever it was told to.
#[derive(Default)]
pub struct FakeTranscriber {
    answer: Mutex<Answer>,
    received: Mutex<Vec<Vec<u8>>>,
    warmed: Mutex<usize>,
}

impl FakeTranscriber {
    /// Answer with `text`, reporting `language` when there is one.
    pub fn answer(&self, text: &str, language: Option<&str>) {
        self.answers(&[text], language);
    }

    /// Answer with each of `texts` in turn, so the utterance a delivery came
    /// from can be identified rather than merely counted.
    pub fn answers(&self, texts: &[&str], language: Option<&str>) {
        let mut answer = self.answer.lock().expect("lock");
        answer.texts = texts.iter().map(|t| (*t).to_owned()).collect();
        answer.language = language.map(ToOwned::to_owned);
    }

    /// Refuse every transcription, as a router with no room would.
    pub fn refuse(&self) {
        self.answer.lock().expect("lock").refuse = true;
    }

    /// Take a moment, so a wake word can land while this is in flight.
    pub fn hold(&self) {
        self.answer.lock().expect("lock").slow = true;
    }

    /// The audio handed over, one entry per transcription.
    pub fn received(&self) -> Vec<Vec<u8>> {
        self.received.lock().expect("lock").clone()
    }

    /// How many times the model was asked for ahead of time.
    pub fn warmed(&self) -> usize {
        *self.warmed.lock().expect("lock")
    }
}

impl Transcriber for FakeTranscriber {
    fn warm(&self) {
        *self.warmed.lock().expect("lock") += 1;
    }

    fn transcribe(&self, audio: &[u8]) -> Result<Transcript, Fault> {
        let call = {
            let mut received = self.received.lock().expect("lock");
            received.push(audio.to_vec());
            received.len() - 1
        };
        let (text, language, refuse, slow) = {
            let answer = self.answer.lock().expect("lock");
            let text = answer
                .texts
                .get(call)
                .or_else(|| answer.texts.last())
                .cloned()
                .unwrap_or_default();
            (text, answer.language.clone(), answer.refuse, answer.slow)
        };
        if slow {
            std::thread::sleep(MOMENT);
        }
        if refuse {
            return Err(Fault::Refused);
        }
        Ok(Transcript {
            text,
            language: language.as_deref().and_then(Language::new),
        })
    }
}

/// A courier that records what it was handed, and posts the agent's reply.
#[derive(Default)]
pub struct FakeCourier {
    delivered: Mutex<Vec<String>>,
    outcome: Mutex<Option<Delivery>>,
    slow: Mutex<bool>,
    reply: Mutex<Option<String>>,
    post: Mutex<Option<Sender<Note>>>,
}

impl FakeCourier {
    /// Report `outcome` instead of taking the transcript.
    pub fn refuse(&self, outcome: Delivery) {
        *self.outcome.lock().expect("lock") = Some(outcome);
    }

    /// Take a moment, standing in for an agent that is working.
    pub fn hold(&self) {
        *self.slow.lock().expect("lock") = true;
    }

    /// Every transcript handed over, in order.
    pub fn delivered(&self) -> Vec<String> {
        self.delivered.lock().expect("lock").clone()
    }

    fn attach(&self, post: Sender<Note>, reply: Option<String>) {
        *self.post.lock().expect("lock") = Some(post);
        *self.reply.lock().expect("lock") = reply;
    }
}

impl Courier for FakeCourier {
    fn deliver(&self, text: &str) -> Delivery {
        self.delivered.lock().expect("lock").push(text.to_owned());
        if *self.slow.lock().expect("lock") {
            std::thread::sleep(MOMENT);
        }
        if let Some(refused) = *self.outcome.lock().expect("lock") {
            return refused;
        }
        // The agent answers after it is prompted, which is what makes the reply
        // arrive mid-turn rather than before the turn starts.
        let reply = self.reply.lock().expect("lock").clone();
        if let (Some(message), Some(post)) = (reply, self.post.lock().expect("lock").as_ref()) {
            drop(post.send(Note::Reply(message)));
        }
        Delivery::Sent
    }
}

/// A synthesizer that records what it was asked to say.
#[derive(Default)]
pub struct FakeSynthesizer {
    spoken: Mutex<Vec<(String, String)>>,
    fail: Mutex<bool>,
}

impl FakeSynthesizer {
    /// Fail to synthesize anything.
    pub fn fail(&self) {
        *self.fail.lock().expect("lock") = true;
    }

    /// Every request, as the text and the language tag it was asked in.
    pub fn spoken(&self) -> Vec<(String, String)> {
        self.spoken.lock().expect("lock").clone()
    }
}

impl Synthesizer for FakeSynthesizer {
    fn synthesize(&self, text: &str, language: &Language) -> Option<Vec<i16>> {
        self.spoken
            .lock()
            .expect("lock")
            .push((text.to_owned(), language.as_str().to_owned()));
        if *self.fail.lock().expect("lock") {
            return None;
        }
        Some(speech(4))
    }
}

/// A speaker that records what reached it, telling cues from replies.
#[derive(Default)]
pub struct FakePlayer {
    cues: Mutex<Vec<Cue>>,
    audio: Mutex<usize>,
    hushed: Mutex<usize>,
    slow: Mutex<bool>,
}

impl FakePlayer {
    /// Take a moment, so a wake word can land during playback.
    pub fn hold(&self) {
        *self.slow.lock().expect("lock") = true;
    }

    /// Which cues reached the speaker, in order.
    pub fn cues(&self) -> Vec<Cue> {
        self.cues.lock().expect("lock").clone()
    }

    /// Whether any synthesized reply reached the speaker.
    pub fn played_audio(&self) -> bool {
        *self.audio.lock().expect("lock") > 0
    }

    /// How many times playback was interrupted.
    pub fn hushed(&self) -> usize {
        *self.hushed.lock().expect("lock")
    }

    /// Which cue these samples are, if they are a cue at all.
    ///
    /// Compared against the real cue audio rather than a label, so a cue that
    /// stopped being produced correctly stops being recognised here too.
    fn recognise(samples: &[i16]) -> Option<Cue> {
        [
            Cue::Listening,
            Cue::Dismissed,
            Cue::Mute,
            Cue::Refused,
            Cue::Blocked,
            Cue::Stopped,
        ]
        .into_iter()
        .find(|cue| cue.samples() == samples)
    }
}

impl Player for FakePlayer {
    fn play(&self, samples: &[i16]) -> bool {
        match Self::recognise(samples) {
            Some(cue) => self.cues.lock().expect("lock").push(cue),
            None => *self.audio.lock().expect("lock") += 1,
        }
        if *self.slow.lock().expect("lock") {
            std::thread::sleep(MOMENT);
        }
        true
    }

    fn hush(&self) {
        *self.hushed.lock().expect("lock") += 1;
    }
}

/// The four fakes, wired together.
pub struct Fakes {
    pub transcriber: Arc<FakeTranscriber>,
    pub courier: Arc<FakeCourier>,
    pub synthesizer: Arc<FakeSynthesizer>,
    pub player: Arc<FakePlayer>,
    reply: Mutex<Option<String>>,
}

impl Fakes {
    pub fn new() -> Self {
        Self {
            transcriber: Arc::new(FakeTranscriber::default()),
            courier: Arc::new(FakeCourier::default()),
            synthesizer: Arc::new(FakeSynthesizer::default()),
            player: Arc::new(FakePlayer::default()),
            reply: Mutex::new(None),
        }
    }

    /// What the agent will post once it has been prompted.
    pub fn reply(&self, message: &str) {
        *self.reply.lock().expect("lock") = Some(message.to_owned());
    }

    fn daemon(&self) -> Daemon {
        let services = Services {
            transcriber: Arc::clone(&self.transcriber) as Arc<_>,
            courier: Arc::clone(&self.courier) as Arc<_>,
            synthesizer: Arc::clone(&self.synthesizer) as Arc<_>,
            player: Arc::clone(&self.player) as Arc<_>,
        };
        let daemon = Daemon::new(
            machine(),
            services,
            Duration::from_millis(300),
            Language::new("en").expect("a language tag"),
        );
        self.courier
            .attach(daemon.notifier(), self.reply.lock().expect("lock").clone());
        daemon
    }

    /// Drive a whole turn from `samples`, waking where `wakes` says.
    pub fn run(&self, samples: &[i16], mut wakes: Wakes) -> Stopped {
        let mut daemon = self.daemon();
        let mut source = WavSource::from_bytes(&wav(samples)).expect("a readable WAV");
        let mut ears = Ears::default();
        daemon.run(&mut source, &mut wakes, &mut ears)
    }

    /// Drive a daemon whose capture is already gone.
    pub fn run_failing(&self) -> Stopped {
        let mut daemon = self.daemon();
        let mut ears = Ears::default();
        daemon.run(&mut Broken, &mut Wakes::never(), &mut ears)
    }
}
