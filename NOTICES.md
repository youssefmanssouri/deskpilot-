# Third-Party Software Notices and Licenses

This document contains licensing notices and attribution information for third-party software components included in DeskPilot application releases, embedded web assets, and Windows installation packages.

> **Notice & Scope Disclaimer:**  
> DeskPilot is an open-source project provided under the [MIT License](LICENSE). This document is compiled for informational and attribution purposes based on source-level dependency analysis and target build inspection. It does not constitute formal legal certification, legal advice, or an absolute guarantee of compiler-level object code presence for every transitive library.

---

## 1. DeskPilot Core Application

DeskPilot is licensed under the MIT License:

```text
MIT License

Copyright (c) 2026 DeskPilot Contributors

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
```

---

## 2. Embedded Frontend Libraries (Bundled in Application Executable)

The following third-party JavaScript libraries are imported by the frontend presentation source code and bundled directly into the application distribution binary via the Tauri asset packaging pipeline (`tauri-codegen-assets`):

### A. React & React DOM
- **Package:** `react` and `react-dom`
- **Version:** `19.3.1`
- **Declared License:** `MIT`
- **Copyright:** Copyright (c) Meta Platforms, Inc. and affiliates.
- **License Text:**

```text
MIT License

Copyright (c) Meta Platforms, Inc. and affiliates.

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
```

### B. Tauri API Client
- **Package:** `@tauri-apps/api`
- **Version:** `2.12.4`
- **Declared License:** `Apache-2.0 OR MIT`
- **Copyright:** Copyright (c) 2017 - Present Tauri Apps Contributors
- **License Text (under MIT option):**

```text
MIT License

Copyright (c) 2017 - Present Tauri Apps Contributors

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
```
*(For the alternative Apache 2.0 terms, see Section 3 below).*

---

## 3. Windows Native Runtime Dependencies (Rust Crates)

This section contains the complete **209-package conservative Windows-target normal dependency inventory** resolved for `x86_64-pc-windows-msvc` from `Cargo.lock` and Cargo metadata.

> **Technical Note on Compiler Inlining & LTO:**  
> DeskPilot's release profile enables link-time optimization (`lto = true`), single codegen units (`codegen-units = 1`), and binary symbol stripping (`strip = true`). Under these optimizations, small utility functions may be inlined or stripped by dead-code elimination if uncalled. This inventory conservatively documents the entire resolved Windows runtime dependency graph to ensure comprehensive attribution.

### Package Inventory Table (209 Packages)


#### License: `MIT OR Apache-2.0` (104 packages)

