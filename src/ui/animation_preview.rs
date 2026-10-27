//! Animation preview: real-time looping playback of the document's frames.
//!
//! Timing uses a monotonic clock and maps elapsed time to a frame index, so frames are
//! never skipped or slowed by how often the UI repaints, and errors do not accumulate.

use std::time::{Duration, Instant};

use crate::config::{PlaybackSettings, PlaybackTiming};
use crate::core::animation::frame_at_time;
use crate::editor::Editor;

#[derive(Debug, Default)]
pub struct Player {
    /// When playback started and the animation time (ms) at that moment.
    clock: Option<(Instant, u64)>,
}

impl Player {
    pub fn is_playing(&self) -> bool {
        self.clock.is_some()
    }

    /// Per-frame durations used for playback.
    fn durations(editor: &Editor, settings: &PlaybackSettings) -> Vec<u32> {
        let frames = editor.document().frames();
        match settings.timing {
            PlaybackTiming::FrameDurations => frames.iter().map(|f| f.duration_ms()).collect(),
            PlaybackTiming::FixedFps => {
                let ms = 1000 / settings.fps.max(1);
                vec![ms; frames.len()]
            }
        }
    }

    /// Starts looping from the active frame. Pixel editing is blocked while playing.
    pub fn start(&mut self, editor: &mut Editor, settings: &PlaybackSettings) {
        let durations = Self::durations(editor, settings);
        let active = editor.document().active_frame_index();
        let offset: u64 = durations[..active.min(durations.len())]
            .iter()
            .map(|&d| u64::from(d))
            .sum();
        editor.set_read_only(true);
        self.clock = Some((Instant::now(), offset));
    }

    /// Stops playback and keeps the frame that was showing.
    pub fn stop(&mut self, editor: &mut Editor) {
        if self.clock.take().is_some() {
            editor.set_read_only(false);
            // The canvas skipped incremental uploads while playing.
            editor.mark_all_dirty();
        }
    }

    pub fn toggle(&mut self, editor: &mut Editor, settings: &PlaybackSettings) {
        if self.is_playing() {
            self.stop(editor);
        } else {
            self.start(editor, settings);
        }
    }

    /// Forgets playback without touching an editor (e.g. when the document closes).
    pub fn reset(&mut self) {
        self.clock = None;
    }

    /// Advances to the frame for the current time and schedules the next repaint.
    pub fn tick(&mut self, ctx: &egui::Context, editor: &mut Editor, settings: &PlaybackSettings) {
        let Some((started, offset)) = self.clock else {
            return;
        };
        let durations = Self::durations(editor, settings);
        let elapsed = offset + started.elapsed().as_millis() as u64;
        match frame_at_time(&durations, elapsed) {
            Some((index, remaining_ms)) => {
                editor.select_frame(index);
                ctx.request_repaint_after(Duration::from_millis(remaining_ms.max(1)));
            }
            None => self.stop(editor),
        }
    }
}
