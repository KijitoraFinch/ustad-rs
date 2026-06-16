# ustad

UTAU Sequence Text (`.ust`) ファイルのRustパーサ

## Motivation
RustプロジェクトでUSTファイルファイル全般を扱うことを目指しているらしい

## 特徴

- まあまあ速い
- まあまあちゃんとした抽象化

## Usage

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

最大限ロスレスに扱いたい場合は `parse_bytes` を利用することでテキスト部分を直に扱えます

```rust
let bytes = std::fs::read("song.ust")?;
let ust = ustad::parse_bytes(&bytes)?;

for note in &ust.notes {
    if let Some(lyric) = note.fields.lyric {
        println!("{lyric:?}");
    }
}

# Ok::<(), Box<dyn std::error::Error>>(())
```

## TODOs

実装済み:

- USTのセクション解析
- `[#VERSION]`、`[#SETTING]`、`[#TRACKEND]`
- 番号付きノートブロック
- UTAUプラグイン用ブロック
- `Charset=` 検出と `encoding_rs` によるデコード補助
- 主要ノートフィールドの抽出

今後の予定:

- writer / round-trip出力
- `Delta` / `Duration` のサポート
- pitch / envelope / vibrato の型

## ドキュメント

- [UST形式メモ](docs/ust-format.md)
- [実装方針](docs/implementation-plan.md)

## ライセンス

MIT OR Apache-2.0