| Package | Version | Type | Notes |
|---|---|---|---|
| `anyhow` | `1.0.104` | Transitive | Unverified (LTO Inlined / Stripped) |
| `base64` | `0.23.1` | Transitive | Unverified (LTO Inlined / Stripped) |
| `bitflags` | `2.13.2` | Transitive | Unverified (LTO Inlined / Stripped) |
| `block-buffer` | `0.10.4` | Transitive | Unverified (LTO Inlined / Stripped) |
| `camino` | `1.2.6` | Transitive | Unverified (LTO Inlined / Stripped) |
| `cargo-platform` | `0.1.9` | Transitive | Unverified (LTO Inlined / Stripped) |
| `cfg-if` | `1.0.5` | Transitive | Unverified (LTO Inlined / Stripped) |
| `cookie` | `0.18.2` | Transitive | Unverified (LTO Inlined / Stripped) |
| `cpufeatures` | `0.2.17` | Transitive | Unverified (LTO Inlined / Stripped) |
| `crc32fast` | `1.5.3` | Transitive | Unverified (LTO Inlined / Stripped) |
| `crossbeam-channel` | `0.5.17` | Transitive | Unverified (LTO Inlined / Stripped) |
| `crossbeam-utils` | `0.8.23` | Transitive | Unverified (LTO Inlined / Stripped) |
| `crypto-common` | `0.1.7` | Transitive | Unverified (LTO Inlined / Stripped) |
| `deranged` | `0.5.9` | Transitive | Unverified (LTO Inlined / Stripped) |
| `digest` | `0.10.7` | Transitive | Unverified (LTO Inlined / Stripped) |
| `dirs` | `7.0.0` | Transitive | Unverified (LTO Inlined / Stripped) |
| `dirs-sys` | `0.5.0` | Transitive | Unverified (LTO Inlined / Stripped) |
| `displaydoc` | `0.2.7` | Transitive | Unverified (LTO Inlined / Stripped) |
| `dtoa` | `1.0.11` | Transitive | Unverified (LTO Inlined / Stripped) |
| `dyn-clone` | `1.0.20` | Transitive | Unverified (LTO Inlined / Stripped) |
| `erased-serde` | `0.4.11` | Transitive | Unverified (LTO Inlined / Stripped) |
| `fdeflate` | `0.3.7` | Transitive | Unverified (LTO Inlined / Stripped) |
| `flate2` | `1.1.10` | Transitive | Unverified (LTO Inlined / Stripped) |
| `form_urlencoded` | `1.2.2` | Transitive | Unverified (LTO Inlined / Stripped) |
| `getrandom` | `0.4.3` | Transitive | Unverified (LTO Inlined / Stripped) |
| `glob` | `0.3.4` | Transitive | Unverified (LTO Inlined / Stripped) |
| `hashbrown` | `0.17.1` | Transitive | Unverified (LTO Inlined / Stripped) |
| `heck` | `0.5.0` | Transitive | Unverified (LTO Inlined / Stripped) |
| `html5ever` | `0.39.0` | Transitive | Unverified (LTO Inlined / Stripped) |
| `http` | `1.5.0` | Transitive | Unverified (LTO Inlined / Stripped) |
| `idna` | `1.1.0` | Transitive | Unverified (LTO Inlined / Stripped) |
| `itoa` | `1.0.18` | Transitive | Unverified (LTO Inlined / Stripped) |
| `jsonptr` | `0.7.1` | Transitive | Unverified (LTO Inlined / Stripped) |
| `keyboard-types` | `0.8.3` | Transitive | Unverified (LTO Inlined / Stripped) |
| `libc` | `0.2.191` | Transitive | Unverified (LTO Inlined / Stripped) |
| `lock_api` | `0.4.14` | Transitive | Unverified (LTO Inlined / Stripped) |
| `log` | `0.4.35` | Transitive | Unverified (LTO Inlined / Stripped) |
| `markup5ever` | `0.39.0` | Transitive | Unverified (LTO Inlined / Stripped) |
| `mime` | `0.3.17` | Transitive | Unverified (LTO Inlined / Stripped) |
| `num-conv` | `0.2.2` | Transitive | Unverified (LTO Inlined / Stripped) |
| `once_cell` | `1.21.4` | Transitive | Unverified (LTO Inlined / Stripped) |
| `parking_lot` | `0.12.5` | Transitive | Unverified (LTO Inlined / Stripped) |
| `parking_lot_core` | `0.9.12` | Transitive | Unverified (LTO Inlined / Stripped) |
| `percent-encoding` | `2.3.2` | Transitive | Unverified (LTO Inlined / Stripped) |
| `png` | `0.18.1` | Transitive | Unverified (LTO Inlined / Stripped) |
| `powerfmt` | `0.2.1` | Transitive | Unverified (LTO Inlined / Stripped) |
| `proc-macro2` | `1.0.107` | Transitive | Unverified (LTO Inlined / Stripped) |
| `quote` | `1.0.47` | Transitive | Unverified (LTO Inlined / Stripped) |
| `regex` | `1.13.1` | Transitive | Unverified (LTO Inlined / Stripped) |
| `regex-automata` | `0.4.18` | Transitive | Unverified (LTO Inlined / Stripped) |
| `regex-syntax` | `0.8.11` | Transitive | Unverified (LTO Inlined / Stripped) |
| `scopeguard` | `1.2.0` | Transitive | Unverified (LTO Inlined / Stripped) |
| `semver` | `1.0.28` | Transitive | Unverified (LTO Inlined / Stripped) |
| `serde` | `1.0.229` | **Direct** | Strongly supported (String Literal / Core Module) |
| `serde_core` | `1.0.229` | Transitive | Unverified (LTO Inlined / Stripped) |
| `serde_derive` | `1.0.229` | Transitive | Unverified (LTO Inlined / Stripped) |
| `serde_derive_internals` | `0.29.1` | Transitive | Unverified (LTO Inlined / Stripped) |
| `serde_json` | `1.0.154` | **Direct** | Strongly supported (String Literal / Core Module) |
| `serde_repr` | `0.1.21` | Transitive | Unverified (LTO Inlined / Stripped) |
| `serde_spanned` | `1.1.2` | Transitive | Unverified (LTO Inlined / Stripped) |
| `serde_with` | `3.24.0` | Transitive | Unverified (LTO Inlined / Stripped) |
| `serde_with_macros` | `3.24.0` | Transitive | Unverified (LTO Inlined / Stripped) |
| `serde-untagged` | `0.1.9` | Transitive | Unverified (LTO Inlined / Stripped) |
| `serialize-to-javascript` | `0.1.2` | Transitive | Unverified (LTO Inlined / Stripped) |
| `serialize-to-javascript-impl` | `0.1.2` | Transitive | Unverified (LTO Inlined / Stripped) |
| `servo_arc` | `0.4.3` | Transitive | Unverified (LTO Inlined / Stripped) |
| `sha2` | `0.10.9` | **Direct** | Strongly supported (String Literal / Core Module) |
| `siphasher` | `1.0.4` | Transitive | Unverified (LTO Inlined / Stripped) |
| `smallvec` | `1.16.4` | Transitive | Unverified (LTO Inlined / Stripped) |
| `socket2` | `0.6.5` | Transitive | Unverified (LTO Inlined / Stripped) |
| `softbuffer` | `0.4.8` | Transitive | Unverified (LTO Inlined / Stripped) |
| `stable_deref_trait` | `1.2.1` | Transitive | Unverified (LTO Inlined / Stripped) |
| `string_cache` | `0.9.0` | Transitive | Unverified (LTO Inlined / Stripped) |
| `syn` | `2.0.119` | Transitive | Unverified (LTO Inlined / Stripped) |
| `tendril` | `0.5.1` | Transitive | Unverified (LTO Inlined / Stripped) |
| `thiserror` | `2.0.21` | Transitive | Unverified (LTO Inlined / Stripped) |
| `thiserror-impl` | `2.0.21` | Transitive | Unverified (LTO Inlined / Stripped) |
| `time` | `0.3.55` | Transitive | Unverified (LTO Inlined / Stripped) |
| `time-core` | `0.1.9` | Transitive | Unverified (LTO Inlined / Stripped) |
| `time-macros` | `0.2.32` | Transitive | Unverified (LTO Inlined / Stripped) |
| `toml` | `1.1.8+spec-1.1.0` | Transitive | Unverified (LTO Inlined / Stripped) |
| `toml_datetime` | `1.1.2+spec-1.1.0` | Transitive | Unverified (LTO Inlined / Stripped) |
| `toml_parser` | `1.1.5+spec-1.1.0` | Transitive | Unverified (LTO Inlined / Stripped) |
| `toml_writer` | `1.1.3+spec-1.1.0` | Transitive | Unverified (LTO Inlined / Stripped) |
| `tray-icon` | `0.25.1` | Transitive | Unverified (LTO Inlined / Stripped) |
| `typeid` | `1.0.3` | Transitive | Unverified (LTO Inlined / Stripped) |
| `typenum` | `1.20.1` | Transitive | Unverified (LTO Inlined / Stripped) |
| `unicode-segmentation` | `1.13.3` | Transitive | Unverified (LTO Inlined / Stripped) |
| `url` | `2.5.8` | Transitive | Unverified (LTO Inlined / Stripped) |
| `web_atoms` | `0.2.6` | Transitive | Unverified (LTO Inlined / Stripped) |
| `web-time` | `1.1.0` | Transitive | Unverified (LTO Inlined / Stripped) |
| `windows` | `0.62.2` | Transitive | Unverified (LTO Inlined / Stripped) |
| `windows-collections` | `0.3.2` | Transitive | Unverified (LTO Inlined / Stripped) |
| `windows-core` | `0.62.2` | Transitive | Unverified (LTO Inlined / Stripped) |
| `windows-future` | `0.3.2` | Transitive | Unverified (LTO Inlined / Stripped) |
| `windows-implement` | `0.60.2` | Transitive | Unverified (LTO Inlined / Stripped) |
| `windows-interface` | `0.59.3` | Transitive | Unverified (LTO Inlined / Stripped) |
| `windows-link` | `0.2.1` | Transitive | Unverified (LTO Inlined / Stripped) |
| `windows-numerics` | `0.3.1` | Transitive | Unverified (LTO Inlined / Stripped) |
| `windows-result` | `0.4.1` | Transitive | Unverified (LTO Inlined / Stripped) |
| `windows-strings` | `0.5.1` | Transitive | Unverified (LTO Inlined / Stripped) |
| `windows-sys` | `0.61.2` | **Direct** | Confirmed (PDB / Import Table) |
| `windows-threading` | `0.2.1` | Transitive | Unverified (LTO Inlined / Stripped) |
| `windows-version` | `0.1.7` | Transitive | Unverified (LTO Inlined / Stripped) |

#### License: `(MIT OR Apache-2.0) AND Unicode-3.0` (1 package)

| Package | Version | Type | Notes |
|---|---|---|---|
| `unicode-ident` | `1.0.27` | Transitive | Unverified (LTO Inlined / Stripped) |

#### License: `Unlicense OR MIT` (4 packages)

| Package | Version | Type | Notes |
|---|---|---|---|
| `aho-corasick` | `1.1.5` | Transitive | Unverified (LTO Inlined / Stripped) |
| `byteorder` | `1.5.0` | Transitive | Unverified (LTO Inlined / Stripped) |
| `memchr` | `2.8.3` | Transitive | Unverified (LTO Inlined / Stripped) |
| `winapi-util` | `0.1.11` | Transitive | Unverified (LTO Inlined / Stripped) |

