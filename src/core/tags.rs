//! Frame tags: named, colored ranges of frames ("walk", "idle", "attack").
//!
//! A tag is a range of frame *positions* (`start..=end`, 0-based). Reordering frames
//! keeps every range where it is: a tag marks a stretch of the timeline, and frames
//! can be moved through it. Inserting a frame inside a tag, or right after its last
//! frame, grows the tag; deleting one shrinks it; a tag whose last frame is deleted
//! disappears (undo brings it back).

use super::Rgba;

/// Most tags a document can have.
pub const MAX_TAGS: usize = 256;
/// Longest tag name, in characters.
pub const MAX_TAG_NAME_CHARS: usize = 48;

/// Colors offered for tags (new tags cycle through them).
pub const TAG_COLORS: [Rgba; 8] = [
    Rgba::opaque(0xE8, 0x6A, 0x33), // ember
    Rgba::opaque(0xE0, 0xB0, 0x3A), // amber
    Rgba::opaque(0x7C, 0xB3, 0x4E), // moss
    Rgba::opaque(0x3F, 0xA7, 0x9A), // teal
    Rgba::opaque(0x4A, 0x8F, 0xD8), // sky
    Rgba::opaque(0x8E, 0x6C, 0xD9), // violet
    Rgba::opaque(0xD9, 0x5F, 0xA8), // sakura
    Rgba::opaque(0x9A, 0x9A, 0xA6), // stone
];

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FrameTag {
    pub name: String,
    /// First frame (0-based, inclusive).
    pub start: usize,
    /// Last frame (0-based, inclusive); never before `start`.
    pub end: usize,
    pub color: Rgba,
}

impl FrameTag {
    pub fn contains(&self, frame: usize) -> bool {
        (self.start..=self.end).contains(&frame)
    }

    /// Frame positions of the tag, clamped to `frame_count`.
    pub fn frames(&self, frame_count: usize) -> std::ops::Range<usize> {
        let end = (self.end + 1).min(frame_count);
        self.start.min(end)..end
    }
}

/// A tag name trimmed and shortened to the limit, or `None` if nothing is left.
pub fn clean_tag_name(name: &str) -> Option<String> {
    let name: String = name
        .trim()
        .chars()
        .filter(|c| !c.is_control())
        .take(MAX_TAG_NAME_CHARS)
        .collect();
    let name = name.trim().to_owned();
    (!name.is_empty()).then_some(name)
}

/// Tags that are valid for `frame_count` frames: ranges inside the document, clean
/// names, at most [`MAX_TAGS`]. Invalid tags are dropped, not repaired.
pub fn sanitize_tags(tags: Vec<FrameTag>, frame_count: usize) -> Vec<FrameTag> {
    tags.into_iter()
        .filter(|t| t.start <= t.end && t.end < frame_count)
        .filter_map(|t| {
            Some(FrameTag {
                name: clean_tag_name(&t.name)?,
                ..t
            })
        })
        .take(MAX_TAGS)
        .collect()
}

/// Tags after a frame was inserted at position `index`.
pub fn after_insert(tags: &[FrameTag], index: usize) -> Vec<FrameTag> {
    tags.iter()
        .map(|t| {
            let mut t = t.clone();
            if index <= t.start {
                t.start += 1;
                t.end += 1;
            } else if index <= t.end + 1 {
                t.end += 1;
            }
            t
        })
        .collect()
}

/// Tags after the frame at position `index` was removed.
pub fn after_remove(tags: &[FrameTag], index: usize) -> Vec<FrameTag> {
    tags.iter()
        .filter(|t| !(t.start == index && t.end == index))
        .map(|t| {
            let mut t = t.clone();
            if index < t.start {
                t.start -= 1;
                t.end -= 1;
            } else if index <= t.end {
                t.end -= 1;
            }
            t
        })
        .collect()
}

/// A name like "Tag 3" that no tag uses yet.
pub fn unused_tag_name(tags: &[FrameTag]) -> String {
    (tags.len() + 1..)
        .map(|n| format!("Tag {n}"))
        .find(|n| !tags.iter().any(|t| &t.name == n))
        .unwrap_or_else(|| "Tag".to_owned())
}

/// The color the next new tag gets.
pub fn next_tag_color(tags: &[FrameTag]) -> Rgba {
    TAG_COLORS
        .iter()
        .copied()
        .find(|c| !tags.iter().any(|t| t.color == *c))
        .unwrap_or(TAG_COLORS[tags.len() % TAG_COLORS.len()])
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tag(start: usize, end: usize) -> FrameTag {
        FrameTag {
            name: "t".into(),
            start,
            end,
            color: TAG_COLORS[0],
        }
    }

    fn ranges(tags: &[FrameTag]) -> Vec<(usize, usize)> {
        tags.iter().map(|t| (t.start, t.end)).collect()
    }

    #[test]
    fn insert_grows_or_shifts() {
        let tags = [tag(2, 4)];
        assert_eq!(ranges(&after_insert(&tags, 0)), [(3, 5)]);
        assert_eq!(ranges(&after_insert(&tags, 2)), [(3, 5)]);
        assert_eq!(ranges(&after_insert(&tags, 3)), [(2, 5)]);
        // Right after the last frame (e.g. duplicating it) joins the tag.
        assert_eq!(ranges(&after_insert(&tags, 5)), [(2, 5)]);
        assert_eq!(ranges(&after_insert(&tags, 6)), [(2, 4)]);
    }

    #[test]
    fn remove_shrinks_shifts_or_drops() {
        let tags = [tag(2, 4), tag(6, 6)];
        assert_eq!(ranges(&after_remove(&tags, 0)), [(1, 3), (5, 5)]);
        assert_eq!(ranges(&after_remove(&tags, 2)), [(2, 3), (5, 5)]);
        assert_eq!(ranges(&after_remove(&tags, 4)), [(2, 3), (5, 5)]);
        assert_eq!(ranges(&after_remove(&tags, 6)), [(2, 4)]);
    }

    #[test]
    fn sanitize_drops_bad_ranges_and_names() {
        let mut bad_name = tag(0, 0);
        bad_name.name = "  \n ".into();
        let mut long = tag(0, 1);
        long.name = "x".repeat(100);
        let out = sanitize_tags(vec![tag(3, 1), tag(0, 9), bad_name, long, tag(1, 2)], 3);
        assert_eq!(ranges(&out), [(0, 1), (1, 2)]);
        assert_eq!(out[0].name.chars().count(), MAX_TAG_NAME_CHARS);
    }

    #[test]
    fn names_and_colors_are_fresh() {
        let mut a = tag(0, 0);
        a.name = "Tag 2".into();
        assert_eq!(unused_tag_name(&[a.clone()]), "Tag 3");
        assert_eq!(next_tag_color(&[a]), TAG_COLORS[1]);
    }
}
