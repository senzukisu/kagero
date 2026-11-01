//! Animation preview: real-time looping playback of the document's frames.
//!
//! Timing uses a monotonic clock and maps elapsed time to a frame index, so frames are
//! never skipped or slowed by how often the UI repaints, and errors do not accumulate.

use std::ops::Range;
use std::time::{Duration, Instant};

use crate::config::{PlaybackSettings, PlaybackTiming};
use crate::core::animation::frame_at_time;
use crate::editor::Editor;

#[derive(Debug, Default)]
pub struct Player {
    /// When playback started and the animation time (ms) at that moment.
    clock: Option<(Instant, u64)>,
    /// Frames being looped (a tag), or `None` for all of them.
    range: Option<Range<usize>>,
}

impl Player {
    pub fn is_playing(&self) -> bool {
        self.clock.is_some()
    }

    /// Per-frame durations used for playback.
    fn durations(
        editor: &Editor,
        settings: &PlaybackSettings,
        range: &Option<Range<usize>>,
    ) -> Vec<u32> {
        let all = editor.document().frames();
        let frames = match range {
            Some(r) => &all[r.start.min(all.len())..r.end.min(all.len())],
            None => all,
        };
        match settings.timing {
            PlaybackTiming::FrameDurations => frames.iter().map(|f| f.duration_ms()).collect(),
            PlaybackTiming::FixedFps => {
                let ms = 1000 / settings.fps.max(1);
                vec![ms; frames.len()]
            }
        }
    }

    /// Starts looping from the active frame — within `range` (a tag) if given, from
    /// its first frame when the active one is outside. Pixel editing is blocked while
    /// playing.
    pub fn start(
        &mut self,
        editor: &mut Editor,
        settings: &PlaybackSettings,
        range: Option<Range<usize>>,
    ) {
        let count = editor.document().frame_count();
        self.range = range.filter(|r| !r.is_empty() && r.end <= count);
        let first = self.range.as_ref().map_or(0, |r| r.start);
        let active = editor.document().active_frame_index();
        let relative = match &self.range {
            Some(r) if !r.contains(&active) => {
                editor.select_frame(first);
                0
            }
            _ => active - first,
        };
        let durations = Self::durations(editor, settings, &self.range);
        let offset: u64 = durations[..relative.min(durations.len())]
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

    /// Forgets playback without touching an editor (e.g. when the document closes).
    pub fn reset(&mut self) {
        self.clock = None;
    }

    /// Advances to the frame for the current time and schedules the next repaint.
    pub fn tick(&mut self, ctx: &egui::Context, editor: &mut Editor, settings: &PlaybackSettings) {
        let Some((started, offset)) = self.clock else {
            return;
        };
        let durations = Self::durations(editor, settings, &self.range);
        let first = self.range.as_ref().map_or(0, |r| r.start);
        let elapsed = offset + started.elapsed().as_millis() as u64;
        match frame_at_time(&durations, elapsed) {
            Some((index, remaining_ms)) => {
                editor.select_frame(first + index);
                ctx.request_repaint_after(Duration::from_millis(remaining_ms.max(1)));
            }
            None => self.stop(editor),
        }
    }
}
