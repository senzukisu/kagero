//! The `.kgr` project format: a ZIP archive with `manifest.json`, `document.json`
//! and one PNG per layer. See `docs/PROJECT_FORMAT.md`.
//!
//! Entries are read by exact name into memory; nothing is ever extracted to disk, so
//! paths stored inside an archive cannot point anywhere on the file system.

use std::io::{Read, Seek, Write};
use std::path::Path;

use serde::{Deserialize, Serialize};
use zip::write::SimpleFileOptions;
use zip::{CompressionMethod, ZipArchive, ZipWriter};

use super::png::{decode_png, encode_png};
use super::{check_file_size, write_atomic, FileError, MAX_KGR_FILE_BYTES};
use crate::core::{
    layers_bytes, sanitize_layer_name, validate_size, Document, MAX_CANVAS_SIZE,
    MAX_DOCUMENT_BYTES, MAX_LAYERS,
};

pub const KGR_EXTENSION: &str = "kgr";
/// Identifier stored in the manifest.
pub const KGR_FORMAT_ID: &str = "kagero-project";
/// Current (and highest readable) format version.
pub const KGR_FORMAT_VERSION: u32 = 1;

const MANIFEST: &str = "manifest.json";
const DOCUMENT: &str = "document.json";
const MAX_JSON_BYTES: u64 = 1024 * 1024;
const MAX_ENTRIES: usize = MAX_LAYERS + 16;
/// Largest stored layer PNG (raw RGBA at max size plus generous PNG overhead).
const MAX_LAYER_PNG_BYTES: u64 = (MAX_CANVAS_SIZE as u64).pow(2) * 4 + 4 * 1024 * 1024;

#[derive(Debug, Serialize, Deserialize)]
struct Manifest {
    format: String,
    format_version: u32,
    #[serde(default)]
    app_version: String,
}

#[derive(Debug, Serialize, Deserialize)]
struct DocumentJson {
    width: u32,
    height: u32,
    #[serde(default)]
    active_layer: usize,
    /// Bottom to top.
    layers: Vec<LayerJson>,
}

#[derive(Debug, Serialize, Deserialize)]
struct LayerJson {
    name: String,
    visible: bool,
    file: String,
}

fn layer_file_name(index: usize) -> String {
    format!("layers/{index:04}.png")
}

/// Writes a project archive to any seekable writer.
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

    let document = DocumentJson {
        width: doc.width(),
        height: doc.height(),
        active_layer: doc.active_index(),
        layers: doc
            .layers()
            .iter()
            .enumerate()
            .map(|(i, l)| LayerJson {
                name: l.name.clone(),
                visible: l.visible,
                file: layer_file_name(i),
            })
            .collect(),
    };
    zip.start_file(DOCUMENT, json_opts)?;
    serde_json::to_writer_pretty(&mut zip, &document)?;

    for (i, layer) in doc.layers().iter().enumerate() {
        zip.start_file(layer_file_name(i), png_opts)?;
        encode_png(&layer.pixels, &mut zip)?;
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

/// Reads and validates a project archive.
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
    if manifest.format_version == 0 {
        return Err(FileError::Invalid(
            "format version 0 is not valid".to_owned(),
        ));
    }
    if manifest.format_version > KGR_FORMAT_VERSION {
        return Err(FileError::UnsupportedVersion {
            found: manifest.format_version,
            supported: KGR_FORMAT_VERSION,
        });
    }

    let doc: DocumentJson =
        serde_json::from_slice(&read_entry(&mut zip, DOCUMENT, MAX_JSON_BYTES)?)?;
    validate_size(doc.width, doc.height)?;
    if doc.layers.is_empty() {
        return Err(FileError::Invalid("the project has no layers".to_owned()));
    }
    if doc.layers.len() > MAX_LAYERS {
        return Err(FileError::Invalid(format!(
            "the project has {} layers (at most {MAX_LAYERS} supported)",
            doc.layers.len()
        )));
    }
    if layers_bytes(doc.width, doc.height, doc.layers.len()) > MAX_DOCUMENT_BYTES {
        return Err(FileError::TooLarge(
            "the layers would need more memory than Kagero allows".to_owned(),
        ));
    }
    if doc.active_layer >= doc.layers.len() {
        return Err(FileError::Invalid(
            "the active layer index is out of range".to_owned(),
        ));
    }

    let mut layers = Vec::with_capacity(doc.layers.len());
    for (i, layer) in doc.layers.iter().enumerate() {
        // Only the canonical names written by Kagero are accepted.
        if layer.file != layer_file_name(i) {
            return Err(FileError::Invalid(format!(
                "unexpected layer file name \"{}\"",
                layer.file
            )));
        }
        let name = sanitize_layer_name(&layer.name).unwrap_or_else(|| format!("Layer {}", i + 1));
        let bytes = read_entry(&mut zip, &layer.file, MAX_LAYER_PNG_BYTES)?;
        let pixels = decode_png(&bytes)?;
        if pixels.width() != doc.width || pixels.height() != doc.height {
            return Err(FileError::Invalid(format!(
                "layer \"{name}\" is {}×{} but the image is {}×{}",
                pixels.width(),
                pixels.height(),
                doc.width,
                doc.height
            )));
        }
        layers.push((name, layer.visible, pixels));
    }
    Ok(Document::from_layers(
        doc.width,
        doc.height,
        layers,
        doc.active_layer,
    )?)
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
