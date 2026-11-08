# 3DGS_Su

3DGS（3D Gaussian Splatting）の PLY / SOG を AviUtl2 の 3D 空間に置き、
カメラ制御に同期して動かすカスタムオブジェクトです。

<img src="https://raw.githubusercontent.com/Suzukeh/3DGS_Su/refs/heads/main/docs/vanadinite.gif" width="720" alt="Vanadinite のターンテーブル">

> 作例: “Vanadinite” by Yann de Couëssin（[SuperSplat](https://superspl.at/user/doudoid)）
> / Source: [scene/663d6a97](https://superspl.at/scene/663d6a97) / CC BY 4.0

## AI利用について

スクリプトの一部はAIによって作成されています。問題や改善については私の責任感に従い誠実な対応をするつもりですが、私の能力を超えて対応不能なケースが生じうることをご理解ください。

## 特長

- PLY / SOG を変換して、そのまま 3D 空間に表示
- AviUtl2 のカメラ制御に追従
- プレビュー／バランス／品質保持の3種類の品質テンプレート
- 品質保持では、追加の間引き・色と不透明度の量子化を避けて表示
- 変換・向き・大きさ・表示設定をAviUtl2内の専用UIで操作。コマンド入力は不要
- 通常合成と加算合成に対応
- 360 度スキャン用のパノラマモード

## 動作環境

- AviUtl2 2.1.x（Windows 10 / 11 64bit）
- 追加のランタイムや外部ツールは不要

## インストール

AviUtl2 カタログで `3DGS_Su` を検索してインストールしてください（登録準備中）。

手動で入れる場合の配置と、初回起動時の注意は
[インストール](https://github.com/Suzukeh/3DGS_Su/blob/main/docs/install.md) にまとめています。

起動すると `3DGS変換` の専用ウィンドウが開きます。
カスタムオブジェクトに `3DGS_Su` が追加されます。

## 使い方

**プレビュー（最大3万点）／バランス（最大15万点）／品質保持（全有効点・浮動小数点・SH）** から選べます。品質保持は処理時間とメモリを多く使います。元のビューアと完全に同じ画素になることを保証する設定ではありません。

1. カスタムオブジェクトとして `3DGS_Su` を追加する
2. `3DGS_Su` オブジェクトを選択する
3. 専用ウィンドウの **ファイル参照...** で `.sog` または `.ply` を選ぶ
4. **品質テンプレート** と必要な設定を選び、**変換して適用** を押す
5. 指定した出力先に `.su3ds` ができ、選択していたオブジェクトに設定される

変換済みの `.su3ds` は **既存の .su3ds を適用...** から使えます。
専用ウィンドウは **表示 → 3DGS変換** で開閉します。
詳細設定は見出しをクリックして開きます。

UIの詳しい操作、パラメータ、対応形式は
[使い方](https://github.com/Suzukeh/3DGS_Su/blob/main/docs/usage.md) を参照してください。

## 作例

Vanadinite のターンテーブルと、360度スキャンの Infinity Octo Art MK のパノラマ表示を掲載しています。GIFは手軽に動きを見るための作例です。

[動き・パノラマの作例を見る](https://github.com/Suzukeh/3DGS_Su/blob/main/docs/samples.md)

## 品質テンプレートの比較

同じVanadiniteを、同じカメラ・表示設定で3DGS_Suから描画しています。

<img src="https://raw.githubusercontent.com/Suzukeh/3DGS_Su/refs/heads/main/docs/quality-presets/overview.png" width="960" alt="プレビュー3万点、バランス15万点、品質保持936,246点の同一視点比較">

> “Vanadinite” by Yann de Couëssin / Source: [scene/663d6a97](https://superspl.at/scene/663d6a97) / CC BY 4.0

品質を見るために、各テンプレートで1周する **1280×720・30fpsのMP4** を用意しています。GIFではなく、RGBロスレス版で色と動きを比較できます。

[1周する比較MP4・細部の拡大・設定と容量の比較](https://github.com/Suzukeh/3DGS_Su/blob/main/docs/quality-presets.md)

## ドキュメント

- [インストール](https://github.com/Suzukeh/3DGS_Su/blob/main/docs/install.md)
- [使い方・パラメータ・対応形式](https://github.com/Suzukeh/3DGS_Su/blob/main/docs/usage.md)
- [動き・パノラマの作例](https://github.com/Suzukeh/3DGS_Su/blob/main/docs/samples.md)
- [品質テンプレート・比較作例](https://github.com/Suzukeh/3DGS_Su/blob/main/docs/quality-presets.md)
- [.su3ds とは](https://github.com/Suzukeh/3DGS_Su/blob/main/docs/su3ds.md)
- [注意・制限](https://github.com/Suzukeh/3DGS_Su/blob/main/docs/notes.md)


## ライセンス

このリポジトリのコードは MIT License です（[LICENSE](LICENSE)）。
同梱ライブラリのライセンスは [THIRD_PARTY_LICENSES.md](THIRD_PARTY_LICENSES.md) を参照してください。

作例の **Vanadinite** と **Infinity Octo Art MK** は CC BY 4.0 の素材です。
素材そのものはこのリポジトリに含めていません。
