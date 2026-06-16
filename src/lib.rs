#![forbid(unsafe_code)]

//! Safe, zero-copy parser for UTAU Sequence Text (`.ust`) files.
//!
//! UST files are INI-like byte streams. Classic UTAU writes Shift_JIS by
//! default, while some files declare another encoding with `Charset=...`.
//! This crate parses structure over bytes first so unknown fields and non-UTF-8
//! lyrics can be preserved without lossy conversion.

use std::borrow::Cow;
use std::fmt;
use std::str;

use encoding_rs::{Encoding, SHIFT_JIS};

/// Parsed UST document.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct Ust<'a> {
    /// All sections in source order, including unknown sections.
    pub blocks: Vec<Block<'a>>,
    /// Parsed version section, when present.
    pub version: Option<VersionSection<'a>>,
    /// Parsed global settings section, when present.
    pub settings: Option<SettingSection>,
    /// Note and plugin-note blocks in source order.
    pub notes: Vec<NoteBlock<'a>>,
    /// Whether a `[#TRACKEND]` marker was found.
    pub track_end: bool,
}

impl<'a> Ust<'a> {
    /// Finds a setting by byte key.
    pub fn setting(&self, key: &[u8]) -> Option<&Field<'a>> {
        self.settings_field(key)
    }

    /// Returns the parsed `[#VERSION]` block, when present.
    pub fn version_block(&self) -> Option<&Block<'a>> {
        let index = self.version.as_ref()?.block_index;
        self.blocks.get(index)
    }

    /// Returns the parsed `[#SETTING]` block, when present.
    pub fn settings_block(&self) -> Option<&Block<'a>> {
        let index = self.settings?.block_index;
        self.blocks.get(index)
    }

    /// Returns the source block for a note object produced by this document.
    pub fn note_block(&self, note: &NoteBlock<'a>) -> Option<&Block<'a>> {
        self.blocks.get(note.block_index)
    }

    /// Returns the first declared charset label from the parsed document.
    pub fn charset(&self) -> Option<&'a [u8]> {
        self.version
            .as_ref()
            .and_then(|version| version.charset)
            .or_else(|| self.setting(b"Charset").map(|field| field.value_trimmed()))
    }

    /// Returns the global tempo, if `[#SETTING]` contains a valid `Tempo`.
    pub fn tempo(&self) -> Option<f64> {
        self.setting(b"Tempo").and_then(Field::parse_f64)
    }
}

/// A UST section.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct Block<'a> {
    pub header: Header<'a>,
    pub line: usize,
    pub items: Vec<Item<'a>>,
}

impl<'a> Block<'a> {
    pub fn field(&self, key: &[u8]) -> Option<&Field<'a>> {
        self.items.iter().find_map(|item| match item {
            Item::Field(field) if field.key.eq_ignore_ascii_case(key) => Some(field),
            _ => None,
        })
    }
}

/// A parsed section header.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub struct Header<'a> {
    pub kind: HeaderKind<'a>,
    pub raw: &'a [u8],
}

/// Known UST section kinds.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum HeaderKind<'a> {
    Version,
    Setting,
    TrackEnd,
    Note(u32),
    Prev,
    Next,
    Insert,
    Delete,
    Other(&'a [u8]),
}

/// A line inside a section.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum Item<'a> {
    Field(Field<'a>),
    Raw(RawLine<'a>),
}

/// `key=value` line.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub struct Field<'a> {
    pub key: &'a [u8],
    pub value: &'a [u8],
    pub line: usize,
}

impl<'a> Field<'a> {
    pub fn value_trimmed(&self) -> &'a [u8] {
        trim_ascii(self.value)
    }

    pub fn parse_i32(&self) -> Option<i32> {
        parse_ascii(self.value_trimmed())
    }

    pub fn parse_u32(&self) -> Option<u32> {
        parse_ascii(self.value_trimmed())
    }

    pub fn parse_f64(&self) -> Option<f64> {
        parse_ascii(self.value_trimmed())
    }

    pub fn parse_bool(&self) -> Option<bool> {
        match self.value_trimmed() {
            b"1" => Some(true),
            b"0" => Some(false),
            value if value.eq_ignore_ascii_case(b"true") => Some(true),
            value if value.eq_ignore_ascii_case(b"false") => Some(false),
            _ => None,
        }
    }
}

