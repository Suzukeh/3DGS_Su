# AviUtl2 カタログ登録用メモ

登録先は [AviUtl2 カタログ](https://github.com/Neosku/aviutl2-catalog-data) です。
ID は **`Suzuke.3DGS_Su`** です。登録はカタログ本体の「パッケージ登録」画面から行います。

## ファイルの使い分け（形式に注意）

| ファイル | 形 | 用途 |
|---|---|---|
| `Suzuke.3DGS_Su.gui-import.json` | **配列** `[ { ... } ]` | 登録画面の **JSON入力** へ貼り付ける（主経路） |
| `Suzuke.3DGS_Su.index.json` | **配列** `[ { ... } ]` | `index.json` への追記（正本） |
| `Suzuke.3DGS_Su.json` | **単一オブジェクト** `{ ... }` | 内容の確認用（そのままでは貼れない） |

単一オブジェクトを貼ると `入力形式が不正です。` で弾かれます。
`id` を持たない JSON を貼ると `上書き対象の要素がありません。` になります。
各欄の値は [GUI入力ガイド](gui-entry.md) にまとめています。

`gui-import.json` は `index.json` 版と同じ内容で、`installer.source` に `"direct": null` だけを加えています。
v0.3.3 は新規パッケージの取り込み時に空の `direct` を下地にして重ねるため、そのままでは
`github` 指定が捨てられ、ダウンロード元が空の「直接URL」になります。`null` で上書きするとこれを避けられます。
送信される `index.json` は画面の入力から作り直されるので、`null` は残りません。

## 登録前に必要なもの

1. GitHub リポジトリ `Suzukeh/3DGS_Su` を公開する
2. `python build_release.py --version v0.5.2` で `dist/3DGS_Su-v0.5.2.zip` を作る
3. リリース `v0.5.2` を作り、その zip を添付する
4. サムネイル `catalog/images/Suzuke.3DGS_Su_thumbnail.webp` を用意する（206x206 アニメーションWebP）
5. 説明画像 `catalog/images/Suzuke.3DGS_Su_preview.webp` を用意する（1280x720 アニメーションWebP）

画像欄は PNG / JPG / GIF / WebP / SVG / BMP を受け付けます（カタログアプリ v0.3.3 の画像選択ダイアログ）。
掲載ガイドの表記は「PNG, JPG」ですが、既存カタログではアニメーション GIF / WebP が使われています。
GIF より色の再現性が高いため、アニメーション WebP（`docs/vanadinite.mp4` から作成、180フレーム・6秒・無限ループ）を使います。
静止画の `*.png` は予備として残しています。

## 登録手順（GUI）

1. AviUtl2 カタログを開き、サイドバーの **パッケージ登録** を選ぶ
2. 画面下部の **JSON入力** を開き、`catalog/Suzuke.3DGS_Su.gui-import.json` の中身を貼り付ける
3. 取り込まれた ID・各欄を確認する（ID は `Suzuke.3DGS_Su`）
4. サムネイルと説明画像をアップロードする
5. **テスト** を実行し、インストーラーの取得と手順を確認する
6. 問題なければ送信する

送信はユーザーが行います。手入力する場合の各欄の値は [GUI入力ガイド](gui-entry.md) にあります。

## 予備：データベースへ直接 PR する場合

1. `aviutl2-catalog-data` を fork する
2. `image/` にサムネイルと説明画像を置く
3. `index.json`（配列）へ `Suzuke.3DGS_Su.index.json` の中身を追加する
4. Pull Request を出す

ID は GUI 登録と同じ `Suzuke.3DGS_Su` です。

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
