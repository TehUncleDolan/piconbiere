# PiconBiere - Scrape and download media from Piccoma/ピッコマ

[![License](https://img.shields.io/badge/License-BSD%203--Clause-blue.svg)](https://opensource.org/licenses/BSD-3-Clause)

## Disclaimer

- PiconBiere was made for the sole purpose of helping users download media from Piccoma for offline consumption. This is for private use only, do not use this tool to promote piracy.
- PiconBiere only support the French version of Piccoma (at least for now)

## Overview

PiconBier scrape images from [Piccoma website](https://piccoma.com/fr).

## Installing

Pre-compiled binaries can be downloaded from the
[Releases](https://github.com/TehUncleDolan/piconbiere/releases/) page.

Alternatively, PiconBiere can be installed from Cargo, via the following command:

```
cargo install piconbiere
```

PiconBiere can be built from source using the latest stable or nightly Rust.
This is primarily useful for developing on PiconBiere.

```
git clone https://github.com/TehUncleDolan/piconbiere.git
cd piconbiere
cargo build --release
cp target/release/piconbiere /usr/local/bin
```

PiconBiere follows Semantic Versioning.

## Usage

PiconBiere is a command-line utility. Basic usage looks similar to the
following.

To download a single episode from a serie, using guest mode:

```text
piconbiere --serie 208 --episode 1
```

To download every episode from a serie:

```text
piconbiere --serie 208 --type episode --user foo@email.com
```

`-user` can be used to login with your account in order to access the episodes
you've bought (you'll be prompted for your password).

Or if you prefer by volume:

```text
piconbiere --serie 208 --type volume --user foo@email.com
```

For more advanced options, please consult the help:

```text
piconbiere -h
```

## Credits

PiconBiere has been inspired by [Pyccoma](https://github.com/catsital/pyccoma)
and [piccoma-downloader](https://github.com/Elastic1/piccoma-downloader)
