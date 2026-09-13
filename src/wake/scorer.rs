//! openWakeWord's streaming pipeline, reimplemented.
//!
//! Every constant here was read out of openwakeword 0.4.0 rather than
//! recalled: `utils.py` owns the framing and `model.py` owns the prediction
//! rules. A port built from that reading reproduced the package's per-chunk
//! scores exactly, which is the property
//! `tests/wake_equivalence.rs` keeps true.
//!
//! Three things in here are wrong in ways nothing reports. Each is marked
//! where it happens, and each has a test that fails when it changes:
//!
//! 1. The mel transform is `x / 10 + 2`, elementwise.
//! 2. Samples are int16 *values* widened to `f32`, never scaled to unit range.
//!    Dividing by 32768 is accepted by the session and quietly wrong.
//! 3. The feature buffer starts as the embeddings of ten seconds of silence,
//!    not empty, which is why the first five scores are forced to zero.
//!
//! This module is the only place that imports the ONNX runtime. Everything
//! above it sees scores, so replacing the engine is a change here and nowhere
//! else -- which matters more than usual, because the engine's licence
//! constrains the whole project. See `docs/adr/0003`.

use std::collections::VecDeque;

use ort::session::Session;
use ort::value::Tensor;

use super::error::Error;
use super::models::ModelSet;

/// The only rate the melspectrogram model was traced for.
pub const SAMPLE_RATE: usize = 16_000;
/// The streaming chunk openWakeWord documents: 80 ms.
pub const CHUNK: usize = 1280;
/// `mel_bins = 32  # fixed by melspectrogram model`.
const MEL_BINS: usize = 32;
/// `_get_embeddings(..., window_size: int = 76, step_size: int = 8)`.
const MEL_WINDOW: usize = 76;
const MEL_STEP: usize = 8;
/// `self.melspectrogram_max_len = 10*97`.
const MEL_MAX_ROWS: usize = 970;
/// Three hops of extra context, because the mel model yields
/// `ceil(n/160 - 3)` frames and would otherwise lose the join between chunks.
const MEL_CONTEXT: usize = 480;
/// `embedding_dim = 96  # fixed by embedding model`.
const FEATURE_DIM: usize = 96;
/// `self.feature_buffer_max_len = 120`.
const FEATURE_MAX_ROWS: usize = 120;
/// `deque(maxlen=sr*10)`.
const RAW_MAX: usize = SAMPLE_RATE * 10;
/// The head's input is `[1, 16, 96]`.
const HEAD_ROWS: usize = 16;
/// "zero predictions for first 5 frames during model initialization".
const WARMUP: usize = 5;
/// Ten seconds of digital silence, which the feature buffer starts as.
const SILENCE_SAMPLES: usize = SAMPLE_RATE * 10;

/// The one place the inference engine's error crosses into this crate's own.
///
/// `ort::Error` is generic over the builder it came from, so this covers every
/// stage: loading a session and running one report the same way. Keeping the
/// conversion here rather than beside the error type is what lets `error.rs`,
/// and everything above the wake stage, avoid naming `ort` at all.
impl<R> From<ort::Error<R>> for Error {
    fn from(error: ort::Error<R>) -> Self {
        Self::Runtime(error.to_string())
    }
}

/// Turns 80-millisecond chunks of 16 kHz mono audio into wake scores.
///
/// One instance carries the whole streaming state, so scores depend on every
/// chunk fed before them. Feeding a fresh recording means a fresh scorer.
pub struct Scorer {
    melspectrogram: Session,
    embedding: Session,
    head: Session,
    raw: VecDeque<i16>,
    /// Row-major, `MEL_BINS` values per row.
    mel: Vec<f32>,
    /// Row-major, `FEATURE_DIM` values per row.
    features: Vec<f32>,
    accumulated: usize,
    emitted: usize,
}

impl Scorer {
    /// A scorer over the given weights, warmed exactly as the package warms it.
    ///
    /// # Errors
    ///
    /// Returns [`Error::Runtime`] when a model will not load, which at this
    /// point means a corrupt or truncated file rather than a missing one:
    /// [`ModelSet`] has already established that all three exist.
    pub fn open(models: &ModelSet) -> Result<Self, Error> {
        let [melspectrogram, embedding, head] = models.paths();
        let mut scorer = Self {
            melspectrogram: session(&melspectrogram)?,
            embedding: session(&embedding)?,
            head: session(&head)?,
            raw: VecDeque::with_capacity(RAW_MAX),
            // `np.ones((76, 32))`. Seeding with zeros changes every score.
            mel: vec![1.0; MEL_WINDOW * MEL_BINS],
            features: Vec::new(),
            accumulated: 0,
            emitted: 0,
        };
        scorer.features = scorer.embeddings_of_silence()?;
        Ok(scorer)
    }

