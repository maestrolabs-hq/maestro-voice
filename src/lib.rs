//! An always-on voice front end for the pi coding agent.
//!
//! A wake word starts a recording, the utterance is transcribed, the transcript
//! reaches a dedicated pi agent through Herdr, and a short spoken block from
//! that agent's reply is read back aloud.
//!
//! This crate owns the microphone, the wake word, voice activity detection, the
//! turn state machine and playback. It owns no models: speech recognition and
//! speech synthesis run as children of the maestro-llamacpp router, which
//! arbitrates their memory against every other local model. Two consequences
//! shape the code:
//!
//! **It supervises processes it did not write.** Capture is an `ffmpeg` child
//! and the models are the router's children. Any of them can hang, exit, or be
//! killed by the operating system.
//!
//! **Its input is a device, so its behaviour must be reachable without one.**
//! Wake-word and endpointing behaviour is decided by pure rules that take
//! classified frames, not by code that opens a microphone. See
//! `docs/adr/0001-the-capture-source-is-a-seam.md`.

pub mod endpoint;
pub mod speak;