/// Non-`key=value` line preserved verbatim.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub struct RawLine<'a> {
    pub bytes: &'a [u8],
    pub line: usize,
}

/// `[#VERSION]` content.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub struct VersionSection<'a> {
    block_index: usize,
    pub version: Option<&'a [u8]>,
    pub charset: Option<&'a [u8]>,
}

/// `[#SETTING]` content.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub struct SettingSection {
    block_index: usize,
}

impl<'a> Ust<'a> {
    fn setting_block(&self) -> Option<&Block<'a>> {
        self.settings_block()
    }
}

impl SettingSection {
    fn field_from_block<'a, 'b>(&self, block: &'b Block<'a>, key: &[u8]) -> Option<&'b Field<'a>> {
        block.field(key)
    }
}

impl<'a> Ust<'a> {
    fn settings_field(&self, key: &[u8]) -> Option<&Field<'a>> {
        let block = self.setting_block()?;
        self.settings?.field_from_block(block, key)
    }
}

/// Note or plugin-note block.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct NoteBlock<'a> {
    block_index: usize,
    pub kind: NoteKind,
    pub fields: NoteFields<'a>,
}

impl<'a> NoteBlock<'a> {
    /// Returns the zero-based block index in the original document.
    pub fn block_index(&self) -> usize {
        self.block_index
    }

    /// Finds a field in this note's source block.
    pub fn field<'b>(&self, key: &[u8], ust: &'b Ust<'a>) -> Option<&'b Field<'a>> {
        ust.note_block(self)?.field(key)
    }
}

/// Note header class.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum NoteKind {
    Index(u32),
    Prev,
    Next,
    Insert,
    Delete,
}

/// Commonly used typed note fields.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
#[non_exhaustive]
pub struct NoteFields<'a> {
    pub length: Option<u32>,
    pub duration: Option<u32>,
    pub delta: Option<i32>,
    pub lyric: Option<&'a [u8]>,
    pub note_num: Option<i32>,
    pub tempo: Option<&'a [u8]>,
    pub velocity: Option<i32>,
    pub intensity: Option<i32>,
    pub modulation: Option<i32>,
    pub flags: Option<&'a [u8]>,
    pub pre_utterance: Option<&'a [u8]>,
    pub voice_overlap: Option<&'a [u8]>,
    pub start_point: Option<&'a [u8]>,
    pub envelope: Option<&'a [u8]>,
    pub pbs: Option<&'a [u8]>,
    pub pbw: Option<&'a [u8]>,
    pub pby: Option<&'a [u8]>,
    pub pbm: Option<&'a [u8]>,
    pub pitches: Option<&'a [u8]>,
    pub vbr: Option<&'a [u8]>,
}

/// Result of decoding a UST byte stream into text.
#[derive(Debug, Clone)]
#[non_exhaustive]
pub struct DecodedText<'a> {
    pub text: Cow<'a, str>,
    pub encoding: &'static Encoding,
    pub had_errors: bool,
}

/// Structural parse error.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct ParseError {
    pub line: usize,
    pub kind: ParseErrorKind,
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum ParseErrorKind {
    ContentBeforeHeader,
    EmptyHeader,
    UnterminatedHeader,
}

impl fmt::Display for ParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "UST parse error on line {}: {}", self.line, self.kind)
    }
}

impl fmt::Display for ParseErrorKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ParseErrorKind::ContentBeforeHeader => write!(f, "content before first [#...] header"),
            ParseErrorKind::EmptyHeader => write!(f, "empty [#] header"),
            ParseErrorKind::UnterminatedHeader => write!(f, "unterminated [#...] header"),
        }
    }
}

impl std::error::Error for ParseError {}

