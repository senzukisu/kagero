//! The `.kgr` project format: a ZIP archive with `manifest.json`, `document.json`
//! and one PNG per layer per frame. See `docs/PROJECT_FORMAT.md`.
//!
//! Version 2 stores animation frames. Version 1 files (Kagero 0.1, one image with layers)
//! are still read and become a single-frame document.
//!
//! Entries are read by exact name into memory; nothing is ever extracted to disk, so
//! paths stored inside an archive cannot point anywhere on the file system.

use std::collections::HashSet;
use std::io::{Read, Seek, Write};
use std::path::Path;

use serde::{Deserialize, Serialize};
use zip::write::SimpleFileOptions;
use zip::{CompressionMethod, ZipArchive, ZipWriter};

use super::png::{decode_png, encode_png};
use super::{check_file_size, write_atomic, FileError, MAX_KGR_FILE_BYTES};
use crate::core::animation::DEFAULT_FRAME_DURATION_MS;
use crate::core::{
    layers_bytes, sanitize_layer_name, validate_size, Document, PixelBuffer, MAX_CANVAS_SIZE,
    MAX_DOCUMENT_BYTES, MAX_FRAMES, MAX_LAYERS,
};

pub const KGR_EXTENSION: &str = "kgr";
/// Identifier stored in the manifest.
pub const KGR_FORMAT_ID: &str = "kagero-project";
/// Current (and highest readable) format version.
pub const KGR_FORMAT_VERSION: u32 = 2;

const MANIFEST: &str = "manifest.json";
const DOCUMENT: &str = "document.json";
const MAX_JSON_BYTES: u64 = 8 * 1024 * 1024;
/// Hard cap on archive entries before `document.json` is read.
const MAX_ENTRIES: usize = MAX_LAYERS * MAX_FRAMES + 16;
/// Largest stored layer PNG (raw RGBA at max size plus generous PNG overhead).
const MAX_LAYER_PNG_BYTES: u64 = (MAX_CANVAS_SIZE as u64).pow(2) * 4 + 4 * 1024 * 1024;

#[derive(Debug, Serialize, Deserialize)]
struct Manifest {
    format: String,
    format_version: u32,
    #[serde(default)]
    app_version: String,
}

// ---------------------------------------------------------------- version 1

#[derive(Debug, Deserialize)]
struct DocumentV1 {
    width: u32,
    height: u32,
    #[serde(default)]
    active_layer: usize,
    /// Bottom to top.
    layers: Vec<LayerV1>,
}

#[derive(Debug, Deserialize)]
struct LayerV1 {
    name: String,
    visible: bool,
    file: String,
}

fn v1_layer_file(index: usize) -> String {
    format!("layers/{index:04}.png")
}

// ---------------------------------------------------------------- version 2

#[derive(Debug, Serialize, Deserialize)]
struct DocumentV2 {
    width: u32,
    height: u32,
    #[serde(default)]
    active_layer: usize,
    #[serde(default)]
    active_frame: usize,
    /// Bottom to top; shared by all frames.
    layers: Vec<LayerV2>,
    /// Playback order.
    frames: Vec<FrameV2>,
}

#[derive(Debug, Serialize, Deserialize)]
struct LayerV2 {
    id: u64,
    name: String,
    visible: bool,
}

#[derive(Debug, Serialize, Deserialize)]
struct FrameV2 {
    id: u64,
    duration_ms: u32,
    /// One PNG per layer, same order as `layers`.
    cels: Vec<String>,
}

fn cel_file(frame: usize, layer: usize) -> String {
    format!("frames/{frame:04}/{layer:04}.png")
}

/// Writes a project archive (format v2) to any seekable writer.
pub fn write_kgr<W: Write + Seek>(doc: &Document, writer: W) -> Result<W, FileError> {
    let mut zip = ZipWriter::new(writer);
    let json_opts = SimpleFileOptions::default().compression_method(CompressionMethod::Deflated);
    // Layer PNGs are already compressed.
    let png_opts = SimpleFileOptions::default().compression_method(CompressionMethod::Stored);

    let manifest = Manifest {
        format: KGR_FORMAT_ID.to_owned(),
        format_version: KGR_FORMAT_VERSION,
        app_version: crate::APP_VERSION.to_owned(),
    };
    zip.start_file(MANIFEST, json_opts)?;
    serde_json::to_writer_pretty(&mut zip, &manifest)?;

    let document = DocumentV2 {
        width: doc.width(),
        height: doc.height(),
        active_layer: doc.active_index(),
        active_frame: doc.active_frame_index(),
        layers: doc
            .layers()
            .iter()
            .map(|l| LayerV2 {
                id: l.id.0,
                name: l.name.clone(),
                visible: l.visible,
            })
            .collect(),
        frames: doc
            .frames()
            .iter()
            .enumerate()
            .map(|(fi, f)| FrameV2 {
                id: f.id.0,
                duration_ms: f.duration_ms(),
                cels: (0..doc.layer_count()).map(|li| cel_file(fi, li)).collect(),
            })
            .collect(),
    };
    zip.start_file(DOCUMENT, json_opts)?;
    serde_json::to_writer_pretty(&mut zip, &document)?;

    for (fi, frame) in doc.frames().iter().enumerate() {
        for (li, cel) in frame.cels().iter().enumerate() {
            zip.start_file(cel_file(fi, li), png_opts)?;
            encode_png(cel, &mut zip)?;
        }
    }
    Ok(zip.finish()?)
}

