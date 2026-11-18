# インストール

[READMEへ戻る](../README.md)

## AviUtl2 カタログから（おすすめ）

AviUtl2 カタログで `3DGS_Su` を検索してインストールしてください。
更新もカタログから行えます。

## 手動で入れる場合

[Releases](https://github.com/Suzukeh/3DGS_Su/releases/latest) から `3DGS_Su-v*.zip` をダウンロードして展開し、
AviUtl2 の `data` フォルダに次のように置きます。

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

AviUtl2 を起動し直すと、カスタムオブジェクトに `3DGS_Su` が、**表示** メニューに **3DGS変換** が追加されます。

初回起動時に「スクリプト・プラグインの追加」ダイアログが出た場合は、
内容を確認して **信頼して使用する** を選んでください。

## うまく動かないとき

- `3DGS_Su.obj2` と `3DGS_Su_atlas.png` が同じフォルダにあるか確認する
- プラグインが `data/Plugin/3DGSConverter_Su/` にあるか確認する
- AviUtl2 を完全に終了してから起動し直す
- **3DGS変換** ウィンドウが見当たらないときは、**表示 → 3DGS変換** で開く

解決しない場合は [Issues](https://github.com/Suzukeh/3DGS_Su/issues) へお知らせください。