/// Parse a UST byte stream without decoding text values.
pub fn parse_bytes(input: &[u8]) -> Result<Ust<'_>, ParseError> {
    let input = strip_utf8_bom(input);
    let mut blocks = Vec::<Block<'_>>::new();
    let mut current: Option<Block<'_>> = None;

    for (line_number, line) in Lines::new(input) {
        let line = trim_line_right(line);
        if line.is_empty() {
            continue;
        }
        if line.starts_with(b"[#") {
            if !line.ends_with(b"]") {
                return Err(ParseError {
                    line: line_number,
                    kind: ParseErrorKind::UnterminatedHeader,
                });
            }
            if let Some(block) = current.take() {
                blocks.push(block);
            }
            let raw = &line[2..line.len() - 1];
            if raw.is_empty() {
                return Err(ParseError {
                    line: line_number,
                    kind: ParseErrorKind::EmptyHeader,
                });
            }
            current = Some(Block {
                header: Header {
                    kind: classify_header(raw),
                    raw,
                },
                line: line_number,
                items: Vec::new(),
            });
            continue;
        }

        let Some(block) = current.as_mut() else {
            return Err(ParseError {
                line: line_number,
                kind: ParseErrorKind::ContentBeforeHeader,
            });
        };
        block.items.push(parse_item(line, line_number));
    }

    if let Some(block) = current {
        blocks.push(block);
    }

    let mut version = None;
    let mut settings = None;
    let mut notes = Vec::new();
    let mut track_end = false;

    for (index, block) in blocks.iter().enumerate() {
        match block.header.kind {
            HeaderKind::Version => {
                version = Some(parse_version_section(index, block));
            }
            HeaderKind::Setting => {
                settings = Some(SettingSection { block_index: index });
            }
            HeaderKind::TrackEnd => {
                track_end = true;
            }
            HeaderKind::Note(note_index) => notes.push(NoteBlock {
                block_index: index,
                kind: NoteKind::Index(note_index),
                fields: parse_note_fields(block),
            }),
            HeaderKind::Prev => notes.push(NoteBlock {
                block_index: index,
                kind: NoteKind::Prev,
                fields: parse_note_fields(block),
            }),
            HeaderKind::Next => notes.push(NoteBlock {
                block_index: index,
                kind: NoteKind::Next,
                fields: parse_note_fields(block),
            }),
            HeaderKind::Insert => notes.push(NoteBlock {
                block_index: index,
                kind: NoteKind::Insert,
                fields: parse_note_fields(block),
            }),
            HeaderKind::Delete => notes.push(NoteBlock {
                block_index: index,
                kind: NoteKind::Delete,
                fields: parse_note_fields(block),
            }),
            HeaderKind::Other(_) => {}
        }
    }

    let ust = Ust {
        blocks,
        version,
        settings,
        notes,
        track_end,
    };
    Ok(ust)
}

/// Parse a UTF-8 string as UST.
pub fn parse_str(input: &str) -> Result<Ust<'_>, ParseError> {
    parse_bytes(input.as_bytes())
}

/// Detect a declared `Charset=` label by scanning the first ten non-empty lines.
///
/// Classic tools often read the beginning as Shift_JIS-compatible ASCII first,
/// then choose the declared encoding. This function follows that practice.
pub fn detect_charset_label(bytes: &[u8]) -> Option<&[u8]> {
    let mut inspected = 0;
    for (_, line) in Lines::new(strip_utf8_bom(bytes)) {
        let line = trim_line_right(line);
        if line.is_empty() {
            continue;
        }
        if inspected == 10 {
            break;
        }
        inspected += 1;
        let Some(field) = parse_field(line, 0) else {
            continue;
        };
        if field.key.eq_ignore_ascii_case(b"Charset") {
            return Some(field.value_trimmed());
        }
    }
    None
}

/// Decode bytes using a declared charset label, or Shift_JIS when absent.
pub fn decode_text<'a>(bytes: &'a [u8], charset_label: Option<&[u8]>) -> DecodedText<'a> {
    let encoding = charset_label
        .and_then(Encoding::for_label)
        .unwrap_or(SHIFT_JIS);
    let (text, _, had_errors) = encoding.decode(strip_utf8_bom(bytes));
    DecodedText {
        text,
        encoding,
        had_errors,
    }
}

