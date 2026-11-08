# AviUtl2 カタログ登録用メモ

`Suzuke.3DGS_Su.json` は
[AviUtl2 カタログ用データベース](https://github.com/Neosku/aviutl2-catalog-data)
へ登録するときのエントリです。

## 登録前に必要なもの

1. GitHub リポジトリ `Suzukeh/3DGS_Su` を公開する
2. `python build_release.py --version v0.5.1` で `dist/3DGS_Su-v0.5.1.zip` を作る
3. リリース `v0.5.1` を作り、その zip を添付する
4. サムネイル `catalog/images/Suzuke.3DGS_Su_thumbnail.png` を用意する（206x206 PNG）
5. 説明画像 `catalog/images/Suzuke.3DGS_Su_preview.png` を用意する（16:9 PNG）

カタログの画像欄は静止画（サムネイル 206x206、説明画像 16:9）のみです。
ターンテーブル動画は `docs/` に置き、README で参照します。

## データベースへの追加

1. `aviutl2-catalog-data` を fork する
2. `image/Suzuke.3DGS_Su_thumbnail.png` にサムネイル、
   `image/Suzuke.3DGS_Su_preview.png` に説明画像を置く
3. `index.json` に `Suzuke.3DGS_Su.json` の内容を追加する
4. Pull Request を出す

カタログ本体の「パッケージ登録」画面から入力する場合は、
`Suzuke.3DGS_Su.json` の各項目を転記し、画像をアップロードします。

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
