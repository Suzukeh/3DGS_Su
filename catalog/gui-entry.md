# カタログ登録画面（GUI）入力ガイド

AviUtl2 カタログ本体の **サイドバー → パッケージ登録** から入力する場合の値です。
`catalog/Suzuke.3DGS_Su.json` は `index.json` 用なので、この画面には**そのまま貼れません**。

## なぜ「入力形式が不正です」になるか

登録画面の **JSON インポート**は、`index.json` 形式ではなく
**v2 source bundle**（`meta` / `content` / `install` / `versions` の4ブロック）を要求します。
`index.json` 形式のファイルを読み込ませると、次のどちらかで弾かれます。

| 操作 | エラー |
|---|---|
| JSON インポートに `Suzuke.3DGS_Su.json` を読ませる | `入力形式が不正です。` |
| ID 欄に `Suzuke.3DGS_Su` を入れる | `ID は英数字・ドット・アンダーバー・ハイフンのみ使用できます` |

ID の検証は `^[a-z0-9]+(-[a-z0-9]+)*\.[a-z0-9]+(-[a-z0-9]+)*$` で、
**小文字の英数字とハイフンだけ**が通ります。大文字とアンダースコアは通りません。

## インポート用ファイル

手入力を省きたい場合は `catalog/Suzuke.3DGS_Su.bundle.json` を
登録画面の JSON インポートに読み込ませてください。
`meta` / `content` / `install` / `versions` を揃えた v2 形式で、配列でも単一オブジェクトでも通ります。
読み込み後に ID と各欄を確認し、テストを実行してから送信してください。

## ID

**`suzuke.3dgs-su`** を使います。

| 値 | 判定 |
|---|---|
| `suzuke.3dgs-su` | OK（採用） |
| `Suzuke.3DGS_Su` | NG（大文字・アンダースコア） |
| `suzuke.3dgs_su` | NG（アンダースコア） |

`index.json` 形式の `Suzuke.3DGS_Su.json` は参考用で、GUI 登録では使いません。

## 入力値

### 一般情報

| 欄 | 値 |
|---|---|
| ID | `suzuke.3dgs-su` |
| パッケージ名 | `3DGS_Su` |
| 作者名 | `Suzuke` |
| オリジナル作者名 | （空） |
| 種類 | `スクリプト` |
| パッケージのサイト | `https://github.com/Suzukeh/3DGS_Su` |
| ニコニ・コモンズID | （空） |
| 依存パッケージ | （空） |
| タグ | `3D`, `3DGS`, `点群`, `ガウシアンスプラッティング`, `カメラ制御` |
| 概要 | `3DGSのPLY/SOGを3D空間にカメラ同期で表示`（26文字、35文字以内） |

### 詳細

**外部MDリンク** を選び、次を入力します。

```
https://raw.githubusercontent.com/Suzukeh/3DGS_Su/main/README.md
```

`https://` で始まる必要があります。アプリ内入力に切り替える場合は README の本文を貼ってください。

### ライセンス

| 欄 | 値 |
|---|---|
| 種類 | `MIT` |
| テンプレートを使用する | オン |
| 著作権年 | `2026` |
| 著作権者 | `Suzuke` |

標準ライセンスで著作権者を入れると、ライセンス本文は空のままにします（両方入れると弾かれます）。

### 画像

| 欄 | ファイル | 条件 |
|---|---|---|
| サムネイル画像 | `catalog/images/Suzuke.3DGS_Su_thumbnail.png` | 206×206 PNG |
| 説明画像 | `catalog/images/Suzuke.3DGS_Su_preview.png` | 16:9 PNG |

### インストーラー

**ダウンロード元: GitHub Release**

| 欄 | 値 |
|---|---|
| GitHub アカウント名 | `Suzukeh` |
| レポジトリ名 | `3DGS_Su` |
| ダウンロードファイル名（正規表現） | `^3DGS_Su-v.*\.zip$` |

**インストール手順**（この順に追加）

| # | action | from | to |
|---|---|---|---|
| 1 | `download` | — | — |
| 2 | `extract` | — | — |
| 3 | `copy` | `{tmp}/Script/3DGS_Su.obj2` | `{scriptsDir}/3DGS_Su` |
| 4 | `copy` | `{tmp}/Script/3DGS_Su_atlas.png` | `{scriptsDir}/3DGS_Su` |
| 5 | `copy` | `{tmp}/Script/README.md` | `{scriptsDir}/3DGS_Su` |
| 6 | `copy` | `{tmp}/Plugin/3DGSConverter_Su/three_dgs_converter_su.aux2` | `{pluginsDir}/3DGSConverter_Su` |
| 7 | `copy` | `{tmp}/Plugin/3DGSConverter_Su/su3ds.exe` | `{pluginsDir}/3DGSConverter_Su` |

**アンインストール手順**（この順に追加、すべて `delete`）

| # | path |
|---|---|
| 1 | `{scriptsDir}/3DGS_Su/3DGS_Su.obj2` |
| 2 | `{scriptsDir}/3DGS_Su/3DGS_Su_atlas.png` |
| 3 | `{scriptsDir}/3DGS_Su/README.md` |
| 4 | `{pluginsDir}/3DGSConverter_Su/three_dgs_converter_su.aux2` |
| 5 | `{pluginsDir}/3DGSConverter_Su/su3ds.exe` |

### バージョン

| 欄 | 値 |
|---|---|
| バージョン | `v0.5.2` |
| リリース日 | `2026-11-14` |

ファイルと XXH3_128（`catalog/update_hashes.py` の出力と同じ値）:

| パス | XXH3_128 |
|---|---|
| `{scriptsDir}/3DGS_Su/3DGS_Su.obj2` | `519db53d5955fbad4ffb6e8da71b73d2` |
| `{scriptsDir}/3DGS_Su/3DGS_Su_atlas.png` | `2a0e792d9e6819be2c11d14c76b8e29f` |
| `{pluginsDir}/3DGSConverter_Su/three_dgs_converter_su.aux2` | `71b82781364b392bdc8ec18cba1775bb` |
| `{pluginsDir}/3DGSConverter_Su/su3ds.exe` | `43562f866be38a579388e5a465846715` |

ハッシュは32文字ちょうどでないと弾かれます。

## 送信前の確認

登録画面の **テスト** を押すと、インストーラーの取得と手順の検証が走ります。
リリース `v0.5.2` が公開済みで、ZIP が `^3DGS_Su-v.*\.zip$` に一致することが前提です。

## 参考

- 掲載ガイド: <https://github.com/Neosku/aviutl2-catalog-data/blob/main/register-package.md>
- 登録内容の修正も同じ画面から行えます。
