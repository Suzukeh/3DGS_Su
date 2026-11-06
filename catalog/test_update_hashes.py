"""過去バージョンの検出ハッシュを更新時に書き換えない。"""
import contextlib
import io
import json
import os
from pathlib import Path
import tempfile
import unittest
from unittest import mock

import update_hashes


class HashHistoryTests(unittest.TestCase):
    def test_latest_update_preserves_previous_version(self):
        with tempfile.TemporaryDirectory(dir=os.environ.get('TMPDIR')) as temp:
            root = Path(temp)
            catalog = root / 'catalog.json'
            source = root / 'artifact.bin'
            source.write_bytes(b'new release fixture')
            entry = {
                'latest-version': 'v0.5.0',
                'version': [
                    {'version': 'v0.4.0', 'file': [{'path': 'artifact', 'XXH3_128': 'prior-release-hash'}]},
                    {'version': 'v0.5.0', 'file': [{'path': 'artifact', 'XXH3_128': ''}]},
                ],
            }
            catalog.write_text(json.dumps(entry), encoding='utf-8')
            with mock.patch.object(update_hashes, 'CATALOG', catalog), \
                    mock.patch.object(update_hashes, 'ROOT', root), \
                    mock.patch.object(update_hashes, 'path_for', return_value=source), \
                    contextlib.redirect_stdout(io.StringIO()):
                update_hashes.main()
            updated = json.loads(catalog.read_text(encoding='utf-8'))
            self.assertEqual(updated['version'][0], entry['version'][0])
            self.assertEqual(len(updated['version'][1]['file'][0]['XXH3_128']), 32)


if __name__ == '__main__':
    unittest.main()