#### License: `MIT` (38 packages)

| Package | Version | Type | Notes |
|---|---|---|---|
| `bytes` | `1.12.1` | Transitive | Unverified (LTO Inlined / Stripped) |
| `cargo_metadata` | `0.19.2` | Transitive | Unverified (LTO Inlined / Stripped) |
| `cfb` | `0.14.0` | Transitive | Unverified (LTO Inlined / Stripped) |
| `darling` | `0.24.1` | Transitive | Unverified (LTO Inlined / Stripped) |
| `darling_core` | `0.24.1` | Transitive | Unverified (LTO Inlined / Stripped) |
| `darling_macro` | `0.24.1` | Transitive | Unverified (LTO Inlined / Stripped) |
| `derive_more` | `2.1.1` | Transitive | Unverified (LTO Inlined / Stripped) |
| `derive_more-impl` | `2.1.1` | Transitive | Unverified (LTO Inlined / Stripped) |
| `dom_query` | `0.28.0` | Transitive | Unverified (LTO Inlined / Stripped) |
| `generic-array` | `0.14.7` | Transitive | Unverified (LTO Inlined / Stripped) |
| `ico` | `0.6.0` | Transitive | Unverified (LTO Inlined / Stripped) |
| `infer` | `0.22.0` | Transitive | Unverified (LTO Inlined / Stripped) |
| `mio` | `1.2.4` | Transitive | Unverified (LTO Inlined / Stripped) |
| `new_debug_unreachable` | `1.0.6` | Transitive | Unverified (LTO Inlined / Stripped) |
| `open` | `5.4.5` | Transitive | Unverified (LTO Inlined / Stripped) |
| `phf` | `0.13.1` | Transitive | Unverified (LTO Inlined / Stripped) |
| `phf_generator` | `0.13.1` | Transitive | Unverified (LTO Inlined / Stripped) |
| `phf_macros` | `0.13.1` | Transitive | Unverified (LTO Inlined / Stripped) |
| `phf_shared` | `0.13.1` | Transitive | Unverified (LTO Inlined / Stripped) |
| `plist` | `1.10.3` | Transitive | Unverified (LTO Inlined / Stripped) |
| `precomputed-hash` | `0.1.1` | Transitive | Unverified (LTO Inlined / Stripped) |
| `quick-xml` | `0.42.0` | Transitive | Unverified (LTO Inlined / Stripped) |
| `schemars` | `0.8.22` | Transitive | Unverified (LTO Inlined / Stripped) |
| `schemars_derive` | `0.8.22` | Transitive | Unverified (LTO Inlined / Stripped) |
| `simd-adler32` | `0.3.10` | Transitive | Unverified (LTO Inlined / Stripped) |
| `strsim` | `0.11.1` | Transitive | Unverified (LTO Inlined / Stripped) |
| `synstructure` | `0.14.0` | Transitive | Unverified (LTO Inlined / Stripped) |
| `tokio` | `1.54.1` | Transitive | Unverified (LTO Inlined / Stripped) |
| `tracing` | `0.1.44` | Transitive | Unverified (LTO Inlined / Stripped) |
| `tracing-attributes` | `0.1.31` | Transitive | Unverified (LTO Inlined / Stripped) |
| `tracing-core` | `0.1.36` | Transitive | Unverified (LTO Inlined / Stripped) |
| `urlpattern` | `0.6.0` | Transitive | Unverified (LTO Inlined / Stripped) |
| `webview2-com` | `0.39.1` | Transitive | Unverified (LTO Inlined / Stripped) |
| `webview2-com-macros` | `0.8.1` | Transitive | Unverified (LTO Inlined / Stripped) |
| `webview2-com-sys` | `0.39.1` | Transitive | Unverified (LTO Inlined / Stripped) |
| `winnow` | `1.0.4` | Transitive | Unverified (LTO Inlined / Stripped) |
| `winreg` | `0.56.0` | **Direct** | Strongly supported (String Literal / Core Module) |
| `zmij` | `1.0.23` | Transitive | Unverified (LTO Inlined / Stripped) |

#### License: `Apache-2.0 OR MIT` (23 packages)

| Package | Version | Type | Notes |
|---|---|---|---|
| `bit-set` | `0.8.0` | Transitive | Unverified (LTO Inlined / Stripped) |
| `bit-vec` | `0.8.0` | Transitive | Unverified (LTO Inlined / Stripped) |
| `ctor` | `1.0.13` | Transitive | Unverified (LTO Inlined / Stripped) |
| `equivalent` | `1.0.3` | Transitive | Unverified (LTO Inlined / Stripped) |
| `fastrand` | `2.5.0` | Transitive | Unverified (LTO Inlined / Stripped) |
| `idna_adapter` | `1.2.2` | Transitive | Unverified (LTO Inlined / Stripped) |
| `indexmap` | `2.14.2` | Transitive | Unverified (LTO Inlined / Stripped) |
| `muda` | `0.20.0` | Transitive | Unverified (LTO Inlined / Stripped) |
| `pin-project-lite` | `0.2.17` | Transitive | Unverified (LTO Inlined / Stripped) |
| `rustc-hash` | `2.1.3` | Transitive | Unverified (LTO Inlined / Stripped) |
| `tauri` | `2.12.5` | **Direct** | Strongly supported (String Literal / Core Module) |
| `tauri-codegen` | `2.7.4` | Transitive | Unverified (LTO Inlined / Stripped) |
| `tauri-macros` | `2.7.4` | Transitive | Unverified (LTO Inlined / Stripped) |
| `tauri-plugin-opener` | `2.7.2` | **Direct** | Strongly supported (String Literal / Core Module) |
| `tauri-plugin-single-instance` | `2.5.5` | **Direct** | Strongly supported (String Literal / Core Module) |
| `tauri-runtime` | `2.12.2` | Transitive | Unverified (LTO Inlined / Stripped) |
| `tauri-runtime-wry` | `2.12.3` | Transitive | Unverified (LTO Inlined / Stripped) |
| `tauri-utils` | `2.10.2` | Transitive | Unverified (LTO Inlined / Stripped) |
| `utf_types` | `1.0.0` | Transitive | Unverified (LTO Inlined / Stripped) |
| `utf8_iter` | `1.1.0` | Transitive | Unverified (LTO Inlined / Stripped) |
| `uuid` | `1.29.0` | Transitive | Unverified (LTO Inlined / Stripped) |
| `window-vibrancy` | `0.8.1` | Transitive | Unverified (LTO Inlined / Stripped) |
| `wry` | `0.57.1` | Transitive | Unverified (LTO Inlined / Stripped) |

#### License: `MPL-2.0` (5 packages)

