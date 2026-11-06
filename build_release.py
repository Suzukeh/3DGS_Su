# -*- coding: utf-8 -*-
"""AviUtl2 カタログ用の配布 zip を作る。

  python build_release.py --version v0.2.0

`plugin-src/target/release` にビルド済みバイナリがあればそれを使い、
無ければリポジトリにコミット済みの `Plugin/` のバイナリを使う。
"""
from __future__ import annotations

import argparse
import hashlib
import shutil
import zipfile
from pathlib import Path

ROOT = Path(__file__).resolve().parent
DIST = ROOT / "dist"


def sha256(path: Path) -> str:
    return hashlib.sha256(path.read_bytes()).hexdigest()


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("--version", default="v0.2.0")
    args = parser.parse_args()
    version = args.version

    build = ROOT / "build" / "pkg"
    if build.exists():
        shutil.rmtree(build)
    (build / "Script").mkdir(parents=True)
    (build / "Plugin" / "3DGSConverter_Su").mkdir(parents=True)

    for name in ["3DGS_Su.obj2", "3DGS_Su_atlas.png", "README.md"]:
        shutil.copy2(ROOT / "Script" / name, build / "Script" / name)

    built = ROOT / "plugin-src" / "target" / "release"
    dll = built / "three_dgs_converter_su.dll"
    exe = built / "su3ds.exe"
    if not dll.is_file():
        dll = ROOT / "Plugin" / "3DGSConverter_Su" / "three_dgs_converter_su.aux2"
    if not exe.is_file():
        exe = ROOT / "Plugin" / "3DGSConverter_Su" / "su3ds.exe"

    shutil.copy2(dll, build / "Plugin" / "3DGSConverter_Su" / "three_dgs_converter_su.aux2")
    shutil.copy2(exe, build / "Plugin" / "3DGSConverter_Su" / "su3ds.exe")
    shutil.copy2(ROOT / "README.md", build / "README.md")
    # オフラインで使い方を読めるよう、詳細MDと作例GIFを同梱する。
    docs = ROOT / "docs"
    for source in docs.rglob("*"):
        if source.is_file() and source.suffix.lower() in {".md", ".gif", ".png"}:
            target = build / "docs" / source.relative_to(docs)
            target.parent.mkdir(parents=True, exist_ok=True)
            shutil.copy2(source, target)
    readme = build / "README.md"
    text = readme.read_text(encoding="utf-8-sig")
    text = text.replace("https://github.com/Suzukeh/3DGS_Su/blob/main/docs/", "docs/")
    text = text.replace("https://raw.githubusercontent.com/Suzukeh/3DGS_Su/refs/heads/main/docs/", "docs/")
    readme.write_text(text, encoding="utf-8")
    shutil.copy2(ROOT / "LICENSE", build / "LICENSE")
    shutil.copy2(ROOT / "THIRD_PARTY_LICENSES.md", build / "THIRD_PARTY_LICENSES.md")

    DIST.mkdir(exist_ok=True)
    zip_path = DIST / f"3DGS_Su-{version}.zip"
    with zipfile.ZipFile(zip_path, "w", zipfile.ZIP_DEFLATED) as archive:
        for path in sorted(build.rglob("*")):
            if path.is_file():
                archive.write(path, path.relative_to(build).as_posix())

    print(f"{zip_path} ({zip_path.stat().st_size / 1024:.1f} KiB)")
    for path in sorted(build.rglob("*")):
        if path.is_file():
            print(f"  {path.relative_to(build).as_posix()}  {sha256(path)}")


if __name__ == "__main__":
    main()