/// Detect charset, then decode bytes.
pub fn decode_text_auto(bytes: &[u8]) -> DecodedText<'_> {
    decode_text(bytes, detect_charset_label(bytes))
}

fn parse_item(line: &[u8], line_number: usize) -> Item<'_> {
    parse_field(line, line_number)
        .map(Item::Field)
        .unwrap_or(Item::Raw(RawLine {
            bytes: line,
            line: line_number,
        }))
}

fn parse_field(line: &[u8], line_number: usize) -> Option<Field<'_>> {
    let split = line.iter().position(|&byte| byte == b'=')?;
    let key = trim_ascii(&line[..split]);
    Some(Field {
        key,
        value: &line[split + 1..],
        line: line_number,
    })
}

fn parse_version_section<'a>(block_index: usize, block: &Block<'a>) -> VersionSection<'a> {
    let mut version = None;
    let mut charset = None;
    for item in &block.items {
        match item {
            Item::Field(field) if field.key.eq_ignore_ascii_case(b"UST Version") => {
                version = Some(field.value_trimmed());
            }
            Item::Field(field) if field.key.eq_ignore_ascii_case(b"Charset") => {
                charset = Some(field.value_trimmed());
            }
            Item::Raw(raw) => {
                if let Some(value) = parse_raw_version(raw.bytes) {
                    version = Some(value);
                }
            }
            _ => {}
        }
    }
    VersionSection {
        block_index,
        version,
        charset,
    }
}

fn parse_raw_version(line: &[u8]) -> Option<&[u8]> {
    const PREFIX: &[u8] = b"UST Version";
    if line.len() < PREFIX.len() || !line[..PREFIX.len()].eq_ignore_ascii_case(PREFIX) {
        return None;
    }
    let rest = trim_ascii(&line[PREFIX.len()..]);
    Some(rest.strip_prefix(b"=").map(trim_ascii).unwrap_or(rest))
}

fn parse_note_fields<'a>(block: &Block<'a>) -> NoteFields<'a> {
    let mut fields = NoteFields::default();
    for item in &block.items {
        let Item::Field(field) = item else {
            continue;
        };
        let value = field.value_trimmed();
        match field.key {
            key if key.eq_ignore_ascii_case(b"Length") => fields.length = field.parse_u32(),
            key if key.eq_ignore_ascii_case(b"Duration") => fields.duration = field.parse_u32(),
            key if key.eq_ignore_ascii_case(b"Delta") => fields.delta = field.parse_i32(),
            key if key.eq_ignore_ascii_case(b"Lyric") => {
                fields.lyric = Some(value.strip_prefix(b"?").unwrap_or(value));
            }
            key if key.eq_ignore_ascii_case(b"NoteNum") => fields.note_num = field.parse_i32(),
            key if key.eq_ignore_ascii_case(b"Tempo") => fields.tempo = Some(value),
            key if key.eq_ignore_ascii_case(b"Velocity") => fields.velocity = field.parse_i32(),
            key if key.eq_ignore_ascii_case(b"Intensity") => fields.intensity = field.parse_i32(),
            key if key.eq_ignore_ascii_case(b"Modulation")
                || key.eq_ignore_ascii_case(b"Moduration") =>
            {
                fields.modulation = field.parse_i32()
            }
            key if key.eq_ignore_ascii_case(b"Flags") => fields.flags = Some(value),
            key if key.eq_ignore_ascii_case(b"PreUtterance") => fields.pre_utterance = Some(value),
            key if key.eq_ignore_ascii_case(b"VoiceOverlap") => fields.voice_overlap = Some(value),
            key if key.eq_ignore_ascii_case(b"StartPoint") => fields.start_point = Some(value),
            key if key.eq_ignore_ascii_case(b"Envelope") => fields.envelope = Some(value),
            key if key.eq_ignore_ascii_case(b"PBS") => fields.pbs = Some(value),
            key if key.eq_ignore_ascii_case(b"PBW") => fields.pbw = Some(value),
            key if key.eq_ignore_ascii_case(b"PBY") => fields.pby = Some(value),
            key if key.eq_ignore_ascii_case(b"PBM") => fields.pbm = Some(value),
            key if key.eq_ignore_ascii_case(b"Pitches")
                || key.eq_ignore_ascii_case(b"Piches")
                || key.eq_ignore_ascii_case(b"PitchBend") =>
            {
                fields.pitches = Some(value)
            }
            key if key.eq_ignore_ascii_case(b"VBR") => fields.vbr = Some(value),
            _ => {}
        }
    }
    fields
}

