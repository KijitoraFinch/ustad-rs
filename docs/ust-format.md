# UST Format Notes

This is a working specification for the parser, based on observed UTAU files and widely used open-source implementations:

- OpenUtau classic UST importer/exporter: <https://github.com/openutau/OpenUtau/blob/master/OpenUtau.Core/Classic/Ust.cs> and <https://github.com/openutau/OpenUtau/blob/master/OpenUtau.Core/Classic/UstNote.cs>
- LibreSVIP UST grammar/model: <https://github.com/SoulMelody/LibreSVIP/blob/main/libresvip/plugins/ust/model.py>
- Utsu UST 1.2 writer/plugin writer: <https://github.com/titinko/utsu/blob/master/src/main/java/com/utsusynth/utsu/files/song/Ust12Writer.java>
- UTAU overview: UST means "Utau Sequence Text"; UTAU is the original singing synthesis application that introduced the project format.

## Encoding

Classic UTAU files are commonly Shift_JIS. Some files declare `Charset=...` near the top, either in `[#VERSION]` or `[#SETTING]`. OpenUtau detects `Charset=` by reading the first ten lines as Shift_JIS-compatible ASCII and otherwise falls back to Shift_JIS.

Parser policy:

- parse section structure and keys as bytes first;
- treat keys and headers as ASCII;
- expose `detect_charset_label`, `decode_text`, and `decode_text_auto` for callers that want decoded text;
- do not require lyrics or paths to be valid UTF-8.

## Container Shape

UST is an INI-like line format. Sections start with `[#...]`.

Typical UST 1.2:

```text
[#VERSION]
UST Version1.2
Charset=UTF-8
[#SETTING]
Tempo=120.00
ProjectName=Song
VoiceDir=voicebank
OutFile=output.wav
CacheDir=cache
Tool1=wavtool.exe
Tool2=resampler.exe
Flags=
Mode2=True
[#0000]
Length=480
Lyric=a
NoteNum=60
[#TRACKEND]
```

Common headers:

- `[#VERSION]`: version metadata. The version line can appear as `UST Version1.2`, `UST Version 1.20`, or `UST Version=1.20`.
- `[#SETTING]`: global settings.
- `[#0000]`, `[#0001]`, ...: note blocks. Writers usually pad to four digits, but readers should accept any ASCII decimal number.
- `[#TRACKEND]`: end marker.
- `[#PREV]`, `[#NEXT]`, `[#INSERT]`, `[#DELETE]`: plugin interchange blocks.

Unknown `[#NAME]` sections should be preserved or ignored without crashing.

## Global Settings

Common `[#SETTING]` keys:

- `Tempo`: BPM. Some tools accept comma decimal input but canonical UST uses dot decimal.
- `Tracks`: usually `1`.
- `Project` or `ProjectName`
- `VoiceDir`
- `OutFile`
- `CacheDir`
- `Tool1`, `Tool2`
- `Flags`
- `Mode2`: `True`/`False` or boolean-like values
- `Autoren`
- `MapFirst`
- `TimeSignatures`: observed form `(numerator/denominator/bar_index),...`
- `Charset`

## Note Fields

Minimum useful note:

```text
[#0000]
Length=480
Lyric=a
NoteNum=60
```

Common keys:

- `Length`: note length in ticks for UST 1.x.
- `Lyric`: lyric or alias. Some imported values are prefixed with `?`; OpenUtau strips that prefix.
- `NoteNum`: MIDI note number. `60` is `C4` in common UST tooling.
- `Tempo`: per-note tempo change at the note position.
- `Velocity`
- `Intensity`
- `Modulation`; misspelled `Moduration` exists in the wild and should be accepted.
- `Flags`
- `PreUtterance`
- `VoiceOverlap`
- `StartPoint`
- `Envelope`
- `Label`

UST 2-style timing fields observed by OpenUtau:

- `Delta`
- `Duration`
- `Length`

When `Delta`, `Duration`, and `Length` are all present, note position is relative to the previous note position plus `Delta`, and audible duration uses `Duration`. Otherwise, classic readers advance by previous note end and use `Length`.

## Pitch and Vibrato

Mode2 pitch fields:

- `PBS`: first pitch point. Separators can be `;` or `,`.
- `PBW`: widths between pitch points.
- `PBY`: y offsets for pitch points.
- `PBM`: interpolation modes. Common values are `s`, `r`, `j`, and empty.

Older pitch fields:

- `PBType`
- `PBStart`
- `Pitches`
- typo `Piches`
- `PitchBend`

Vibrato:

- `VBR`: comma-separated values. Common interpretation is length, period, depth, fade-in, fade-out, phase/shift/drift depending on tool.

Envelope:

- Common base form is `p1,p2,p3,v1,v2,v3,v4`.
- Extended forms include `,%,p4,p5,v5` or other trailing points.

## Plugin Fields

Plugin files may include:

- `[#PREV]` and `[#NEXT]` context notes.
- `[#INSERT]` and `[#DELETE]` edit commands.
- `@preuttr`
- `@overlap`
- `@stpoint`
- `@filename`
- `@alias`
- `@cache`

These are important for round-tripping UTAU plugin input and output, so the parser keeps all unknown `key=value` pairs.

## Compatibility Rules

The crate should:

- preserve byte values and source order;
- split `key=value` only at the first `=`;
- trim ASCII whitespace around keys but not decode values during parse;
- accept CRLF and LF;
- accept UTF-8 BOM;
- keep raw non-field lines, especially version lines;
- report structural errors such as content before the first section or malformed headers.