/// Saves a project safely (temporary file + replace).
pub fn save_kgr(doc: &Document, path: &Path) -> Result<(), FileError> {
    write_atomic(path, |w| {
        write_kgr(doc, w)?;
        Ok(())
    })
}

/// Opens a project file.
pub fn load_kgr(path: &Path) -> Result<Document, FileError> {
    check_file_size(path, MAX_KGR_FILE_BYTES)?;
    let file = std::fs::File::open(path)?;
    read_kgr(std::io::BufReader::new(file))
}

/// Reads and validates a project archive of any supported version.
pub fn read_kgr<R: Read + Seek>(reader: R) -> Result<Document, FileError> {
    let mut zip = ZipArchive::new(reader)?;
    if zip.len() > MAX_ENTRIES {
        return Err(FileError::Invalid(format!(
            "the archive has {} entries (at most {MAX_ENTRIES} expected)",
            zip.len()
        )));
    }

    let manifest: Manifest =
        serde_json::from_slice(&read_entry(&mut zip, MANIFEST, MAX_JSON_BYTES)?)?;
    if manifest.format != KGR_FORMAT_ID {
        return Err(FileError::Invalid(format!(
            "unknown format identifier \"{}\"",
            manifest.format
        )));
    }
    // Check the version before touching anything else in the archive.
    match manifest.format_version {
        0 => Err(FileError::Invalid(
            "format version 0 is not valid".to_owned(),
        )),
        1 => {
            let doc_bytes = read_entry(&mut zip, DOCUMENT, MAX_JSON_BYTES)?;
            read_v1(&mut zip, &doc_bytes)
        }
        2 => {
            let doc_bytes = read_entry(&mut zip, DOCUMENT, MAX_JSON_BYTES)?;
            read_v2(&mut zip, &doc_bytes)
        }
        found => Err(FileError::UnsupportedVersion {
            found,
            supported: KGR_FORMAT_VERSION,
        }),
    }
}

/// Checks size, layer/frame counts and the memory budget before decoding any pixels.
fn validate_shape(
    width: u32,
    height: u32,
    layers: usize,
    frames: usize,
    entries: usize,
) -> Result<(), FileError> {
    validate_size(width, height)?;
    if layers == 0 {
        return Err(FileError::Invalid("the project has no layers".to_owned()));
    }
    if frames == 0 {
        return Err(FileError::Invalid("the project has no frames".to_owned()));
    }
    if layers > MAX_LAYERS {
        return Err(FileError::Invalid(format!(
            "the project has {layers} layers (at most {MAX_LAYERS} supported)"
        )));
    }
    if frames > MAX_FRAMES {
        return Err(FileError::Invalid(format!(
            "the project has {frames} frames (at most {MAX_FRAMES} supported)"
        )));
    }
    if layers_bytes(width, height, layers * frames) > MAX_DOCUMENT_BYTES {
        return Err(FileError::TooLarge(
            "the frames and layers would need more memory than Kagero allows".to_owned(),
        ));
    }
    if entries > layers * frames + 16 {
        return Err(FileError::Invalid(
            "the archive contains unexpected extra entries".to_owned(),
        ));
    }
    Ok(())
}

fn read_cel<R: Read + Seek>(
    zip: &mut ZipArchive<R>,
    file: &str,
    width: u32,
    height: u32,
) -> Result<PixelBuffer, FileError> {
    let bytes = read_entry(zip, file, MAX_LAYER_PNG_BYTES)?;
    let pixels = decode_png(&bytes)?;
    if pixels.width() != width || pixels.height() != height {
        return Err(FileError::Invalid(format!(
            "\"{file}\" is {}×{} but the image is {width}×{height}",
            pixels.width(),
            pixels.height(),
        )));
    }
    Ok(pixels)
}