fn classify_header(raw: &[u8]) -> HeaderKind<'_> {
    match raw {
        value if value.eq_ignore_ascii_case(b"VERSION") => HeaderKind::Version,
        value if value.eq_ignore_ascii_case(b"SETTING") => HeaderKind::Setting,
        value if value.eq_ignore_ascii_case(b"TRACKEND") => HeaderKind::TrackEnd,
        value if value.eq_ignore_ascii_case(b"PREV") => HeaderKind::Prev,
        value if value.eq_ignore_ascii_case(b"NEXT") => HeaderKind::Next,
        value if value.eq_ignore_ascii_case(b"INSERT") => HeaderKind::Insert,
        value if value.eq_ignore_ascii_case(b"DELETE") => HeaderKind::Delete,
        value if value.iter().all(u8::is_ascii_digit) => parse_ascii(value)
            .map(HeaderKind::Note)
            .unwrap_or(HeaderKind::Other(value)),
        value => HeaderKind::Other(value),
    }
}

fn parse_ascii<T: str::FromStr>(bytes: &[u8]) -> Option<T> {
    str::from_utf8(bytes).ok()?.parse().ok()
}

fn trim_line_right(mut bytes: &[u8]) -> &[u8] {
    if bytes.ends_with(b"\r") {
        bytes = &bytes[..bytes.len() - 1];
    }
    bytes
}

fn trim_ascii(mut bytes: &[u8]) -> &[u8] {
    while let Some((first, rest)) = bytes.split_first() {
        if first.is_ascii_whitespace() {
            bytes = rest;
        } else {
            break;
        }
    }
    while let Some((last, rest)) = bytes.split_last() {
        if last.is_ascii_whitespace() {
            bytes = rest;
        } else {
            break;
        }
    }
    bytes
}

fn strip_utf8_bom(bytes: &[u8]) -> &[u8] {
    bytes.strip_prefix(&[0xEF, 0xBB, 0xBF]).unwrap_or(bytes)
}

struct Lines<'a> {
    bytes: &'a [u8],
    offset: usize,
    line: usize,
}

impl<'a> Lines<'a> {
    fn new(bytes: &'a [u8]) -> Self {
        Self {
            bytes,
            offset: 0,
            line: 1,
        }
    }
}

