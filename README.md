# 3DGS_Su

3DGS（3D Gaussian Splatting）の PLY / SOG を AviUtl2 の 3D 空間に置き、
カメラ制御に同期して動かすカスタムオブジェクトです。

<img src="https://raw.githubusercontent.com/Suzukeh/3DGS_Su/refs/heads/main/docs/vanadinite.gif" width="720" alt="Vanadinite のターンテーブル">

> 作例: “Vanadinite” by Yann de Couëssin（[SuperSplat](https://superspl.at/user/doudoid)）
> / Source: [scene/663d6a97](https://superspl.at/scene/663d6a97) / CC BY 4.0

## 特長

- PLY / SOG を変換して、そのまま 3D 空間に表示
- AviUtl2 のカメラ制御に追従
- 3DGS の楕円形の広がりを保ったまま描画
- 変換プラグインを同梱。追加のソフトは不要
- 通常合成と加算合成に対応

## インストール

### AviUtl2 カタログから

AviUtl2 カタログで `3DGS_Su` を検索してインストールしてください（登録準備中）。

### 手動

配布 zip を展開し、次のように置きます。

```text
data/
  Script/
    3DGS_Su/
      3DGS_Su.obj2
      3DGS_Su_atlas.png
      README.md
  Plugin/
    3DGSConverter_Su/
      three_dgs_converter_su.aux2
      su3ds.exe
```

AviUtl2 を起動し直すと、カスタムオブジェクトに `3DGS_Su`、
編集メニューに「3DGS変換」が追加されます。
初回起動時にプラグインの信頼確認が出た場合は許可してください。

## 使い方

1. カスタムオブジェクトとして `3DGS_Su` を追加する
2. オブジェクトを選択した状態で **編集 → 3DGS変換 → 「3DGSを変換して適用...」**
3. `.sog` または `.ply` を選ぶ
4. 元ファイルと同じフォルダに `.su3ds` ができ、選択中のオブジェクトに設定される

カメラ制御の対象レイヤーに置くと、カメラワークに同期して点群が動きます。
カメラ制御の外では XY 平面へ正射影した表示になります。

### メニュー

| 項目 | 動作 |
|---|---|
| 3DGSを変換して適用... | `.sog` / `.ply` を変換し、選択オブジェクトに設定 |
| 既存の .su3ds を適用... | 変換済みファイルを設定 |
| 直前の変換結果を適用 | 直前に変換したファイルを再適用 |

## 対応形式

- **PLY**: binary little endian / big endian / ASCII
  - `x y z`, `f_dc_0..2`（なければ `red green blue`）
  - `opacity`, `scale_0..2`, `rot_0..3`
- **SOG**: PlayCanvas / SuperSplat の zip、または展開フォルダ
  - `meta.json`、WebP 5 枚（means L/U、scales、quats、sh0）

変換時に最大辺が 400 単位になるよう正規化します。
実寸を保つ場合は、変換オプションで `--fit 0` を指定してください。

## パラメータ

| 項目 | 内容 |
|---|---|
| 3DGSファイル | 変換済み `.su3ds` |
| サイズ | 点の大きさの倍率。位置は変えない |
| 不透明度 | 全体の不透明度 |
| 手前クリップ | カメラからの距離がこれ未満の点を捨てる |
| 表示上限 | 読み込んだ点のうち先頭から何点描くか |
| 奥から描く | 通常合成のとき、奥から手前へ並べ替える |
| 楕円 | オフにすると等方の丸い点になる |
| 再読み込み | 0→1 にした瞬間だけ `.su3ds` を読み直す |
| 合成 | 通常（既定）/ 加算 |

## 作例

### Vanadinite

<img src="https://raw.githubusercontent.com/Suzukeh/3DGS_Su/refs/heads/main/docs/vanadinite.gif" width="640" alt="Vanadinite のターンテーブル">

“Vanadinite” by Yann de Couëssin（[SuperSplat](https://superspl.at/user/doudoid)）
/ Source: [scene/663d6a97](https://superspl.at/scene/663d6a97) / CC BY 4.0

### 風景: Infinity Octo Art MK

<img src="https://raw.githubusercontent.com/Suzukeh/3DGS_Su/refs/heads/main/docs/infinity-octo.gif" width="640" alt="Infinity Octo Art MK のターンテーブル">

“Infinity Octo Art MK ( Wendy Taylor's Octo Art )” by Natural Splats
（[SuperSplat](https://superspl.at/user/naturalsplats)）
/ Source: [scene/94b17a0d](https://superspl.at/scene/94b17a0d) / CC BY 4.0

## 注意

- **SH（視点依存の色）は使いません。** PLY / SOG の DC 成分のみです。
- 色は 512 色、不透明度は 8 段階です。
- 表示できる点数の目安はおよそ 3 万点です。200 万点級の風景スキャンは、間引くと細部が失われます。
- 通常合成は点単位の painter's sort です。点が交差する場面では完全な前後関係になりません。
- オブジェクト自体の回転・拡大は点群全体に掛かります。

## ライセンス

このリポジトリのコードは MIT License です（`LICENSE`）。
同梱ライブラリのライセンスは `THIRD_PARTY_LICENSES.md` を参照してください。

作例の **Vanadinite** と **Infinity Octo Art MK** は CC BY 4.0 の素材です。
素材そのものはこのリポジトリに含めていません。

## ご利用にあたって

スクリプトの一部はAIによって作成されています。問題や改善については私の責任感に従い誠実な対応をするつもりですが、私の能力を超えて対応不能なケースが生じうることをご理解ください。
