//! Speech recognition and synthesis, as the router serves them.
//!
//! Both models are children of the maestro-llamacpp router, which admits them
//! against the same memory budget as every other local model. This crate never
//! loads one: it asks, and the router decides.
//!
//! **The dedicated endpoint, not the generic one.** A transcription is
//! `multipart/form-data`, and the generic endpoint finds the model by reading a
//! JSON body, so it refuses a multipart request with `body_not_json`. The
//! dedicated shape carries the model in the path and the body is never parsed.
//! Verified against a running router: `POST /models/whisper/v1/audio/transcriptions`
//! answered 200 with a transcript, and the same request to `/v1/audio/transcriptions`
//! answered 400 `body_not_json`.
//!
//! **Causes are told apart by code, never by prose.** The router answers a
//! refusal as JSON carrying a stable machine-readable `code`; its messages are
//! for people and change freely.

pub mod audio;
/// Reading and writing the few JSON string fields this daemon exchanges.
///
/// Public because `crate::agent` reads Herdr's error code with it: telling
/// causes apart by code rather than by prose is a rule both boundaries follow,
/// and two spellings of it would eventually disagree.
pub mod json;

use crate::capture::SAMPLE_RATE;
use crate::http;
use crate::service::{Synthesizer, Transcriber, Transcript};
use crate::speak::Language;
use crate::turn::Fault;

/// Where the transcription service answers, under its model.
const TRANSCRIPTIONS: &str = "v1/audio/transcriptions";

/// Where the synthesis service answers, under its model.
const SPEECH: &str = "v1/audio/speech";

/// The router, and which of its entries to ask for.
#[derive(Debug, Clone)]
pub struct Router {
    address: String,
    transcriber: String,
    synthesizer: String,
}

impl Router {
    /// A client for the router at `address`, using the two named entries.
    #[must_use]
    pub fn new(address: &str, transcriber: &str, synthesizer: &str) -> Self {
        Self {
            address: address.to_owned(),
            transcriber: transcriber.to_owned(),
            synthesizer: synthesizer.to_owned(),
        }
    }

    /// The dedicated path for one entry.
    fn path(model: &str, suffix: &str) -> String {
        format!("/models/{model}/{suffix}")
    }

    /// What a reply that is not a success means for a turn.
    ///
    /// Every refusal ends the turn the same way, so this exists to put the code
    /// in the log rather than to branch on it. Reading the code rather than the
    /// message is what keeps that log useful when the router rewords itself.
    fn refused(body: &str) -> Fault {
        let code = json::field(body, "code").unwrap_or_else(|| "no code".to_owned());
        eprintln!("maestro-voice: the router refused: {code}");
        Fault::Refused
    }
}

impl Transcriber for Router {
    /// Ask the router to load the transcriber, without waiting for a result.
    ///
    /// `GET /models/<id>/health` relays to the child, and relaying to a child
    /// that is not running is what starts it. Verified against a running
    /// router: the request answered 200 and the router logged the entry moving
    /// from loading to ready, with `/models` then reporting it loaded.
    ///
    /// Best effort throughout. The transcription that follows would load the
    /// model anyway, so a warm that fails costs latency and never correctness,
    /// and it is never announced.
    fn warm(&self) {
        let path = Self::path(&self.transcriber, "health");
        if let Err(why) = http::get(&self.address, &path) {
            eprintln!("maestro-voice: could not warm the transcriber: {why}");
        }
    }

    fn transcribe(&self, wav: &[u8]) -> Result<Transcript, Fault> {
        let path = Self::path(&self.transcriber, TRANSCRIPTIONS);
        let reply = http::post_file(
            &self.address,
            &path,
            ("file", "utterance.wav", wav),
            &[
                ("model", &self.transcriber),
                ("response_format", "json"),
                // No language: the owner speaks two, and one is detected per
                // utterance so the reply comes back in whichever was used.
                // Naming one here would silently transcribe the other wrongly.
            ],
        )
        .map_err(|why| {
            eprintln!("maestro-voice: the router did not answer: {why}");
            Fault::Unreachable
        })?;

        if !reply.ok() {
            return Err(Self::refused(&reply.text()));
        }

        let body = reply.text();
        Ok(Transcript {
            text: json::field(&body, "text").unwrap_or_default(),
            language: json::field(&body, "language")
                .as_deref()
                .and_then(Language::new),
        })
    }
}

impl Synthesizer for Router {
    fn synthesize(&self, text: &str, language: &Language) -> Option<Vec<i16>> {
        let path = Self::path(&self.synthesizer, SPEECH);
        let body = format!(
            "{{\"model\":{},\"input\":{},\"language\":{},\"response_format\":\"wav\"}}",
            json::quote(&self.synthesizer),
            json::quote(text),
            json::quote(language.as_str()),
        );

        let reply = match http::post_json(&self.address, &path, &body) {
            Ok(reply) => reply,
            Err(why) => {
                eprintln!("maestro-voice: the router did not answer: {why}");
                return None;
            }
        };
        if !reply.ok() {
            // The cause is logged for the record; every refusal ends the reply
            // the same way, so there is nothing here to branch on.
            let _refused = Router::refused(&reply.text());
            return None;
        }

        match audio::decode(&reply.body) {
            Ok((samples, rate)) => Some(audio::resample(&samples, rate, SAMPLE_RATE)),
            Err(why) => {
                eprintln!("maestro-voice: the reply was not playable audio: {why}");
                None
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::Router;

    #[test]
    fn a_request_names_its_model_in_the_path_rather_than_the_body() {
        // The generic endpoint refuses a multipart body with body_not_json, so
        // getting this shape wrong breaks every transcription.
        assert_eq!(
            Router::path("whisper", super::TRANSCRIPTIONS),
            "/models/whisper/v1/audio/transcriptions"
        );
        assert_eq!(
            Router::path("tts", super::SPEECH),
            "/models/tts/v1/audio/speech"
        );
    }

    #[test]
    fn the_warm_request_addresses_the_child_health_endpoint() {
        assert_eq!(Router::path("whisper", "health"), "/models/whisper/health");
    }
}
