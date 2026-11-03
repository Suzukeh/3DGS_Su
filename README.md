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
- 変換専用UIを同梱。追加のソフトは不要
- 通常合成と加算合成に対応
- 360 度スキャン用のパノラマモード

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

AviUtl2 を起動すると、`3DGS変換` の専用ウィンドウが開きます。
カスタムオブジェクトに `3DGS_Su` が追加されます。
初回起動時にプラグインの信頼確認が出た場合は許可してください。

## 使い方

<img src="https://raw.githubusercontent.com/Suzukeh/3DGS_Su/refs/heads/main/docs/ui/03-converter.png" width="760" alt="3DGS変換の専用UI">

1. カスタムオブジェクトとして `3DGS_Su` を追加する
2. `3DGS_Su` オブジェクトを選択する
3. 専用ウィンドウの **参照...** で `.sog` または `.ply` を選ぶ
4. 必要なら **点数の上限** を変え、**変換して適用** を押す
5. 元ファイルと同じフォルダに `.su3ds` ができ、選択中のオブジェクトに設定される

変換済みの `.su3ds` を使う場合は **既存の .su3ds を適用...** を押します。
直前に変換したものは **直前の結果を適用** で再適用できます。

専用ウィンドウを閉じた場合は、**表示 → 3DGS変換** で開き直せます。
オブジェクトの右クリックメニューからも開けます。

### オブジェクト設定

<img src="https://raw.githubusercontent.com/Suzukeh/3DGS_Su/refs/heads/main/docs/ui/01-settings.png" width="760" alt="3DGS_Su のオブジェクト設定">

カメラ制御の対象レイヤーに置くと、カメラワークに同期して点群が動きます。
カメラ制御の外では XY 平面へ正射影した表示になります。
表示サイズはオブジェクト設定の **サイズ**、描画する点の数は **表示上限** で調整できます。

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
| パノラマ | 360 度スキャン用の球面投影。カメラの向きで見回す |
| 画角 | パノラマ時の水平画角（度） |
| 再読み込み | 0→1 にした瞬間だけ `.su3ds` を読み直す |
| 合成 | 通常（既定）/ 加算 |

## 作例

### Vanadinite

<img src="https://raw.githubusercontent.com/Suzukeh/3DGS_Su/refs/heads/main/docs/vanadinite.gif" width="640" alt="Vanadinite のターンテーブル">

“Vanadinite” by Yann de Couëssin（[SuperSplat](https://superspl.at/user/doudoid)）
/ Source: [scene/663d6a97](https://superspl.at/scene/663d6a97) / CC BY 4.0

### 風景: Infinity Octo Art MK

<img src="https://raw.githubusercontent.com/Suzukeh/3DGS_Su/refs/heads/main/docs/infinity-octo.gif" width="640" alt="Infinity Octo Art MK のパノラマ表示">

“Infinity Octo Art MK ( Wendy Taylor's Octo Art )” by Natural Splats
（[SuperSplat](https://superspl.at/user/naturalsplats)）
/ Source: [scene/94b17a0d](https://superspl.at/scene/94b17a0d) / CC BY 4.0

> 風景例は 3DGS_Su のパノラマモードで表示しています。
> 200 万点のうち 50 万点を使い、水平画角 100 度で見ています。

## .su3ds と SuperSplat の違い

### なぜ .su3ds が必要なのか

PLY / SOG の生データは、位置・回転・スケール・不透明度・色（SH）を
点ごとに持っています。90,000 点の PLY が約 21 MB、SOG でも数 MB あり、
数百万点になると Lua が毎フレーム読み直して描画するには大きすぎます。

`.su3ds` は、このツールで描画するために必要な情報だけを
**1 点 40 バイトの固定長**にしたキャッシュです。

- 位置、描画用の共分散、色、不透明度だけを保持
- 色は 512 色、不透明度は 8 段階に量子化
- 30,000 点で約 1.2 MB。読み込み後は直接参照でき、毎フレームの処理は
  深度順の並べ替えと描画だけになる

つまり `.su3ds` は一般的な 3DGS 形式ではなく、
**AviUtl2 の Lua でリアルタイムに描くための事前変換キャッシュ**です。
元の PLY / SOG を毎回変換する必要はなく、一度作れば使い回せます。

### SuperSplat はなぜリアルタイムでもきれいなのか

SuperSplat は GPU で点群をそのまま描いています。

- 200 万点を間引かず、float のまま GPU に載せる
- 画面空間でガウシアンの広がりを評価する（EWA）
- フレームごとに深度ソートし、視点依存の SH 色も使う
- 高解像度、アンチエイリアス、HDR 的な色処理まで行う

一方、3DGS_Su は AviUtl2 の `drawpoly` で四角形を並べる方式です。
GPU の点群バッファを直接読む API がないため、
**表示点数を減らし、色と不透明度を量子化し、簡易的な深度順で描画**しています。
既定の 30,000 点は軽さを優先した値で、`表示上限` を上げれば
密度は増えますが、SuperSplat と同じ画質にはなりません。
これは実装の不足というより、AviUtl2 側の描画経路の制約による差です。

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