impl<'a> Iterator for Lines<'a> {
    type Item = (usize, &'a [u8]);

    fn next(&mut self) -> Option<Self::Item> {
        if self.offset >= self.bytes.len() {
            return None;
        }
        let start = self.offset;
        let current_line = self.line;
        while self.offset < self.bytes.len() && self.bytes[self.offset] != b'\n' {
            self.offset += 1;
        }
        let end = self.offset;
        if self.offset < self.bytes.len() {
            self.offset += 1;
            self.line += 1;
        }
        Some((current_line, &self.bytes[start..end]))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_ust_12_with_common_fields() {
        let ust = parse_str(
            "[#VERSION]\r\n\
             UST Version1.2\r\n\
             Charset=UTF-8\r\n\
             [#SETTING]\r\n\
             Tempo=120.5\r\n\
             ProjectName=demo\r\n\
             Mode2=True\r\n\
             [#0000]\r\n\
             Length=480\r\n\
             Lyric=?あ\r\n\
             NoteNum=60\r\n\
             PBS=-40;0\r\n\
             PBW=80,120\r\n\
             PBY=0,20\r\n\
             PBM=s,r\r\n\
             [#TRACKEND]\r\n",
        )
        .unwrap();

        assert_eq!(ust.version.unwrap().version, Some(b"1.2".as_slice()));
        assert_eq!(ust.charset(), Some(b"UTF-8".as_slice()));
        assert_eq!(ust.tempo(), Some(120.5));
        assert!(ust.track_end);
        assert_eq!(ust.notes.len(), 1);
        assert_eq!(ust.notes[0].kind, NoteKind::Index(0));
        assert_eq!(ust.notes[0].fields.length, Some(480));
        assert_eq!(ust.notes[0].fields.lyric, Some("あ".as_bytes()));
        assert_eq!(ust.notes[0].fields.note_num, Some(60));
        assert_eq!(ust.notes[0].fields.pbs, Some(b"-40;0".as_slice()));
    }

    #[test]
    fn keeps_unknown_fields_and_plugin_headers() {
        let ust = parse_str(
            "[#SETTING]\n\
             Tempo=100\n\
             [#PREV]\n\
             Length=240\n\
             @preuttr=12.5\n\
             $custom=value\n\
             [#INSERT]\n\
             Length=120\n",
        )
        .unwrap();

        assert_eq!(ust.notes[0].kind, NoteKind::Prev);
        assert_eq!(ust.notes[1].kind, NoteKind::Insert);
        assert_eq!(
            ust.notes[0]
                .field(b"@preuttr", &ust)
                .unwrap()
                .value_trimmed(),
            b"12.5"
        );
        assert_eq!(
            ust.notes[0]
                .field(b"$custom", &ust)
                .unwrap()
                .value_trimmed(),
            b"value"
        );
    }

    #[test]
    fn detects_charset_in_first_lines() {
        let label = detect_charset_label(b"[#VERSION]\nUST Version=1.20\nCharset=UTF-8\n");
        assert_eq!(label, Some(b"UTF-8".as_slice()));
    }

    #[test]
    fn charset_detection_ignores_blank_lines() {
        let label = detect_charset_label(
            b"\n\n\n\n\n\n\n\n\n\n[#VERSION]\nUST Version=1.20\nCharset=UTF-8\n",
        );
        assert_eq!(label, Some(b"UTF-8".as_slice()));
    }

    #[test]
    fn decodes_shift_jis_by_default() {
        let bytes = b"[#SETTING]\nProjectName=\x83e\x83X\x83g\n";
        let decoded = decode_text_auto(bytes);
        assert_eq!(decoded.encoding.name(), "Shift_JIS");
        assert!(!decoded.had_errors);
        assert!(decoded.text.contains("テスト"));
    }

    #[test]
    fn rejects_content_before_header() {
        let err = parse_str("Tempo=120\n[#SETTING]\n").unwrap_err();
        assert_eq!(err.kind, ParseErrorKind::ContentBeforeHeader);
        assert_eq!(err.line, 1);
    }

    #[test]
    fn malformed_headers_return_errors_without_panicking() {
        assert!(parse_bytes(b"").unwrap().blocks.is_empty());

        let err = parse_bytes(b"[#").unwrap_err();
        assert_eq!(err.kind, ParseErrorKind::UnterminatedHeader);

        let err = parse_bytes(b"[#]\n").unwrap_err();
        assert_eq!(err.kind, ParseErrorKind::EmptyHeader);
    }

    #[test]
    fn oversized_numeric_header_is_preserved_as_unknown() {
        let ust = parse_bytes(b"[#999999999999999999999999]\nLength=480\n").unwrap();
        assert!(matches!(ust.blocks[0].header.kind, HeaderKind::Other(_)));
        assert!(ust.notes.is_empty());
    }

    #[test]
    fn invalid_typed_values_become_none() {
        let ust = parse_str(
            "[#SETTING]\n\
             Tempo=not-a-number\n\
             [#0000]\n\
             Length=abc\n\
             NoteNum=oops\n",
        )
        .unwrap();

        assert_eq!(ust.tempo(), None);
        assert_eq!(ust.notes[0].fields.length, None);
        assert_eq!(ust.notes[0].fields.note_num, None);
    }
}
