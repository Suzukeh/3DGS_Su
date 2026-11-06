# aviutl2-eframe 0.43.0 のローカル修正

crates.io の `aviutl2-eframe` 0.43.0 をMITライセンスのまま同梱し、`[patch.crates-io]` でこのコピーを使用します。元の `Cargo.toml.orig` と `LICENSE` を保持しています。ユーザーのCargoレジストリは変更しません。

## 変更点

- 終了要求の検出後は `event_loop.exit()` で停止を要求し、そのイベントをeframeへ転送しません。元の実装は停止要求の直後にもイベントを転送していました。
- AviUtl2が登録済みクライアントHWNDをプラグインのDropより先に破棄した場合も、`IsWindow` で検出してイベント転送を停止します。生成前のHWND=0では初期化を継続します。
- 正常動作中のイベント転送、終了時の既存join処理、ライセンスは維持します。panicの握り潰しやログの抑制はしていません。

停止ゲートは `src/lifecycle.rs`。同じソースを親パッケージの `tests/eframe_lifecycle.rs` からテストします。