| Package | Version | Type | Notes |
|---|---|---|---|
| `cssparser` | `0.37.0` | Transitive | See Section 5 (Uncalled HTML parser) |
| `cssparser-macros` | `0.7.1` | Transitive | See Section 5 (Uncalled HTML parser) |
| `dtoa-short` | `0.3.5` | Transitive | See Section 5 (Uncalled HTML parser) |
| `option-ext` | `0.2.0` | Transitive | See Section 5 (Uncalled on Windows) |
| `selectors` | `0.38.0` | Transitive | See Section 5 (Uncalled HTML parser) |

#### License: `CC0-1.0 OR MIT-0 OR Apache-2.0` (1 package)

| Package | Version | Type | Notes |
|---|---|---|---|
| `dunce` | `1.0.5` | Transitive | Unverified (LTO Inlined / Stripped) |

#### License: `Apache-2.0 AND MIT` (1 package)

| Package | Version | Type | Notes |
|---|---|---|---|
| `dpi` | `0.1.2` | Transitive | Unverified (LTO Inlined / Stripped) |

#### License: `MIT OR Apache-2.0 OR Zlib` (1 package)

| Package | Version | Type | Notes |
|---|---|---|---|
| `raw-window-handle` | `0.6.2` | Transitive | Unverified (LTO Inlined / Stripped) |

#### License: `BSD-3-Clause AND MIT` (1 package)

| Package | Version | Type | Notes |
|---|---|---|---|
| `brotli` | `9.1.1` | Transitive | Unverified (LTO Inlined / Stripped) |

#### License: `BSD-3-Clause` (2 packages)

| Package | Version | Type | Notes |
|---|---|---|---|
| `alloc-no-stdlib` | `3.0.0` | Transitive | Unverified (LTO Inlined / Stripped) |
| `alloc-stdlib` | `0.3.0` | Transitive | Unverified (LTO Inlined / Stripped) |

#### License: `BSD-3-Clause/MIT` (1 package)

| Package | Version | Type | Notes |
|---|---|---|---|
| `brotli-decompressor` | `6.2.0` | Transitive | Unverified (LTO Inlined / Stripped) |

#### License: `MIT OR Zlib OR Apache-2.0` (1 package)

| Package | Version | Type | Notes |
|---|---|---|---|
| `miniz_oxide` | `0.8.9` | Transitive | Unverified (LTO Inlined / Stripped) |

#### License: `0BSD OR MIT OR Apache-2.0` (1 package)

| Package | Version | Type | Notes |
|---|---|---|---|
| `adler2` | `2.0.1` | Transitive | Unverified (LTO Inlined / Stripped) |

#### License: `MIT/Apache-2.0` (2 packages)

| Package | Version | Type | Notes |
|---|---|---|---|
| `ident_case` | `1.0.1` | Transitive | Unverified (LTO Inlined / Stripped) |
| `json-patch` | `4.2.0` | Transitive | Unverified (LTO Inlined / Stripped) |

#### License: `Zlib` (1 package)

| Package | Version | Type | Notes |
|---|---|---|---|
| `foldhash` | `0.2.0` | Transitive | Unverified (LTO Inlined / Stripped) |

#### License: `Apache-2.0 / MIT` (1 package)

| Package | Version | Type | Notes |
|---|---|---|---|
| `fnv` | `1.0.7` | Transitive | Unverified (LTO Inlined / Stripped) |

#### License: `Unicode-3.0` (18 packages)

| Package | Version | Type | Notes |
|---|---|---|---|
| `icu_collections` | `2.4.0` | Transitive | Unverified (LTO Inlined / Stripped) |
| `icu_locale_core` | `2.4.0` | Transitive | Unverified (LTO Inlined / Stripped) |
| `icu_normalizer` | `2.4.0` | Transitive | Strongly supported (String Literal / Core Module) |
| `icu_normalizer_data` | `2.4.0` | Transitive | Unverified (LTO Inlined / Stripped) |
| `icu_properties` | `2.4.0` | Transitive | Unverified (LTO Inlined / Stripped) |
| `icu_properties_data` | `2.4.0` | Transitive | Unverified (LTO Inlined / Stripped) |
| `icu_provider` | `2.4.0` | Transitive | Unverified (LTO Inlined / Stripped) |
| `litemap` | `0.8.3` | Transitive | Unverified (LTO Inlined / Stripped) |
| `potential_utf` | `0.1.6` | Transitive | Unverified (LTO Inlined / Stripped) |
| `tinystr` | `0.8.5` | Transitive | Unverified (LTO Inlined / Stripped) |
| `writeable` | `0.6.4` | Transitive | Unverified (LTO Inlined / Stripped) |
| `yoke` | `0.8.4` | Transitive | Unverified (LTO Inlined / Stripped) |
| `yoke-derive` | `0.8.4` | Transitive | Unverified (LTO Inlined / Stripped) |
| `zerofrom` | `0.1.8` | Transitive | Unverified (LTO Inlined / Stripped) |
| `zerofrom-derive` | `0.1.8` | Transitive | Unverified (LTO Inlined / Stripped) |
| `zerotrie` | `0.2.5` | Transitive | Unverified (LTO Inlined / Stripped) |
| `zerovec` | `0.11.9` | Transitive | Unverified (LTO Inlined / Stripped) |
| `zerovec-derive` | `0.11.7` | Transitive | Unverified (LTO Inlined / Stripped) |

#### License: `Unlicense/MIT` (2 packages)

| Package | Version | Type | Notes |
|---|---|---|---|
| `same-file` | `1.0.6` | Transitive | Unverified (LTO Inlined / Stripped) |
| `walkdir` | `2.5.0` | Transitive | Unverified (LTO Inlined / Stripped) |

#### License: `Apache-2.0` (1 package)

| Package | Version | Type | Notes |
|---|---|---|---|
| `tao` | `0.37.2` | Transitive | Unverified (LTO Inlined / Stripped) |


---

### Standard License Texts for Rust Dependency Families

#### 1. The MIT License
```text
MIT License

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
```

