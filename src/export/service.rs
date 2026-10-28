//! Runs an export request: scale, lay out, encode and write files.
//!
//! [`run`] is synchronous and used by tests; [`ExportJob::spawn`] runs the same code on
//! a background thread with progress and cancellation for the UI. Every file is
//! encoded in memory first and then written atomically (temporary file + rename), so a
//! failed or canceled export never leaves half-written files behind.

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, MutexGuard};

use super::render::{scale_nearest, FrameImage};
use super::{gif, metadata, spritesheet, still};
use super::{
    ExportError, ExportSummary, GifOptions, JpegOptions, MetadataFormat, SheetOptions, StillFormat,
};
use crate::file_io::write_atomic;

/// Where and how to write.
#[derive(Debug, Clone, PartialEq)]
pub enum ExportTarget {
    /// One still image per frame; `paths.len()` equals the number of frames.
    Images {
        format: StillFormat,
        paths: Vec<PathBuf>,
    },
    /// One animated GIF.
    Gif { path: PathBuf },
    /// One sprite sheet image, optionally with JSON next to it.
    Sheet {
        image_path: PathBuf,
        json_path: Option<PathBuf>,
    },
}

/// A complete, self-contained export job (owns its frame snapshot).
#[derive(Debug, Clone)]
pub struct ExportRequest {
    /// Unscaled frames from [`super::render::snapshot`].
    pub frames: Vec<FrameImage>,
    pub scale: u32,
    pub jpeg: JpegOptions,
    pub gif: GifOptions,
    pub sheet: SheetOptions,
    /// Base name and digits used for frame names in metadata.
    pub base_name: String,
    pub digits: u8,
    pub target: ExportTarget,
}

impl ExportRequest {
    /// Every file this request will create or replace.
    pub fn output_paths(&self) -> Vec<PathBuf> {
        match &self.target {
            ExportTarget::Images { paths, .. } => paths.clone(),
            ExportTarget::Gif { path } => vec![path.clone()],
            ExportTarget::Sheet {
                image_path,
                json_path,
            } => std::iter::once(image_path.clone())
                .chain(json_path.clone())
                .collect(),
        }
    }

    /// Steps reported by progress (frames to process + files to write).
    pub fn total_steps(&self) -> usize {
        match &self.target {
            ExportTarget::Images { .. } | ExportTarget::Gif { .. } => self.frames.len(),
            ExportTarget::Sheet { .. } => self.frames.len() + 1,
        }
    }
}

fn write_bytes(path: &Path, bytes: &[u8]) -> Result<(), ExportError> {
    write_atomic(path, |w| {
        std::io::Write::write_all(w, bytes)?;
        Ok(())
    })?;
    Ok(())
}

fn file_name(path: &Path) -> String {
    path.file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_default()
}