/// Kagero 0.1 projects: one image with layers → one frame.
fn read_v1<R: Read + Seek>(zip: &mut ZipArchive<R>, json: &[u8]) -> Result<Document, FileError> {
    let doc: DocumentV1 = serde_json::from_slice(json)?;
    validate_shape(doc.width, doc.height, doc.layers.len(), 1, zip.len())?;
    if doc.active_layer >= doc.layers.len() {
        return Err(FileError::Invalid(
            "the active layer index is out of range".to_owned(),
        ));
    }
    let mut infos = Vec::with_capacity(doc.layers.len());
    let mut cels = Vec::with_capacity(doc.layers.len());
    for (i, layer) in doc.layers.iter().enumerate() {
        // Only the canonical names written by Kagero are accepted.
        if layer.file != v1_layer_file(i) {
            return Err(FileError::Invalid(format!(
                "unexpected layer file name \"{}\"",
                layer.file
            )));
        }
        let name = sanitize_layer_name(&layer.name).unwrap_or_else(|| format!("Layer {}", i + 1));
        cels.push(read_cel(zip, &layer.file, doc.width, doc.height)?);
        infos.push((name, layer.visible));
    }
    Ok(Document::from_parts(
        doc.width,
        doc.height,
        infos,
        vec![(DEFAULT_FRAME_DURATION_MS, cels)],
        doc.active_layer,
        0,
    )?)
}

/// Kagero 0.2 projects with animation frames.
fn read_v2<R: Read + Seek>(zip: &mut ZipArchive<R>, json: &[u8]) -> Result<Document, FileError> {
    let doc: DocumentV2 = serde_json::from_slice(json)?;
    validate_shape(
        doc.width,
        doc.height,
        doc.layers.len(),
        doc.frames.len(),
        zip.len(),
    )?;
    if doc.active_layer >= doc.layers.len() {
        return Err(FileError::Invalid(
            "the active layer index is out of range".to_owned(),
        ));
    }
    if doc.active_frame >= doc.frames.len() {
        return Err(FileError::Invalid(
            "the active frame index is out of range".to_owned(),
        ));
    }
    let layer_ids: Vec<u64> = doc.layers.iter().map(|l| l.id).collect();
    let frame_ids: Vec<u64> = doc.frames.iter().map(|f| f.id).collect();
    check_ids(&layer_ids, "layer")?;
    check_ids(&frame_ids, "frame")?;

    let infos: Vec<(String, bool)> = doc
        .layers
        .iter()
        .enumerate()
        .map(|(i, l)| {
            let name = sanitize_layer_name(&l.name).unwrap_or_else(|| format!("Layer {}", i + 1));
            (name, l.visible)
        })
        .collect();
    let mut frames = Vec::with_capacity(doc.frames.len());
    for (fi, frame) in doc.frames.iter().enumerate() {
        if frame.cels.len() != doc.layers.len() {
            return Err(FileError::Invalid(format!(
                "frame {} has {} layer images, expected {}",
                fi + 1,
                frame.cels.len(),
                doc.layers.len()
            )));
        }
        let mut cels = Vec::with_capacity(frame.cels.len());
        for (li, file) in frame.cels.iter().enumerate() {
            if *file != cel_file(fi, li) {
                return Err(FileError::Invalid(format!(
                    "unexpected layer file name \"{file}\""
                )));
            }
            cels.push(read_cel(zip, file, doc.width, doc.height)?);
        }
        frames.push((frame.duration_ms, cels));
    }
    let mut document = Document::from_parts(
        doc.width,
        doc.height,
        infos,
        frames,
        doc.active_layer,
        doc.active_frame,
    )?;
    document.restore_ids(&layer_ids, &frame_ids)?;
    Ok(document)
}

/// Ids must be non-zero and unique.
fn check_ids(ids: &[u64], what: &str) -> Result<(), FileError> {
    let mut seen = HashSet::with_capacity(ids.len());
    for &id in ids {
        if id == 0 || !seen.insert(id) {
            return Err(FileError::Invalid(format!(
                "duplicate or invalid {what} id {id}"
            )));
        }
    }
    Ok(())
}

/// Reads one archive entry into memory, refusing entries larger than `limit`.
fn read_entry<R: Read + Seek>(
    zip: &mut ZipArchive<R>,
    name: &str,
    limit: u64,
) -> Result<Vec<u8>, FileError> {
    let entry = match zip.by_name(name) {
        Ok(e) => e,
        Err(zip::result::ZipError::FileNotFound) => {
            return Err(FileError::Invalid(format!("\"{name}\" is missing")));
        }
        Err(e) => return Err(e.into()),
    };
    if entry.size() > limit {
        return Err(FileError::TooLarge(format!("\"{name}\" is too big")));
    }
    let mut buf = Vec::with_capacity(entry.size() as usize);
    // The declared size may lie; never read more than the limit.
    entry.take(limit + 1).read_to_end(&mut buf)?;
    if buf.len() as u64 > limit {
        return Err(FileError::TooLarge(format!("\"{name}\" is too big")));
    }
    Ok(buf)
}
