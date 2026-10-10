//! OWNER: markers agent. Contract signatures below must stay.
use anyhow::{anyhow, Context, Result};
use std::path::Path;

#[derive(Clone, Debug, PartialEq)]
pub struct Marker {
    /// Seconds.
    pub time: f64,
    /// "Artist - Title" (or whatever name the file gives).
    pub name: Option<String>,
    /// Scene id to force for the segment starting here.
    pub scene: Option<String>,
}

/// Parse a CUE sheet, Audacity label file or plain text list (detected by content). Sorted by time.
pub fn load(path: &Path) -> Result<Vec<Marker>> {
    let bytes = std::fs::read(path).with_context(|| format!("reading {}", path.display()))?;
    parse(&decode(bytes)).with_context(|| format!("parsing {}", path.display()))
}

/// UTF-8 (BOM stripped), else Latin-1.
/// ponytail: Latin-1 maps 0x80-0x9F to C1 controls, not cp1252 punctuation; add a table if smart quotes matter.
fn decode(bytes: Vec<u8>) -> String {
    let text = String::from_utf8(bytes).unwrap_or_else(|e| e.into_bytes().iter().map(|&b| b as char).collect());
    match text.strip_prefix('\u{feff}') {
        Some(rest) => rest.to_string(),
        None => text,
    }
}

type Lines<'a> = Vec<(usize, &'a str)>;

fn parse(text: &str) -> Result<Vec<Marker>> {
    let lines: Lines = text.lines().enumerate().map(|(i, l)| (i + 1, l)).collect();
    let mut markers = if is_cue(&lines) {
        parse_cue(&lines)?
    } else if is_audacity(&lines) {
        parse_audacity(&lines)?
    } else {
        parse_plain(&lines)?
    };
    markers.sort_by(|a, b| a.time.total_cmp(&b.time));
    Ok(markers)
}

fn is_cue(lines: &Lines) -> bool {
    lines
        .iter()
        .any(|(_, l)| l.split_whitespace().next() == Some("TRACK"))
}

fn is_audacity(lines: &Lines) -> bool {
    lines
        .iter()
        .find(|(_, l)| !l.trim().is_empty() && !l.starts_with('\\'))
        .map_or(false, |(_, l)| {
            let f: Vec<&str> = l.splitn(3, '\t').collect();
            f.len() == 3 && parse_seconds(f[0]).is_some()
        })
}

fn parse_cue(lines: &Lines) -> Result<Vec<Marker>> {
    struct Track {
        line: usize,
        performer: Option<String>,
        title: Option<String>,
        time: Option<f64>,
    }
    let mut global_performer = None;
    let mut tracks: Vec<Track> = Vec::new();
    for &(n, raw) in lines {
        let line = raw.trim();
        let (kw, rest) = line.split_once(char::is_whitespace).unwrap_or((line, ""));
        let rest = rest.trim();
        match kw {
            "TRACK" => tracks.push(Track { line: n, performer: None, title: None, time: None }),
            "PERFORMER" => match tracks.last_mut() {
                Some(t) => t.performer = cue_value(rest),
                None => global_performer = cue_value(rest),
            },
            "TITLE" => {
                if let Some(t) = tracks.last_mut() {
                    t.title = cue_value(rest);
                }
            }
            "INDEX" => {
                if let Some(t) = tracks.last_mut() {
                    let mut it = rest.split_whitespace();
                    if it.next() == Some("01") {
                        let time = it
                            .next()
                            .and_then(parse_cue_index)
                            .ok_or_else(|| anyhow!("line {n}: bad INDEX 01 time"))?;
                        t.time = Some(time);
                    }
                }
            }
            _ => {}
        }
    }
    tracks
        .into_iter()
        .map(|t| {
            let time = t
                .time
                .ok_or_else(|| anyhow!("line {}: TRACK has no INDEX 01", t.line))?;
            let performer = t.performer.or_else(|| global_performer.clone());
            let name = match (performer, t.title) {
                (Some(p), Some(ti)) => Some(format!("{p} - {ti}")),
                (p, ti) => p.or(ti),
            };
            Ok(make(time, &name.unwrap_or_default()))
        })
        .collect()
}

fn parse_audacity(lines: &Lines) -> Result<Vec<Marker>> {
    let mut out = Vec::new();
    for &(n, l) in lines {
        if l.trim().is_empty() || l.starts_with('\\') {
            continue;
        }
        let f: Vec<&str> = l.splitn(3, '\t').collect();
        if f.len() != 3 {
            return Err(anyhow!("line {n}: expected start<TAB>end<TAB>label"));
        }
        let start = parse_seconds(f[0]).ok_or_else(|| anyhow!("line {n}: bad start time"))?;
        let end = parse_seconds(f[1]).ok_or_else(|| anyhow!("line {n}: bad end time"))?;
        if end < start {
            return Err(anyhow!("line {n}: end before start"));
        }
        out.push(make(start, f[2].trim()));
    }
    Ok(out)
}

fn parse_plain(lines: &Lines) -> Result<Vec<Marker>> {
    let mut out = Vec::new();
    for &(n, raw) in lines {
        let l = raw.trim();
        if l.is_empty() || l.starts_with('#') {
            continue;
        }
        let (tok, rest) = l.split_once(char::is_whitespace).unwrap_or((l, ""));
        let time = parse_time(tok).ok_or_else(|| anyhow!("line {n}: bad time '{tok}'"))?;
        out.push(make(time, rest.trim()));
    }
    Ok(out)
}

/// Build a marker, pulling the first `@scene` token out of the name.
fn make(time: f64, name: &str) -> Marker {
    let (scene, name) = split_scene(name);
    Marker { time, name, scene }
}

