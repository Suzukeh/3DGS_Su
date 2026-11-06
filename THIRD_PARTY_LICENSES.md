# Third-party licenses

The release binary includes the following Rust crates. Their licenses are
provided by their respective authors; the full texts are available from the
crate repositories or <https://crates.io/>.

| Crate | Version | License |
|---|---|---|
| `aviutl2` | 0.43.0 | MIT |
| `aviutl2-sys` | 0.43.0 | MIT |
| `aviutl2-eframe` (locally patched) | 0.43.0 | MIT |
| `eframe` / `egui` | 0.35.0 | MIT OR Apache-2.0 |
| `winit` | 0.30.13 | Apache-2.0 |
| `serde` | 1.0.229 | MIT OR Apache-2.0 |
| `aviutl2-alias` | 0.43.0 | MIT |
| `native-dialog` | 0.9.7 | MIT |
| `tracing` | 0.1.44 | MIT |
| `tracing-subscriber` | 0.3.23 | MIT |
| `serde_json` | 1.0.x | MIT OR Apache-2.0 |
| `image` | 0.25.x | MIT OR Apache-2.0 |
| `zip` | 2.x | MIT |
| `windows` | 0.62.x | MIT OR Apache-2.0 |

Transitive dependency licenses are listed by `cargo metadata` and are included
in the source distribution of each crate. The `Cargo.lock` file pins the exact
versions used for the release.

The sample media in `docs/` and `catalog/images/` use the following CC BY 4.0
scenes. The model data itself is not included in this repository.

- **Vanadinite** by Yann de Couëssin
  (https://superspl.at/user/doudoid, scene: https://superspl.at/scene/663d6a97),
  licensed under CC BY 4.0 (https://creativecommons.org/licenses/by/4.0/).
- **Infinity Octo Art MK ( Wendy Taylor's Octo Art )** by Natural Splats
  (https://superspl.at/user/naturalsplats, scene: https://superspl.at/scene/94b17a0d),
  licensed under CC BY 4.0 (https://creativecommons.org/licenses/by/4.0/).

## aviutl2-eframe MIT license

MIT License

Copyright (c) 2025 Nanashi. <sevenc7c.com>

Permission is hereby granted, free of charge, to any person obtaining a copy
of this software and associated documentation files (the "Software"), to deal
in the Software without restriction, including without limitation the rights
to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
copies of the Software, and to permit persons to whom the Software is
furnished to do so, subject to the following conditions:

The above copyright notice and this permission notice shall be included in all
copies or substantial portions of the Software.

THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
SOFTWARE.
