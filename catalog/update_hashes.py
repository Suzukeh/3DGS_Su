# -*- coding: utf-8 -*-
"""カタログ JSON の XXH3_128 をリポジトリ内のファイルから更新する。

  py -3 catalog/update_hashes.py

`xxhash` が必要（開発用）。
"""
from __future__ import annotations

import json
from pathlib import Path

import xxhash

ROOT = Path(__file__).resolve().parents[1]
CATALOG = ROOT / "catalog" / "Suzuke.3DGS_Su.json"

PREFIX = {
    "{scriptsDir}/3DGS_Su/": ROOT / "Script",
    "{pluginsDir}/3DGSConverter_Su/": ROOT / "Plugin" / "3DGSConverter_Su",
}


def path_for(catalog_path: str) -> Path:
    for prefix, base in PREFIX.items():
        if catalog_path.startswith(prefix):
            return base / catalog_path[len(prefix):]
    raise ValueError(f"未対応のパス: {catalog_path}")


def main() -> None:
    entry = json.loads(CATALOG.read_text(encoding="utf-8"))
    versions = [v for v in entry["version"] if v["version"] == entry["latest-version"]]
    if len(versions) != 1:
        raise SystemExit("latest-version に一致する版は1件だけ必要です")
    # 過去版は、その版を配布した当時のハッシュを保持する。
    for version in versions:
        for file in version["file"]:
            path = path_for(file["path"])
            if not path.is_file():
                raise SystemExit(f"ファイルがありません: {path}")
            file["XXH3_128"] = xxhash.xxh3_128_hexdigest(path.read_bytes())
            print(f"{path.relative_to(ROOT)}: {file['XXH3_128']}")
    CATALOG.write_text(
        json.dumps(entry, ensure_ascii=False, indent=2) + "\n", encoding="utf-8"
    )


if __name__ == "__main__":
    main()