fn split_scene(name: &str) -> (Option<String>, Option<String>) {
    let mut scene = None;
    let mut kept = Vec::new();
    for w in name.split_whitespace() {
        match w.strip_prefix('@') {
            Some(id) if !id.is_empty() => {
                scene.get_or_insert_with(|| id.to_string());
            }
            _ => kept.push(w),
        }
    }
    let name = (!kept.is_empty()).then(|| kept.join(" "));
    (scene, name)
}

fn cue_value(rest: &str) -> Option<String> {
    let v = rest
        .strip_prefix('"')
        .and_then(|r| r.strip_suffix('"'))
        .unwrap_or(rest)
        .trim();
    (!v.is_empty()).then(|| v.to_string())
}

/// CUE INDEX time: mm:ss:ff at 75 frames per second.
fn parse_cue_index(s: &str) -> Option<f64> {
    let p: Vec<u64> = s.split(':').map(|x| x.parse().ok()).collect::<Option<_>>()?;
    match p.as_slice() {
        &[m, s, f] => Some((m * 60 + s) as f64 + f as f64 / 75.0),
        _ => None,
    }
}

/// Plain seconds, `m:ss(.s)`, or `h:mm:ss(.s)`; optional surrounding `[` `]`.
pub(crate) fn parse_time(tok: &str) -> Option<f64> {
    let tok = tok.strip_prefix('[').unwrap_or(tok);
    let tok = tok.strip_suffix(']').unwrap_or(tok);
    if tok.contains(':') {
        parse_clock(tok)
    } else {
        parse_seconds(tok)
    }
}

fn parse_clock(tok: &str) -> Option<f64> {
    let p: Vec<f64> = tok
        .split(':')
        .map(|x| x.parse::<f64>().ok().and_then(valid))
        .collect::<Option<_>>()?;
    let secs = match p.as_slice() {
        &[m, s] => m * 60.0 + s,
        &[h, m, s] => h * 3600.0 + m * 60.0 + s,
        _ => return None,
    };
    valid(secs)
}

fn parse_seconds(s: &str) -> Option<f64> {
    s.parse::<f64>().ok().and_then(valid)
}

fn valid(x: f64) -> Option<f64> {
    (x.is_finite() && x >= 0.0).then_some(x)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn approx(a: f64, b: f64) -> bool {
        (a - b).abs() < 1e-6
    }

    #[test]
    fn cue_sheet() {
        let cue = r#"PERFORMER "Global"
TITLE "Mix"
FILE "a.mp3" MP3
  TRACK 01 AUDIO
    TITLE "Intro"
    INDEX 01 00:00:00
  TRACK 02 AUDIO
    PERFORMER "Daft Punk"
    TITLE "One More Time @club"
    INDEX 01 01:02:37
  TRACK 03 AUDIO
    TITLE "Last"
    INDEX 01 03:00:00
"#;
        let m = parse(cue).unwrap();
        assert_eq!(m.len(), 3);
        assert!(approx(m[0].time, 0.0));
        assert_eq!(m[0].name.as_deref(), Some("Global - Intro"));
        assert_eq!(m[0].scene, None);
        assert!(approx(m[1].time, 62.0 + 37.0 / 75.0));
        assert_eq!(m[1].name.as_deref(), Some("Daft Punk - One More Time"));
        assert_eq!(m[1].scene.as_deref(), Some("club"));
        assert!(approx(m[2].time, 180.0));
        assert_eq!(m[2].name.as_deref(), Some("Global - Last"));
    }

    #[test]
    fn audacity_labels() {
        // Out of order on purpose (sorting), plus a spectral line and an empty label.
        let txt = "30.0\t30.0\t\n\
                   \\ 0.0 1.0 0.5\n\
                   0.000000\t0.000000\tintro\n\
                   12.5\t20.0\t@chorus Drop\n";
        let m = parse(txt).unwrap();
        assert_eq!(m.len(), 3);
        assert_eq!(m[0].name.as_deref(), Some("intro"));
        assert!(approx(m[1].time, 12.5));
        assert_eq!(m[1].name.as_deref(), Some("Drop"));
        assert_eq!(m[1].scene.as_deref(), Some("chorus"));
        assert!(approx(m[2].time, 30.0));
        assert_eq!(m[2].name, None);
    }

    #[test]
    fn plain_text() {
        let txt = "# comment\n[01:02:03] Artist - Title @blue\n1:30.5 Second\n123.4\n\n00:45 Short\n";
        let m = parse(txt).unwrap();
        let times: Vec<f64> = m.iter().map(|x| x.time).collect();
        assert!(approx(times[0], 45.0));
        assert!(approx(times[1], 90.5));
        assert!(approx(times[2], 123.4));
        assert!(approx(times[3], 3723.0));
        assert_eq!(m[0].name.as_deref(), Some("Short"));
        assert_eq!(m[2].name, None);
        assert_eq!(m[3].name.as_deref(), Some("Artist - Title"));
        assert_eq!(m[3].scene.as_deref(), Some("blue"));
        assert!(parse("oops here").is_err());
    }

    #[test]
    fn bom_and_latin1() {
        let mut utf8 = b"\xEF\xBB\xBF".to_vec();
        utf8.extend(b"0.0\t0.0\tCaf\xC3\xA9\n");
        let m = parse(&decode(utf8)).unwrap();
        assert_eq!(m[0].name.as_deref(), Some("Caf\u{e9}"));
        let m = parse(&decode(b"1:00 Caf\xE9\n".to_vec())).unwrap();
        assert!(approx(m[0].time, 60.0));
        assert_eq!(m[0].name.as_deref(), Some("Caf\u{e9}"));
    }
}
