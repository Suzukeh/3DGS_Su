# AviUtl2 カタログ登録用メモ

登録先は [AviUtl2 カタログ](https://github.com/Neosku/aviutl2-catalog-data) です。
GUI の「パッケージ登録」から入れるため、ID は **`suzuke.3dgs-su`**（小文字）を使います。

## ファイルの使い分け（形式に注意）

| ファイル | 形 | 用途 |
|---|---|---|
| `Suzuke.3DGS_Su.bundle.json` | **配列（v2 source bundle）** | 登録画面の JSON インポート用（主経路） |
| `Suzuke.3DGS_Su.json` | **単一オブジェクト** `{ ... }` | 内容の確認用 |
| `Suzuke.3DGS_Su.index.json` | **配列** `[ { ... } ]` | `index.json` へ直接追記する場合の予備 |

登録画面の JSON インポートは `index.json` 形式ではなく v2 source bundle を要求します。
画面から登録する場合は `Suzuke.3DGS_Su.bundle.json` を読み込ませてください。
手入力する場合の各欄の値と、ID が小文字限定である点は [GUI入力ガイド](gui-entry.md) にまとめています。

`index.json` を直接編集する経路では、単一オブジェクトをそのまま貼ると
`Invalid input: expected array, received object`（形式が不正）で弾かれます。
その場合は `Suzuke.3DGS_Su.index.json` を使い、既存配列の末尾へ中身を追記してください。

## 登録前に必要なもの

1. GitHub リポジトリ `Suzukeh/3DGS_Su` を公開する
2. `python build_release.py --version v0.5.2` で `dist/3DGS_Su-v0.5.2.zip` を作る
3. リリース `v0.5.2` を作り、その zip を添付する
4. サムネイル `catalog/images/Suzuke.3DGS_Su_thumbnail.png` を用意する（206x206 PNG）
5. 説明画像 `catalog/images/Suzuke.3DGS_Su_preview.png` を用意する（16:9 PNG）

カタログの画像欄は静止画（サムネイル 206x206、説明画像 16:9）のみです。
ターンテーブル動画は `docs/` に置き、README で参照します。

## 登録手順（GUI）

1. AviUtl2 カタログを開き、サイドバーの **パッケージ登録** を選ぶ
2. **JSON インポート** で `catalog/Suzuke.3DGS_Su.bundle.json` を読み込む
3. 読み込まれた ID・各欄を確認する（ID は `suzuke.3dgs-su`）
4. サムネイルと説明画像をアップロードする
5. **テスト** を実行し、インストーラーの取得と手順を確認する
6. 問題なければ送信する

送信はユーザーが行います。手入力する場合の各欄の値は [GUI入力ガイド](gui-entry.md) にあります。

## 予備：データベースへ直接 PR する場合

1. `aviutl2-catalog-data` を fork する
2. `image/` にサムネイルと説明画像を置く
3. `index.json`（配列）へ `Suzuke.3DGS_Su.index.json` の中身を追加する
4. Pull Request を出す

この経路では ID は `Suzuke.3DGS_Su` のままでも通りますが、
GUI 登録と揃えるため `suzuke.3dgs-su` を使います。

## バージョン更新時

`Script/3DGS_Su.obj2`、`Script/3DGS_Su_atlas.png`、
`Plugin/3DGSConverter_Su/*` を更新したら、次でハッシュを更新します。

```bat
py -3 catalog/update_hashes.py
```

`index.json` の `version` に新しいエントリを追加し、
`version[].file[].XXH3_128` を更新してください。

## 補足

- パッケージ種別は、主成果物が `.obj2` のため `スクリプト` にしています。
  変換プラグインを同梱しますが、ユーザー操作の中心はカスタムオブジェクトです。
- `description` は GitHub の README を raw URL で参照しています。
  README を更新すればカタログの詳細も更新されます。
- リリース zip のファイル名は `^3DGS_Su-v.*\.zip$` に一致させてください。
