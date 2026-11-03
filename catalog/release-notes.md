# 3DGS_Su v0.2.0

AviUtl2 で 3DGS（3D Gaussian Splatting）の PLY / SOG をカメラ同期で表示する
カスタムオブジェクトと、変換用の汎用プラグインです。

## 同梱物

- `Script/3DGS_Su.obj2` / `3DGS_Su_atlas.png`
- `Plugin/3DGSConverter_Su/three_dgs_converter_su.aux2`
- `Plugin/3DGSConverter_Su/su3ds.exe`

## 使い方

1. `3DGS_Su` オブジェクトを追加
2. オブジェクトを選択し、編集 → 3DGS変換 → 「3DGSを変換して適用...」
3. `.sog` / `.ply` を選ぶ

## 主な内容

- PLY（binary LE/BE、ASCII）と SOG（zip / 展開フォルダ）に対応
- 変換プラグインを同梱。追加のソフトは不要
- カメラ制御に同期した EWA ビルボード
- 通常合成 / 加算合成

## 制限

- SH（視点依存色）は DC 成分のみ
- 色 512 色、不透明度 8 段階
- 通常合成は点単位の painter's sort

## ライセンス

MIT。作例の Vanadinite（Yann de Couëssin）と Infinity Octo Art MK（Natural Splats）は CC BY 4.0 です。
