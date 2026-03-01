use anyhow::anyhow;
use anyhow::{Context, Result};
use log::{info, warn};
use quick_xml::Reader;
use quick_xml::events::{BytesStart, Event};
use std::fs::{File, remove_file};
use std::io::{BufReader, BufWriter, Write};
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

#[derive(Debug, Default)]
struct VdjSong {
    path: String,
    size: Option<String>,
    songlength: Option<String>,
    artist: Option<String>,
    title: Option<String>,
    bpm: Option<String>,
    key: Option<String>,
}

/// Returns true when the given file is a VirtualDJ playlist descriptor.
pub fn is_vdj_playlist(path: &Path) -> bool {
    path.extension()
        .and_then(|ext| ext.to_str())
        .map(|ext| ext.eq_ignore_ascii_case("vdjfolder"))
        .unwrap_or(false)
}

/// Convert every VirtualDJ playlist descriptor under `root` into an .m3u file.
pub fn convert_all(root: &Path) -> Result<()> {
    if !root.exists() {
        return Ok(());
    }

    for entry in WalkDir::new(root).follow_links(false) {
        let entry = match entry {
            Ok(entry) => entry,
            Err(e) => {
                warn!("Skipping entry while scanning for playlists: {}", e);
                continue;
            }
        };
        let path = entry.path();
        if path.is_file() && is_vdj_playlist(path) {
            match convert_and_replace(path) {
                Ok(output) => {
                    info!("Materialized {} -> {}", path.display(), output.display());
                }
                Err(e) => warn!("Failed to convert {}: {}", path.display(), e),
            }
        }
    }

    Ok(())
}

/// Convert the provided .vdjfolder file into an .m3u playlist.
pub fn convert(path: &Path) -> Result<PathBuf> {
    let songs = read_vdj_songs(path)?;
    let output_path = playlist_output_path(path).ok_or_else(|| {
        anyhow!(
            "unable to determine playlist destination for {}",
            path.display()
        )
    })?;

    if let Some(parent) = output_path.parent() {
        std::fs::create_dir_all(parent)?;
    }

    let file = File::create(&output_path)
        .with_context(|| format!("creating playlist {}", output_path.display()))?;
    let mut writer = BufWriter::new(file);

    for song in songs.iter() {
        writeln!(writer, "#EXTVDJ:{}", format_vdj_attributes(song))?;
        writeln!(writer, "{}", song.path)?;
    }

    Ok(output_path)
}

/// Convert and delete the descriptor so only the .m3u remains.
pub fn convert_and_replace(path: &Path) -> Result<PathBuf> {
    let output_path = convert(path)?;
    remove_file(path).with_context(|| format!("removing {}", path.display()))?;
    Ok(output_path)
}

/// Remove the derived .m3u when the descriptor disappears.
pub fn remove(path: &Path) -> Result<()> {
    if let Some(output_path) = playlist_output_path(path) {
        match remove_file(&output_path) {
            Ok(()) => Ok(()),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),
            Err(e) => Err(e).with_context(|| format!("removing {}", output_path.display())),
        }
    } else {
        Ok(())
    }
}

fn read_vdj_songs(path: &Path) -> Result<Vec<VdjSong>> {
    let file = File::open(path).with_context(|| format!("opening {}", path.display()))?;
    let mut reader = Reader::from_reader(BufReader::new(file));
    reader.trim_text(true);
    let mut buf = Vec::new();
    let mut songs: Vec<(usize, VdjSong)> = Vec::new();
    let mut fallback_idx = 0;

    loop {
        match reader.read_event_into(&mut buf) {
            Ok(Event::Empty(ref element)) if element.name().as_ref() == b"song" => {
                let (song, idx) = parse_song_attributes(element)?;
                let final_idx = idx.unwrap_or_else(|| {
                    let value = fallback_idx;
                    fallback_idx += 1;
                    value
                });
                if song.path.is_empty() {
                    buf.clear();
                    continue;
                }
                songs.push((final_idx, song));
            }
            Ok(Event::Eof) => break,
            Ok(_) => {}
            Err(e) => return Err(e).with_context(|| format!("parsing {}", path.display())),
        }
        buf.clear();
    }

    songs.sort_by_key(|(idx, _)| *idx);
    Ok(songs.into_iter().map(|(_, song)| song).collect())
}

fn parse_song_attributes(element: &BytesStart) -> Result<(VdjSong, Option<usize>)> {
    let mut song = VdjSong::default();
    let mut idx: Option<usize> = None;

    for attr in element.attributes() {
        let attr = attr?;
        let value = attr.unescape_value()?.into_owned();
        match attr.key.as_ref() {
            b"path" => song.path = value.clone(),
            b"size" => song.size = normalize_optional_value(value.clone()),
            b"songlength" => song.songlength = normalize_optional_value(value.clone()),
            b"artist" => song.artist = normalize_optional_value(value.clone()),
            b"title" => song.title = normalize_optional_value(value.clone()),
            b"bpm" => song.bpm = normalize_optional_value(value.clone()),
            b"key" => song.key = normalize_optional_value(value.clone()),
            b"idx" => {
                if let Ok(parsed) = value.parse::<usize>() {
                    idx = Some(parsed);
                }
            }
            _ => {}
        }
    }

    Ok((song, idx))
}

fn normalize_optional_value(value: String) -> Option<String> {
    if value.trim().is_empty() {
        None
    } else {
        Some(value)
    }
}

fn playlist_output_path(vdj_file: &Path) -> Option<PathBuf> {
    Some(vdj_file.with_extension("m3u"))
}

fn format_vdj_attributes(song: &VdjSong) -> String {
    let mut fragments = String::new();
    if let Some(size) = &song.size {
        fragments.push_str(&format!("<filesize>{}</filesize>", escape_xml(size)));
    }
    if let Some(artist) = &song.artist {
        fragments.push_str(&format!("<artist>{}</artist>", escape_xml(artist)));
    }
    if let Some(title) = &song.title {
        fragments.push_str(&format!("<title>{}</title>", escape_xml(title)));
    }
    if let Some(length) = &song.songlength {
        fragments.push_str(&format!("<songlength>{}</songlength>", escape_xml(length)));
    }
    if let Some(bpm) = &song.bpm {
        fragments.push_str(&format!("<bpm>{}</bpm>", escape_xml(bpm)));
    }
    if let Some(key) = &song.key {
        fragments.push_str(&format!("<key>{}</key>", escape_xml(key)));
    }
    fragments
}

fn escape_xml(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
}
