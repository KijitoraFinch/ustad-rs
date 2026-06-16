# ustad

`ustad` は UTAU Sequence Text (`.ust`) ファイルを扱うための、安全でロスレスなRustパーサです。

USTはShift_JISで保存されることが多く、歌詞・音源パス・プラグイン拡張などに非UTF-8の値が入りえます。`ustad` はまずバイト列として構造を解析し、必要なところだけ呼び出し側でデコードできるようにしています。

## 特徴

- `unsafe` 不使用。crate全体で `#![forbid(unsafe_code)]`
- `parse_bytes` によるゼロコピー寄りの低レイヤAST
- Shift_JIS既定、`Charset=` 宣言つきファイルのデコード補助
- 未知のセクション・未知の `key=value` を保持
- `Tempo`、`Length`、`NoteNum` など主要フィールドの型付きアクセサ
- 通常ノート `[#0000]` とプラグイン用 `[#PREV]` / `[#NEXT]` / `[#INSERT]` / `[#DELETE]` に対応
- 壊れた入力はpanicではなく `Result` / `Option` で扱う

## 使い方

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

最大限ロスレスに扱いたい場合は、文字列化せずに `parse_bytes` を使ってください。歌詞やパスなど表示が必要な値だけを後からデコードする設計です。

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

## 現在のスコープ

実装済み:

- USTのセクション解析
- `[#VERSION]`、`[#SETTING]`、`[#TRACKEND]`
- 番号付きノートブロック
- UTAUプラグイン用ブロック
- `Charset=` 検出と `encoding_rs` によるデコード補助
- 主要ノートフィールドの抽出
- 不正ヘッダや不正数値入力の境界テスト

未実装または今後の予定:

- writer / round-trip出力
- 高レベルな所有モデル
- UST 2系の `Delta` / `Duration` を使ったタイミング正規化
- pitch / envelope / vibrato の型付き詳細パーサ
- 実USTコーパスによる回帰テスト
- fuzzingとベンチマーク

## ドキュメント

- [UST形式メモ](docs/ust-format.md)
- [実装方針](docs/implementation-plan.md)

## ライセンス

MIT OR Apache-2.0
