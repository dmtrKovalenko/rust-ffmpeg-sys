[![ffmpeg-sys-next on crates.io](https://img.shields.io/crates/v/ffmpeg-sys-next?cacheSeconds=3600)](https://crates.io/crates/ffmpeg-sys-next)
[![build](https://github.com/zmwangx/rust-ffmpeg-sys/actions/workflows/build.yml/badge.svg?branch=master)](https://github.com/zmwangx/rust-ffmpeg-sys/actions)

This is a fork of the abandoned [ffmpeg-sys](https://github.com/meh/rust-ffmpeg-sys) crate. You can find this crate as [ffmpeg-sys-next](https://crates.io/crates/ffmpeg-sys-next) on crates.io.

This crate contains low level bindings to FFmpeg. You're probably interested in the high level bindings instead: [ffmpeg-next](https://github.com/zmwangx/rust-ffmpeg).

A word on versioning: major and minor versions track major and minor versions of FFmpeg, e.g. 4.2.x of this crate has been updated to support the 4.2.x series of FFmpeg. Patch level is reserved for bug fixes of this crate and does not track FFmpeg patch versions.

## ffmpeg-sys-fframes: prebuilt FFmpeg

`ffmpeg-sys-fframes` is `ffmpeg-sys-next` with prebuilt FFmpeg binaries, used by
[fframes](https://github.com/dmtrKovalenko/fframes). With the `build` and `static` features the
build script first downloads a static FFmpeg matching the target and the enabled features from
the [`binaries-<version>` release](https://github.com/dmtrKovalenko/rust-ffmpeg-sys/releases)
and only compiles FFmpeg from source when there is none. Archives are built by
`.github/workflows/binaries.yml` for macOS (arm64, x86_64) and Linux (x86_64, arm64) with the
codec sets fframes uses. External codec libraries (x264, x265) still come from the system.

- `FFMPEG_FORCE_BUILD=1` always compiles from source (so do `FFMPEG_MARCH`/`FFMPEG_MTUNE`, the
  archives are compiled with `build-portable`).
- `FFMPEG_BINARIES_URL` overrides the download URL template (`{tag}`, `{key}`; `file://` works).
- `FFMPEG_BINARIES_CACHE` is where downloads are kept (default: the user cache directory).
- `FFMPEG_BINARIES_EXPORT_DIR` makes a source build write its archive there.

## Feature flags

In addition to feature flags declared in `Cargo.toml`, this crate performs various compile-time version and feature detections and exposes the results in additional flags. These flags are briefly documented below; run `cargo build -vv` to view more details.

- `ffmpeg_<x>_<y>` flags (new in v4.3.2), e.g. `ffmpeg_4_4`, indicating the FFmpeg installation being compiled against is at least version `<x>.<y>`. Currently available:

  - `ffmpeg_3_0`
  - `ffmpeg_3_1`
  - `ffmpeg_3_2`
  - `ffmpeg_3_3`
  - `ffmpeg_3_4`
  - `ffmpeg_4_0`
  - `ffmpeg_4_1`
  - `ffmpeg_4_2`
  - `ffmpeg_4_3`
  - `ffmpeg_4_4`
  - `ffmpeg_5_0`
  - `ffmpeg_5_1`
  - `ffmpeg_6_0`
  - `ffmpeg_6_1`
  - `ffmpeg_7_0`
  - `ffmpeg_7_1`
  - `ffmpeg_8_0`
  - `ffmpeg_8_1`
  - `ffmpeg_9_0`

- `avcodec_version_greater_than_<x>_<y>` (new in v4.3.2), e.g., `avcodec_version_greater_than_58_90`. The name should be self-explanatory.

- `ff_api_<feature>`, e.g. `ff_api_vaapi`, corresponding to whether their respective uppercase deprecation guards evaluate to true.

- `ff_api_<feature>_is_defined`, e.g. `ff_api_vappi_is_defined`, similar to above except these are enabled as long as the corresponding deprecation guards are defined.
