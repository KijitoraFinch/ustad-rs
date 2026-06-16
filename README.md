# ustad

`ustad` is a safe Rust parser for UTAU Sequence Text (`.ust`) files.

The parser is intentionally byte-first:

- it preserves Shift_JIS and other non-UTF-8 values without replacement;
- it keeps unknown fields and unknown sections in source order;
- it exposes typed helpers for common numeric fields such as `Tempo`, `Length`, and `NoteNum`;
- it recognizes normal note blocks and plugin blocks such as `[#PREV]`, `[#NEXT]`, `[#INSERT]`, and `[#DELETE]`.

```rust
let bytes = std::fs::read("song.ust")?;
let decoded = ustad::decode_text_auto(&bytes);
let ust = ustad::parse_str(&decoded.text)?;

println!("tempo: {:?}", ust.tempo());
for note in &ust.notes {
    println!("{:?} {:?}", note.kind, note.fields.note_num);
}
# Ok::<(), Box<dyn std::error::Error>>(())
```

For maximum fidelity, use `parse_bytes` directly and decode only the individual values that your application displays.

## Documentation

- [UST format notes](docs/ust-format.md)
- [Implementation plan](docs/implementation-plan.md)
