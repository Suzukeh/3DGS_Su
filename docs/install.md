# インストール

[READMEへ戻る](../README.md)

## AviUtl2 カタログから

AviUtl2 カタログで `3DGS_Su` を検索してインストールしてください。

## 手動で入れる場合

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

AviUtl2 を起動すると `3DGS変換` の専用ウィンドウが開き、
カスタムオブジェクトに `3DGS_Su` が追加されます。

初回起動時に「スクリプト・プラグインの追加」ダイアログが出た場合は、
内容を確認して **信頼して使用する** を選んでください。

## うまく読み込まれない場合

- `3DGS_Su.obj2` と `3DGS_Su_atlas.png` は同じフォルダに置く
- プラグインは `data/Plugin/3DGSConverter_Su/` に置く
- AviUtl2 を完全に終了してから起動し直す
- `3DGS変換` ウィンドウを閉じた場合は **表示 → 3DGS変換** で開き直す