/// Runs the request to completion. `progress(done, total)` is called as work advances.
pub fn run(
    request: &ExportRequest,
    cancel: &AtomicBool,
    progress: &mut dyn FnMut(usize, usize),
) -> Result<ExportSummary, ExportError> {
    if request.frames.is_empty() {
        return Err(ExportError::NoFrames);
    }
    let total = request.total_steps();
    let canceled = || cancel.load(Ordering::Relaxed);
    let mut summary = ExportSummary::default();
    match &request.target {
        ExportTarget::Images { format, paths } => {
            if paths.len() != request.frames.len() {
                return Err(ExportError::Invalid(
                    "one file name per frame is needed".into(),
                ));
            }
            let mut flattened = false;
            for (i, (frame, path)) in request.frames.iter().zip(paths).enumerate() {
                if canceled() {
                    return Err(ExportError::Canceled);
                }
                let img = scale_nearest(&frame.image, request.scale)?;
                flattened |= *format == StillFormat::Jpeg && img.pixels().any(|p| p.0[3] < 255);
                let mut bytes = Vec::new();
                still::encode(*format, &img, &request.jpeg, &mut bytes)?;
                write_bytes(path, &bytes)?;
                summary.files.push(path.clone());
                progress(i + 1, total);
            }
            if flattened {
                summary.notes.push(
                    "Transparent pixels were filled with the JPEG background color.".to_owned(),
                );
            }
        }
        ExportTarget::Gif { path } => {
            let mut frames = Vec::with_capacity(request.frames.len());
            for f in &request.frames {
                if canceled() {
                    return Err(ExportError::Canceled);
                }
                frames.push(FrameImage {
                    image: scale_nearest(&f.image, request.scale)?,
                    ..f.clone()
                });
            }
            let mut bytes = Vec::new();
            let report = gif::encode(&frames, &request.gif, &mut bytes, &canceled, &mut |done| {
                progress(done, total)
            })?;
            write_bytes(path, &bytes)?;
            summary.files.push(path.clone());
            if report.reduced {
                summary.notes.push(format!(
                    "The animation had more than 255 colors; GIF reduced them to 255{}.",
                    if request.gif.dither {
                        " with dithering"
                    } else {
                        ""
                    }
                ));
            }
            if report.soft_alpha {
                summary.notes.push(
                    "Semi-transparent pixels became fully opaque or transparent (GIF has no soft transparency).".to_owned(),
                );
            }
        }
        ExportTarget::Sheet {
            image_path,
            json_path,
        } => {
            let mut frames = Vec::with_capacity(request.frames.len());
            for (i, f) in request.frames.iter().enumerate() {
                if canceled() {
                    return Err(ExportError::Canceled);
                }
                frames.push(FrameImage {
                    image: scale_nearest(&f.image, request.scale)?,
                    ..f.clone()
                });
                progress(i + 1, total);
            }
            let plan = spritesheet::plan(&frames, &request.sheet)?;
            let sheet = spritesheet::compose(&frames, &plan, &request.sheet)?;
            if canceled() {
                return Err(ExportError::Canceled);
            }
            let format = match request.sheet.image_format {
                StillFormat::Jpeg => StillFormat::Png,
                other => other,
            };
            let mut bytes = Vec::new();
            still::encode(format, &sheet, &request.jpeg, &mut bytes)?;
            let json = match (json_path, request.sheet.metadata) {
                (Some(_), MetadataFormat::Kagero) => Some(metadata::kagero_json(
                    &plan,
                    &request.sheet,
                    &file_name(image_path),
                    &request.base_name,
                    request.digits,
                    request.scale,
                )),
                (Some(_), MetadataFormat::JsonHash) => Some(metadata::json_hash(
                    &plan,
                    &file_name(image_path),
                    &request.base_name,
                    request.digits,
                    request.scale,
                )),
                _ => None,
            }
            .transpose()
            .map_err(|e| ExportError::encode("JSON", e))?;
            write_bytes(image_path, &bytes)?;
            summary.files.push(image_path.clone());
            if let (Some(path), Some(json)) = (json_path, json) {
                write_bytes(path, json.as_bytes())?;
                summary.files.push(path.clone());
            }
            progress(total, total);
        }
    }
    Ok(summary)
}

/// Progress of a background export.
#[derive(Debug, Clone, Default)]
pub struct JobState {
    pub done: usize,
    pub total: usize,
    /// Set when the job ends: the summary, or a user-facing error message.
    pub finished: Option<Result<ExportSummary, String>>,
}

/// A running background export.
pub struct ExportJob {
    state: Arc<Mutex<JobState>>,
    cancel: Arc<AtomicBool>,
}

impl ExportJob {
    fn lock(state: &Mutex<JobState>) -> MutexGuard<'_, JobState> {
        state.lock().unwrap_or_else(|e| e.into_inner())
    }

    /// Starts the export on a background thread. `repaint` wakes the UI on progress.
    pub fn spawn(request: ExportRequest, repaint: Arc<dyn Fn() + Send + Sync>) -> Self {
        let state = Arc::new(Mutex::new(JobState {
            done: 0,
            total: request.total_steps(),
            finished: None,
        }));
        let cancel = Arc::new(AtomicBool::new(false));
        let (s, c) = (state.clone(), cancel.clone());
        let spawned = std::thread::Builder::new()
            .name("kagero-export".into())
            .spawn(move || {
                let result = run(&request, &c, &mut |done, total| {
                    let mut st = Self::lock(&s);
                    st.done = done;
                    st.total = total;
                    drop(st);
                    repaint();
                });
                if let Err(e) = &result {
                    tracing::warn!("export failed: {e}");
                }
                Self::lock(&s).finished = Some(result.map_err(|e| e.to_string()));
                repaint();
            });
        if let Err(e) = spawned {
            Self::lock(&state).finished = Some(Err(format!("could not start the export: {e}")));
        }
        Self { state, cancel }
    }

    pub fn state(&self) -> JobState {
        Self::lock(&self.state).clone()
    }

    pub fn cancel(&self) {
        self.cancel.store(true, Ordering::Relaxed);
    }
}