#### 2. Apache License, Version 2.0
```text
Apache License
                           Version 2.0, January 2004
                        http://www.apache.org/licenses/

   TERMS AND CONDITIONS FOR USE, REPRODUCTION, AND DISTRIBUTION

   1. Definitions.

      "License" shall mean the terms and conditions for use, reproduction,
      and distribution as defined by Sections 1 through 9 of this document.

      "Licensor" shall mean the copyright owner or entity authorized by
      the copyright owner that is granting the License.

      "Legal Entity" shall mean the union of the acting entity and all
      other entities that control, are controlled by, or are under common
      control with that entity. For the purposes of this definition,
      "control" means (i) the power, direct or indirect, to cause the
      direction or management of such entity, whether by contract or
      otherwise, or (ii) ownership of fifty percent (50%) or more of the
      outstanding shares, or (iii) beneficial ownership of such entity.

      "You" (or "Your") shall mean an individual or Legal Entity
      exercising permissions granted by this License.

      "Source" form shall mean the preferred form for making modifications,
      including but not limited to software source code, documentation
      source, and configuration files.

      "Object" form shall mean any form resulting from mechanical
      transformation or translation of a Source form, including but
      not limited to compiled object code, generated documentation,
      and conversions to other media types.

      "Work" shall mean the work of authorship, whether in Source or
      Object form, made available under the License, as indicated by a
      copyright notice that is included in or attached to the work
      (an example is provided in the Appendix below).

      "Derivative Works" shall mean any work, whether in Source or Object
      form, that is based on (or derived from) the Work and for which the
      editorial revisions, annotations, elaborations, or other modifications
      represent, as a whole, an original work of authorship. For the purposes
      of this License, Derivative Works shall not include works that remain
      separable from, or merely link (or bind by name) to the interfaces of,
      the Work and Derivative Works thereof.

      "Contribution" shall mean any work of authorship, including
      the original version of the Work and any modifications or additions
      to that Work or Derivative Works thereof, that is intentionally
      submitted to Licensor for inclusion in the Work by the copyright owner
      or by an individual or Legal Entity authorized to submit on behalf of
      the copyright owner. For the purposes of this definition, "submitted"
      means any form of electronic, verbal, or written communication sent
      to the Licensor or its representatives, including but not limited to
      communication on electronic mailing lists, source code control systems,
      and issue tracking systems that are managed by, or on behalf of, the
      Licensor for the purpose of discussing and improving the Work, but
      excluding communication that is conspicuously marked or otherwise
      designated in writing by the copyright owner as "Not a Contribution."

      "Contributor" shall mean Licensor and any individual or Legal Entity
      on behalf of whom a Contribution has been received by Licensor and
      subsequently incorporated within the Work.

   2. Grant of Copyright License. Subject to the terms and conditions of
      this License, each Contributor hereby grants to You a perpetual,
      worldwide, non-exclusive, no-charge, royalty-free, irrevocable
      copyright license to reproduce, prepare Derivative Works of,
      publicly display, publicly perform, sublicense, and distribute the
      Work and such Derivative Works in Source or Object form.

   3. Grant of Patent License. Subject to the terms and conditions of
      this License, each Contributor hereby grants to You a perpetual,
      worldwide, non-exclusive, no-charge, royalty-free, irrevocable
      (except as stated in this section) patent license to make, have made,
      use, offer to sell, sell, import, and otherwise transfer the Work,
      where such license applies only to those patent claims licensable
      by such Contributor that are necessarily infringed by their
      Contribution(s) alone or by combination of their Contribution(s)
      with the Work to which such Contribution(s) was submitted. If You
      institute patent litigation against any entity (including a
      cross-claim or counterclaim in a lawsuit) alleging that the Work
      or a Contribution incorporated within the Work constitutes direct
      or contributory patent infringement, then any patent licenses
      granted to You under this License for that Work shall terminate
      as of the date such litigation is filed.

   4. Redistribution. You may reproduce and distribute copies of the
      Work or Derivative Works thereof in any medium, with or without
      modifications, and in Source or Object form, provided that You
      meet the following conditions:

      (a) You must give any other recipients of the Work or
          Derivative Works a copy of this License; and

      (b) You must cause any modified files to carry prominent notices
          stating that You changed the files; and

      (c) You must retain, in the Source form of any Derivative Works
          that You distribute, all copyright, patent, trademark, and
          attribution notices from the Source form of the Work,
          excluding those notices that do not pertain to any part of
          the Derivative Works; and

      (d) If the Work includes a "NOTICE" text file as part of its
          distribution, then any Derivative Works that You distribute must
          include a readable copy of the attribution notices contained
          within such NOTICE file, excluding those notices that do not
          pertain to any part of the Derivative Works, in at least one
          of the following places: within a NOTICE text file distributed
          as part of the Derivative Works; within the Source form or
          documentation, if provided along with the Derivative Works; or,
          within a display generated by the Derivative Works, if and
          wherever such third-party notices normally appear. The contents
          of the NOTICE file are for informational purposes only and
          do not modify the License. You may add Your own attribution
          notices within Derivative Works that You distribute, alongside
          or as an addendum to the NOTICE text from the Work, provided
          that such additional attribution notices cannot be construed
          as modifying the License.

      You may add Your own copyright statement to Your modifications and
      may provide additional or different license terms and conditions
      for use, reproduction, or distribution of Your modifications, or
      for any such Derivative Works as a whole, provided Your use,
      reproduction, and distribution of the Work otherwise complies with
      the conditions stated in this License.

   5. Submission of Contributions. Unless You explicitly state otherwise,
      any Contribution intentionally submitted for inclusion in the Work
      by You to the Licensor shall be under the terms and conditions of
      this License, without any additional terms or conditions.
      Notwithstanding the above, nothing herein shall supersede or modify
      the terms of any separate license agreement you may have executed
      with Licensor regarding such Contributions.

   6. Trademarks. This License does not grant permission to use the trade
      names, trademarks, service marks, or product names of the Licensor,
      except as required for reasonable and customary use in describing the
      origin of the Work and reproducing the content of the NOTICE file.

   7. Disclaimer of Warranty. Unless required by applicable law or
      agreed to in writing, Licensor provides the Work (and each
      Contributor provides its Contributions) on an "AS IS" BASIS,
      WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or
      implied, including, without limitation, any warranties or conditions
      of TITLE, NON-INFRINGEMENT, MERCHANTABILITY, or FITNESS FOR A
      PARTICULAR PURPOSE. You are solely responsible for determining the
      appropriateness of using or redistributing the Work and assume any
      risks associated with Your exercise of permissions under this License.

   8. Limitation of Liability. In no event and under no legal theory,
      whether in tort (including negligence), contract, or otherwise,
      unless required by applicable law (such as deliberate and grossly
      negligent acts) or agreed to in writing, shall any Contributor be
      liable to You for damages, including any direct, indirect, special,
      incidental, or consequential damages of any character arising as a
      result of this License or out of the use or inability to use the
      Work (including but not limited to damages for loss of goodwill,
      work stoppage, computer failure or malfunction, or any and all
      other commercial damages or losses), even if such Contributor
      has been advised of the possibility of such damages.

   9. Accepting Warranty or Additional Liability. While redistributing
      the Work or Derivative Works thereof, You may choose to offer,
      and charge a fee for, acceptance of support, warranty, indemnity,
      or other liability obligations and/or rights consistent with this
      License. However, in accepting such obligations, You may act only
      on Your own behalf and on Your sole responsibility, not on behalf
      of any other Contributor, and only if You agree to indemnify,
      defend, and hold each Contributor harmless for any liability
      incurred by, or claims asserted against, such Contributor by reason
      of your accepting any such warranty or additional liability.

   END OF TERMS AND CONDITIONS
```

