# パッケージ追加: Suzuke.3DGS_Su

## 概要

3DGS（3D Gaussian Splatting）の PLY / SOG を AviUtl2 の 3D 空間に表示する
カスタムオブジェクトと、変換用の汎用プラグインです。

- ID: `Suzuke.3DGS_Su`
- リポジトリ: https://github.com/Suzukeh/3DGS_Su
- リリース: https://github.com/Suzukeh/3DGS_Su/releases/tag/v0.5.0
- ライセンス: MIT

## 同梱物

- `Script/3DGS_Su/3DGS_Su.obj2`
- `Script/3DGS_Su/3DGS_Su_atlas.png`
- `Plugin/3DGSConverter_Su/three_dgs_converter_su.aux2`
- `Plugin/3DGSConverter_Su/su3ds.exe`

## 確認事項

- `installer.source.github.pattern` は `^3DGS_Su-v.*\\.zip$`
- リリース asset `3DGS_Su-v0.5.0.zip` のファイルハッシュを
  `version[].file[].XXH3_128` に設定済み
- `latest-version` は `v0.5.0`
- サムネイル: `image/Suzuke.3DGS_Su_thumbnail.png`
- 説明画像: `image/Suzuke.3DGS_Su_preview.png`

## 補足

パッケージ種別は主成果物が `.obj2` のため `スクリプト` としています。
変換プラグインを同梱しますが、ユーザー操作の中心はカスタムオブジェクトです。
