# PiconBiere - Scrape and download media from Piccoma/ピッコマ

[![License](https://img.shields.io/badge/License-BSD%203--Clause-blue.svg)](https://opensource.org/licenses/BSD-3-Clause)

## Disclaimer

- PiconBiere was made for the sole purpose of helping users download media from Piccoma for offline consumption. This is for private use only, do not use this tool to promote piracy.
- PiconBiere only support the French version of Piccoma (at least for now)

## Overview

PiconBiere scrape images from [Piccoma website](https://piccoma.com/fr).

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
piconbiere --serie 208 --number 1
```

To download a single volume from a serie, using your account:

```text
piconbiere --serie 208 --number 1 --type volume --user foo@email.com
```

`--user` is used to login with your account in order to access the media
you've bought (you'll be prompted for your password).

`--number` can be repeated in order to download multiple episodes (or volumes)
in single run.

For example, to download the episodes 1, 3 and 8:

```test
piconbiere --serie 208 -n 1 -n 3 -n 8
```

Finally, you can download every episode of a serie with:

```text
piconbiere --serie 208 -u foo@email.com
```

Or, every volume (when `--type` is not specified it defaults to episode)::

```text
piconbiere --serie 208 -t volume -u foo@email.com
```

For more advanced options, please consult the help:

```text
piconbiere -h
```

## Credits

PiconBiere has been inspired by [Pyccoma](https://github.com/catsital/pyccoma)
and [piccoma-downloader](https://github.com/Elastic1/piccoma-downloader)