#### 3. Unicode License Agreement - Data Files and Software (Unicode-3.0)
```text
UNICODE LICENSE V3

COPYRIGHT AND PERMISSION NOTICE

Copyright © 2020-2024 Unicode, Inc.

NOTICE TO USER: Carefully read the following legal agreement. BY
DOWNLOADING, INSTALLING, COPYING OR OTHERWISE USING DATA FILES, AND/OR
SOFTWARE, YOU UNEQUIVOCALLY ACCEPT, AND AGREE TO BE BOUND BY, ALL OF THE
TERMS AND CONDITIONS OF THIS AGREEMENT. IF YOU DO NOT AGREE, DO NOT
DOWNLOAD, INSTALL, COPY, DISTRIBUTE OR USE THE DATA FILES OR SOFTWARE.

Permission is hereby granted, free of charge, to any person obtaining a
copy of data files and any associated documentation (the "Data Files") or
software and any associated documentation (the "Software") to deal in the
Data Files or Software without restriction, including without limitation
the rights to use, copy, modify, merge, publish, distribute, and/or sell
copies of the Data Files or Software, and to permit persons to whom the
Data Files or Software are furnished to do so, provided that either (a)
this copyright and permission notice appear with all copies of the Data
Files or Software, or (b) this copyright and permission notice appear in
associated Documentation.

THE DATA FILES AND SOFTWARE ARE PROVIDED "AS IS", WITHOUT WARRANTY OF ANY
KIND, EXPRESS OR IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF
MERCHANTABILITY, FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT OF
THIRD PARTY RIGHTS.

IN NO EVENT SHALL THE COPYRIGHT HOLDER OR HOLDERS INCLUDED IN THIS NOTICE
BE LIABLE FOR ANY CLAIM, OR ANY SPECIAL INDIRECT OR CONSEQUENTIAL DAMAGES,
OR ANY DAMAGES WHATSOEVER RESULTING FROM LOSS OF USE, DATA OR PROFITS,
WHETHER IN AN ACTION OF CONTRACT, NEGLIGENCE OR OTHER TORTIOUS ACTION,
ARISING OUT OF OR IN CONNECTION WITH THE USE OR PERFORMANCE OF THE DATA
FILES OR SOFTWARE.

Except as contained in this notice, the name of a copyright holder shall
not be used in advertising or otherwise to promote the sale, use or other
dealings in these Data Files or Software without prior written
authorization of the copyright holder.

SPDX-License-Identifier: Unicode-3.0

—

Portions of ICU4X may have been adapted from ICU4C and/or ICU4J.
ICU 1.8.1 to ICU 57.1 © 1995-2016 International Business Machines Corporation and others.
```

#### 4. The 3-Clause BSD License
```text
Copyright (c) 2016 Dropbox, Inc.
All rights reserved.

Redistribution and use in source and binary forms, with or without modification, are permitted provided that the following conditions are met:

1. Redistributions of source code must retain the above copyright notice, this list of conditions and the following disclaimer.

2. Redistributions in binary form must reproduce the above copyright notice, this list of conditions and the following disclaimer in the documentation and/or other materials provided with the distribution.

3. Neither the name of the copyright holder nor the names of its contributors may be used to endorse or promote products derived from this software without specific prior written permission.

THIS SOFTWARE IS PROVIDED BY THE COPYRIGHT HOLDERS AND CONTRIBUTORS "AS IS" AND ANY EXPRESS OR IMPLIED WARRANTIES, INCLUDING, BUT NOT LIMITED TO, THE IMPLIED WARRANTIES OF MERCHANTABILITY AND FITNESS FOR A PARTICULAR PURPOSE ARE DISCLAIMED. IN NO EVENT SHALL THE COPYRIGHT HOLDER OR CONTRIBUTORS BE LIABLE FOR ANY DIRECT, INDIRECT, INCIDENTAL, SPECIAL, EXEMPLARY, OR CONSEQUENTIAL DAMAGES (INCLUDING, BUT NOT LIMITED TO, PROCUREMENT OF SUBSTITUTE GOODS OR SERVICES; LOSS OF USE, DATA, OR PROFITS; OR BUSINESS INTERRUPTION) HOWEVER CAUSED AND ON ANY THEORY OF LIABILITY, WHETHER IN CONTRACT, STRICT LIABILITY, OR TORT (INCLUDING NEGLIGENCE OR OTHERWISE) ARISING IN ANY WAY OUT OF THE USE OF THIS SOFTWARE, EVEN IF ADVISED OF THE POSSIBILITY OF SUCH DAMAGE.
```

#### 5. The zlib/libpng License (Zlib)
```text
Copyright (c) 2024 Orson Peters

This software is provided 'as-is', without any express or implied warranty. In
no event will the authors be held liable for any damages arising from the use of
this software.

Permission is granted to anyone to use this software for any purpose, including
commercial applications, and to alter it and redistribute it freely, subject to
the following restrictions:

1. The origin of this software must not be misrepresented; you must not claim
    that you wrote the original software. If you use this software in a product,
    an acknowledgment in the product documentation would be appreciated but is
    not required.

2. Altered source versions must be plainly marked as such, and must not be
    misrepresented as being the original software.

3. This notice may not be removed or altered from any source distribution.
```

#### 6. The Unlicense (Public Domain Dedication)
```text
This is free and unencumbered software released into the public domain.

Anyone is free to copy, modify, publish, use, compile, sell, or
distribute this software, either in source code form or as a compiled
binary, for any purpose, commercial or non-commercial, and by any
means.

In jurisdictions that recognize copyright laws, the author or authors
of this software dedicate any and all copyright interest in the
software to the public domain. We make this dedication for the benefit
of the public at large and to the detriment of our heirs and
successors. We intend this dedication to be an overt act of
relinquishment in perpetuity of all present and future rights to this
software under copyright law.

THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND,
EXPRESS OR IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF
MERCHANTABILITY, FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT.
IN NO EVENT SHALL THE AUTHORS BE LIABLE FOR ANY CLAIM, DAMAGES OR
OTHER LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE,
ARISING FROM, OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR
OTHER DEALINGS IN THE SOFTWARE.

For more information, please refer to <http://unlicense.org/>
```

#### 7. The 0BSD License (Zero-Clause BSD)
```text
Copyright (C) Jonas Schievink <jonasschievink@gmail.com>

Permission to use, copy, modify, and/or distribute this software for
any purpose with or without fee is hereby granted.

THE SOFTWARE IS PROVIDED "AS IS" AND THE AUTHOR DISCLAIMS ALL WARRANTIES
WITH REGARD TO THIS SOFTWARE INCLUDING ALL IMPLIED WARRANTIES OF
MERCHANTABILITY AND FITNESS. IN NO EVENT SHALL THE AUTHOR BE LIABLE FOR
ANY SPECIAL, DIRECT, INDIRECT, OR CONSEQUENTIAL DAMAGES OR ANY DAMAGES
WHATSOEVER RESULTING FROM LOSS OF USE, DATA OR PROFITS, WHETHER IN
AN ACTION OF CONTRACT, NEGLIGENCE OR OTHER TORTIOUS ACTION, ARISING OUT
OF OR IN CONNECTION WITH THE USE OR PERFORMANCE OF THIS SOFTWARE.
```