    /// Feed one chunk and take its score.
    ///
    /// # Errors
    ///
    /// Returns [`Error::Runtime`] when any of the three sessions fails.
    pub fn push(&mut self, chunk: &[i16]) -> Result<f32, Error> {
        self.raw.extend(chunk.iter().copied());
        while self.raw.len() > RAW_MAX {
            self.raw.pop_front();
        }
        self.accumulated += chunk.len();

        if self.accumulated >= CHUNK {
            self.extend_melspectrogram()?;
            self.extend_features()?;
            self.accumulated = 0;
        }
        trim(&mut self.features, FEATURE_DIM, FEATURE_MAX_ROWS);

        self.predict()
    }

    /// The melspectrogram of one span, transformed as the package transforms it.
    fn melspectrogram_of(&mut self, samples: &[i16]) -> Result<Vec<f32>, Error> {
        // int16 values widened, never scaled. Dividing by 32768 here is the
        // plausible-looking bug the session accepts without complaint.
        let input: Vec<f32> = samples.iter().map(|&s| f32::from(s)).collect();
        let shape = vec![1, length(input.len())];
        let outputs = self
            .melspectrogram
            .run(ort::inputs![Tensor::from_array((shape, input))?])?;
        let (_, data) = outputs[0].try_extract_tensor::<f32>()?;
        // `melspec_transform: Callable = lambda x: x/10 + 2`.
        Ok(data.iter().map(|value| value / 10.0 + 2.0).collect())
    }

    /// One 96-value embedding per `MEL_WINDOW`-row window.
    fn embed(&mut self, windows: Vec<f32>, count: usize) -> Result<Vec<f32>, Error> {
        let shape = vec![length(count), length(MEL_WINDOW), length(MEL_BINS), 1];
        let outputs = self
            .embedding
            .run(ort::inputs![Tensor::from_array((shape, windows))?])?;
        let (_, data) = outputs[0].try_extract_tensor::<f32>()?;
        Ok(data.to_vec())
    }

    /// The initial feature buffer: 116 rows of embedded silence.
    fn embeddings_of_silence(&mut self) -> Result<Vec<f32>, Error> {
        let mel = self.melspectrogram_of(&vec![0_i16; SILENCE_SAMPLES])?;
        let rows = mel.len() / MEL_BINS;

        let mut batch = Vec::new();
        let mut count = 0;
        let mut start = 0;
        while start + MEL_WINDOW <= rows {
            batch.extend_from_slice(&mel[start * MEL_BINS..(start + MEL_WINDOW) * MEL_BINS]);
            count += 1;
            start += MEL_STEP;
        }
        self.embed(batch, count)
    }

    fn extend_melspectrogram(&mut self) -> Result<(), Error> {
        let want = (self.accumulated + MEL_CONTEXT).min(self.raw.len());
        let span: Vec<i16> = self
            .raw
            .iter()
            .skip(self.raw.len() - want)
            .copied()
            .collect();

        let fresh = self.melspectrogram_of(&span)?;
        self.mel.extend_from_slice(&fresh);
        trim(&mut self.mel, MEL_BINS, MEL_MAX_ROWS);
        Ok(())
    }

    /// One embedding per 1280 samples that arrived, oldest first, each over the
    /// 76 mel rows ending `8 * i` back.
    fn extend_features(&mut self) -> Result<(), Error> {
        let rows = self.mel.len() / MEL_BINS;
        for i in (0..self.accumulated / CHUNK).rev() {
            let end = rows - MEL_STEP * i;
            if end < MEL_WINDOW {
                continue;
            }
            let window = self.mel[(end - MEL_WINDOW) * MEL_BINS..end * MEL_BINS].to_vec();
            let embedding = self.embed(window, 1)?;
            self.features.extend_from_slice(&embedding);
        }
        Ok(())
    }

    fn predict(&mut self) -> Result<f32, Error> {
        let rows = self.features.len() / FEATURE_DIM;
        let tail = self.features[(rows - HEAD_ROWS) * FEATURE_DIM..].to_vec();
        let shape = vec![1, length(HEAD_ROWS), length(FEATURE_DIM)];
        let outputs = self
            .head
            .run(ort::inputs![Tensor::from_array((shape, tail))?])?;
        let (_, data) = outputs[0].try_extract_tensor::<f32>()?;

        let score = if self.emitted < WARMUP {
            0.0
        } else {
            data.first().copied().unwrap_or(0.0)
        };
        self.emitted += 1;
        Ok(score)
    }
}

/// Matching openWakeWord's session options, so neither side gains parallelism
/// the other does not have.
fn session(path: &std::path::Path) -> Result<Session, Error> {
    Ok(Session::builder()?
        .with_intra_threads(1)?
        .with_inter_threads(1)?
        .commit_from_file(path)?)
}

/// Drop whole rows from the front until at most `max_rows` remain.
fn trim(buffer: &mut Vec<f32>, width: usize, max_rows: usize) {
    let rows = buffer.len() / width;
    if rows > max_rows {
        buffer.drain(..(rows - max_rows) * width);
    }
}

/// A dimension as the runtime wants it. Sizes here are bounded by the buffer
/// caps above, all far below `i64::MAX`.
#[expect(clippy::cast_possible_wrap, reason = "bounded by the buffer caps")]
const fn length(value: usize) -> i64 {
    value as i64
}