#### 8. MIT No Attribution License (MIT-0)
```text
MIT No Attribution

Permission is hereby granted, free of charge, to any person obtaining a copy of this
software and associated documentation files (the "Software"), to deal in the Software
without restriction, including without limitation the rights to use, copy, modify,
merge, publish, distribute, sublicense, and/or sell copies of the Software, and to
permit persons to whom the Software is furnished to do so.

THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR IMPLIED,
INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY, FITNESS FOR A
PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE AUTHORS OR COPYRIGHT
HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER LIABILITY, WHETHER IN AN ACTION
OF CONTRACT, TORT OR OTHERWISE, ARISING FROM, OUT OF OR IN CONNECTION WITH THE
SOFTWARE OR THE USE OR OTHER DEALINGS IN THE SOFTWARE.
```

#### 9. Creative Commons Zero 1.0 Universal (CC0 1.0)
```text
Creative Commons Legal Code

CC0 1.0 Universal

    CREATIVE COMMONS CORPORATION IS NOT A LAW FIRM AND DOES NOT PROVIDE
    LEGAL SERVICES. DISTRIBUTION OF THIS DOCUMENT DOES NOT CREATE AN
    ATTORNEY-CLIENT RELATIONSHIP. CREATIVE COMMONS PROVIDES THIS
    INFORMATION ON AN "AS-IS" BASIS. CREATIVE COMMONS MAKES NO WARRANTIES
    REGARDING THE USE OF THIS DOCUMENT OR THE INFORMATION OR WORKS
    PROVIDED HEREUNDER, AND DISCLAIMS LIABILITY FOR DAMAGES RESULTING FROM
    THE USE OF THIS DOCUMENT OR THE INFORMATION OR WORKS PROVIDED
    HEREUNDER.

Statement of Purpose

The laws of most jurisdictions throughout the world automatically confer
exclusive Copyright and Related Rights (defined below) upon the creator
and subsequent owner(s) (each and all, an "owner") of an original work of
authorship and/or a database (each, a "Work").

Certain owners wish to permanently relinquish those rights to a Work for
the purpose of contributing to a commons of creative, cultural and
scientific works ("Commons") that the public can reliably and without fear
of later claims of infringement build upon, modify, incorporate in other
works, reuse and redistribute as freely as possible in any form whatsoever
and for any purposes, including without limitation commercial purposes.
These owners may contribute to the Commons to promote the ideal of a free
culture and the further production of creative, cultural and scientific
works, or to gain reputation or greater distribution for their Work in
part through the use and efforts of others.

For these and/or other purposes and motivations, and without any
expectation of additional consideration or compensation, the person
associating CC0 with a Work (the "Affirmer"), to the extent that he or she
is an owner of Copyright and Related Rights in the Work, voluntarily
elects to apply CC0 to the Work and publicly distribute the Work under its
terms, with knowledge of his or her Copyright and Related Rights in the
Work and the meaning and intended legal effect of CC0 on those rights.

1. Copyright and Related Rights. A Work made available under CC0 may be
protected by copyright and related or neighboring rights ("Copyright and
Related Rights"). Copyright and Related Rights include, but are not
limited to, the following:

  i. the right to reproduce, adapt, distribute, perform, display,
     communicate, and translate a Work;
 ii. moral rights retained by the original author(s) and/or performer(s);
iii. publicity and privacy rights pertaining to a person's image or
     likeness depicted in a Work;
 iv. rights protecting against unfair competition in regards to a Work,
     subject to the limitations in paragraph 4(a), below;
  v. rights protecting the extraction, dissemination, use and reuse of data
     in a Work;
 vi. database rights (such as those arising under Directive 96/9/EC of the
     European Parliament and of the Council of 11 March 1996 on the legal
     protection of databases, and under any national implementation
     thereof, including any amended or successor version of such
     directive); and
vii. other similar, equivalent or corresponding rights throughout the
     world based on applicable law or treaty, and any national
     implementations thereof.

2. Waiver. To the greatest extent permitted by, but not in contravention
of, applicable law, Affirmer hereby overtly, fully, permanently,
irrevocably and unconditionally waives, abandons, and surrenders all of
Affirmer's Copyright and Related Rights and associated claims and causes
of action, whether now known or unknown (including existing as well as
future claims and causes of action), in the Work (i) in all territories
worldwide, (ii) for the maximum duration provided by applicable law or
treaty (including future time extensions), (iii) in any current or future
medium and for any number of copies, and (iv) for any purpose whatsoever,
including without limitation commercial, advertising or promotional
purposes (the "Waiver"). Affirmer makes the Waiver for the benefit of each
member of the public at large and to the detriment of Affirmer's heirs and
successors, fully intending that such Waiver shall not be subject to
revocation, rescission, cancellation, termination, or any other legal or
equitable action to disrupt the quiet enjoyment of the Work by the public
as contemplated by Affirmer's express Statement of Purpose.

3. Public License Fallback. Should any part of the Waiver for any reason
be judged legally invalid or ineffective under applicable law, then the
Waiver shall be preserved to the maximum extent permitted taking into
account Affirmer's express Statement of Purpose. In addition, to the
extent the Waiver is so judged Affirmer hereby grants to each affected
person a royalty-free, non transferable, non sublicensable, non exclusive,
irrevocable and unconditional license to exercise Affirmer's Copyright and
Related Rights in the Work (i) in all territories worldwide, (ii) for the
maximum duration provided by applicable law or treaty (including future
time extensions), (iii) in any current or future medium and for any number
of copies, and (iv) for any purpose whatsoever, including without
limitation commercial, advertising or promotional purposes (the
"License"). The License shall be deemed effective as of the date CC0 was
applied by Affirmer to the Work. Should any part of the License for any
reason be judged legally invalid or ineffective under applicable law, such
partial invalidity or ineffectiveness shall not invalidate the remainder
of the License, and in such case Affirmer hereby affirms that he or she
will not (i) exercise any of his or her remaining Copyright and Related
Rights in the Work or (ii) assert any associated claims and causes of
action with respect to the Work, in either case contrary to Affirmer's
express Statement of Purpose.

4. Limitations and Disclaimers.

 a. No trademark or patent rights held by Affirmer are waived, abandoned,
    surrendered, licensed or otherwise affected by this document.
 b. Affirmer offers the Work as-is and makes no representations or
    warranties of any kind concerning the Work, express, implied,
    statutory or otherwise, including without limitation warranties of
    title, merchantability, fitness for a particular purpose, non
    infringement, or the absence of latent or other defects, accuracy, or
    the present or absence of errors, whether or not discoverable, all to
    the greatest extent permissible under applicable law.
 c. Affirmer disclaims responsibility for clearing rights of other persons
    that may apply to the Work or any use thereof, including without
    limitation any person's Copyright and Related Rights in the Work.
    Further, Affirmer disclaims responsibility for obtaining any necessary
    consents, permissions or other rights required for any use of the
    Work.
 d. Affirmer understands and acknowledges that Creative Commons is not a
    party to this document and has no duty or obligation with respect to
    this CC0 or use of the Work.
```

---

## 4. Windows MSI Installer Components

The DeskPilot Windows Installer package (`deskpilot_0.1.1_x64_en-US.msi`) embeds specific third-party components within its internal database to manage installation dialogs and runtime prerequisites:

### A. WiX Toolset v3.14 (Installer Custom Actions & Dialog Resources)
- **Included MSI Components:**
  - `WixUIWixca`: Precompiled custom action DLL (providing path validation and EULA dialog management).
  - UI Dialog Bitmaps & Icons: `WixUI_Bmp_Banner`, `WixUI_Bmp_Dialog`, `WixUI_Ico_Exclam`, `WixUI_Ico_Info`, `WixUI_Bmp_New`, `WixUI_Bmp_Up`.
- **License:** Microsoft Reciprocal License (MS-RL)
- **Copyright:** Copyright (c) .NET Foundation and contributors. All rights reserved.
- **Source Code Availability:** Upstream source code for WiX Toolset v3.14 is publicly available from the .NET Foundation repository at [https://github.com/wixtoolset/wix3](https://github.com/wixtoolset/wix3).
- **Toolchain Distinction:** The WiX compiler tools (`candle.exe` and `light.exe`, version 3.14.1.8722) execute strictly on the build machine to generate the installer database and are **not redistributed**.
- **Application Independence:** Under MS-RL §3(A), reciprocal licensing applies strictly to files containing code from the software; the application binary `deskpilot.exe` contains zero WiX code and remains independently governed by DeskPilot's MIT License. (Note: While standard open-source practice provides upstream repository links, DeskPilot does not claim that providing an upstream link alone conclusively resolves every legal interpretation of MS-RL source redistribution).

#### Microsoft Reciprocal License (MS-RL) Text
```text
Copyright (c) .NET Foundation and contributors.
This software is released under the Microsoft Reciprocal License (MS-RL) (the "License"); you may not use the software except in compliance with the License.

The text of the Microsoft Reciprocal License (MS-RL) can be found online at:
 http://opensource.org/licenses/ms-rl


Microsoft Reciprocal License (MS-RL)

This license governs use of the accompanying software. If you use the software, you accept this license. If you do not accept the license, do not use the software.

1. Definitions
 The terms "reproduce," "reproduction," "derivative works," and "distribution" have the same meaning here as under U.S. copyright law.
 A "contribution" is the original software, or any additions or changes to the software.
 A "contributor" is any person that distributes its contribution under this license.
 "Licensed patents" are a contributor's patent claims that read directly on its contribution.

2. Grant of Rights
 (A) Copyright Grant- Subject to the terms of this license, including the license conditions and limitations in section 3, each contributor grants you a non-exclusive, worldwide, royalty-free copyright license to reproduce its contribution, prepare derivative works of its contribution, and distribute its contribution or any derivative works that you create.
 (B) Patent Grant- Subject to the terms of this license, including the license conditions and limitations in section 3, each contributor grants you a non-exclusive, worldwide, royalty-free license under its licensed patents to make, have made, use, sell, offer for sale, import, and/or otherwise dispose of its contribution in the software or derivative works of the contribution in the software.

3. Conditions and Limitations
 (A) Reciprocal Grants- For any file you distribute that contains code from the software (in source code or binary format), you must provide recipients the source code to that file along with a copy of this license, which license will govern that file. You may license other files that are entirely your own work and do not contain code from the software under any terms you choose.
 (B) No Trademark License- This license does not grant you rights to use any contributors' name, logo, or trademarks.
 (C) If you bring a patent claim against any contributor over patents that you claim are infringed by the software, your patent license from such contributor to the software ends automatically.
 (D) If you distribute any portion of the software, you must retain all copyright, patent, trademark, and attribution notices that are present in the software.
 (E) If you distribute any portion of the software in source code form, you may do so only under this license by including a complete copy of this license with your distribution. If you distribute any portion of the software in compiled or object code form, you may only do so under a license that complies with this license.
 (F) The software is licensed "as-is." You bear the risk of using it. The contributors give no express warranties, guarantees or conditions. You may have additional consumer rights under your local laws which this license cannot change. To the extent permitted under your local laws, the contributors exclude the implied warranties of merchantability, fitness for a particular purpose and non-infringement.
```

### B. Microsoft Edge WebView2 Runtime Bootstrapper
- **Included File:** `MicrosoftEdgeWebview2Setup.exe` (embedded in MSI Binary table)
- **File Version:** `1.3.315.17`
- **SHA-256 Checksum:** `CF2F2445FEF0C5C14E372045F84998771EEB8BB01D59BE4AFD00692FA160082E`
- **Nature of Component:** Proprietary installer utility provided by Microsoft Corporation.
- **Redistribution Documentation:** Distributed in accordance with official Microsoft guidance:  
  [Distribute a WebView2 app and the WebView2 Runtime](https://learn.microsoft.com/en-us/microsoft-edge/webview2/concepts/distribution)
- **Distribution Boundary:** The installer embeds solely the lightweight bootstrapper executable (~1.89 MB). The full Evergreen WebView2 Runtime (~150 MB) is **not embedded**; if the host system lacks WebView2, the bootstrapper connects to Microsoft CDN servers during setup to install the runtime.
- **Licensing Disclaimer:** The bootstrapper is a proprietary Microsoft component and is not open source. Authenticode digital signatures verify file integrity and publisher authenticity, but do not substitute for official vendor licensing terms.

---

## 5. Informational Exclusions

To prevent inaccurate or misleading attribution claims, the following components are explicitly identified as **excluded from Windows binary redistribution obligations**:

1. **`option-ext@0.2.0` (MPL-2.0):**
   - `option-ext` appears in `Cargo.lock` as a transitive dependency through `dirs-sys`.
   - Inspection of `dirs-sys-0.5.0` source code establishes that `option-ext` is called exclusively in `src/xdg_user_dirs.rs`, which is conditional on `#[cfg(all(unix, not(any(target_os = "macos", target_os = "ios"))))]`.
   - On Windows, `dirs-sys` compiles `target_windows` using native Win32 APIs with zero call sites to `option-ext`.
   - Because the call path is Unix-only, `option-ext` is uncalled and unlinked in Windows release builds. It is strongly supported as not redistributed in `deskpilot.exe`, and its MPL-2.0 license text is not included as a required binary redistribution notice.
2. **Build-Only HTML Parsing Crates (`cssparser`, `cssparser-macros`, `dtoa-short`, `selectors`):**
   - These MPL-2.0 crates appear in the Cargo graph through `dom_query` (an optional dependency of `tauri-utils`).
   - DeskPilot embeds pre-built production HTML/JS assets directly via `tauri-codegen-assets` without HTML DOM parsing. These crates contribute zero object code to `deskpilot.exe` and are not redistributed.
3. **Build & Development Tooling:**
   - Development dependencies such as Vite, `lightningcss` (Vite CSS minifier), `lru-cache`, Vitest, JSDOM, TypeScript, and the WiX build executables (`candle.exe`, `light.exe`) operate solely during development, testing, and packaging. They are not bundled into shipped application artifacts.
4. **Project Iconography (`icon.png`):**
   - The provenance of `src-tauri/icons/icon.png` remains unresolved in repository commit history. No original authorship or third-party license is claimed in this document pending formal owner declaration.
