//! piconbiere - Scrape and download media from Piccoma/ピッコマ

// Lints {{{

#![deny(
    nonstandard_style,
    rust_2018_idioms,
    future_incompatible,
    rustdoc::all,
    rustdoc::missing_crate_level_docs,
    missing_docs,
    unreachable_pub,
    unsafe_code,
    unused,
    unused_import_braces,
    unused_lifetimes,
    unused_qualifications,
    variant_size_differences,
    warnings,
    clippy::all,
    clippy::pedantic,
    clippy::clone_on_ref_ptr,
    clippy::exit,
    clippy::filetype_is_file,
    clippy::float_cmp_const,
    clippy::lossy_float_literal,
    clippy::mem_forget,
    clippy::panic,
    clippy::pattern_type_mismatch,
    clippy::rest_pat_in_fully_bound_structs,
    clippy::unneeded_field_pattern,
    clippy::verbose_file_reads,
    clippy::dbg_macro,
    clippy::let_underscore_must_use,
    clippy::todo,
    clippy::unwrap_used,
    clippy::use_debug
)]
#![allow(
    // The 90’s called and wanted their charset back :p
    clippy::non_ascii_literal,
)]

// }}}

use clap::Parser;
use eyre::{ensure, eyre, Result, WrapErr};
use indicatif::{MultiProgress, ProgressBar, ProgressStyle};
use piconbiere::{fs, termio, Client, Media, MediaType, Serie, SerieID};
use std::{
    collections::{HashMap, HashSet},
    io::{Cursor, Write},
    path::{Path, PathBuf},
    thread,
};
use zip::{write::FileOptions, ZipWriter};

fn main() -> Result<()> {
    let opts = Opts::parse();
    let client = Client::new(opts.retry);

    // If a username is provided, try to login.
    if let Some(ref email) = opts.user {
        let password = rpassword::prompt_password("Your password: ")
            .context("read password")?;
        client
            .login(email, &password)
            .with_context(|| format!("login as {email}"))?;
    }

    // Fetch serie info and media list.
    let serie =
        Serie::new(&client, opts.serie, opts.r#type).context("get serie")?;

    // Create output directory, if necessary.
    let destination = [opts.output, fs::sanitize_name(serie.title())]
        .iter()
        .collect::<PathBuf>();
    fs::mkdir_p(&destination).context("create serie directory")?;

    download(&client, &destination, &serie, opts.r#type, &opts.number)
        .with_context(|| format!("download serie {}", opts.serie))?;

    Ok(())
}

fn download(
    client: &Client,
    destination: &Path,
    serie: &Serie,
    media_type: MediaType,
    selection: &[u16],
) -> Result<()> {
    let media_list =
        compute_media_list(serie.media(), media_type, selection, destination)?;

    // Setup the progress bars (for media and pages).
    println!("Downloading {}", serie.title());
    let progress_bars = MultiProgress::new();
    let media_pb = progress_bars.add(ProgressBar::new(media_list.len() as u64));
    media_pb.set_style(
        ProgressStyle::default_bar()
            .template("{msg:10}    [{bar:40.cyan/blue}] {pos:>4}/{len:4}")
            .progress_chars("##-"),
    );
    media_pb.set_message(media_type.to_string());
    let page_pb = progress_bars.add(ProgressBar::new(
        media_list
            .iter()
            .map(|media| u64::from(media.page_count()))
            .sum(),
    ));
    setup_page_progress_bar(&page_pb);
    thread::spawn(move || {
        // Must be spawned in a dedicated thread to move forward/update.
        progress_bars.join().expect("wait for progress bars");
    });

    // Download every page of every (available) media o/
    for media in media_list {
        download_pages(client, media, destination, &page_pb)
            .with_context(|| format!("download {}", media.title()))?;
        media_pb.inc(1);
    }

    page_pb.finish();
    media_pb.finish();

    Ok(())
}

/// Downloads the specified media pages as CBZ.
fn download_pages(
    client: &Client,
    media: &Media,
    directory: &Path,
    progress_bar: &ProgressBar,
) -> Result<()> {
    let title = media.title();
    let mut buf = Vec::new();

    // Download every image and make a CBZ out of them, all in-memory.
    {
        let mut cbz = ZipWriter::new(Cursor::new(&mut buf));
        let options = FileOptions::default();

        // Add the media directory in the archive.
        cbz.add_directory(title, options)
            .context("create media directory")?;

        // XXX: we can use enumerate because the pages are sorted.
        for (i, page) in media.fetch_pages(client.clone())?.enumerate() {
            let filename = format!("{:03}.webp", i);
            let page =
                page.with_context(|| format!("fetch page {}", filename))?;

            // Encode the image as lossless WebP.
            let encoder = webp::Encoder::from_image(&page)
                .map_err(|err| eyre!("encode {}: {}", filename, err))?;
            let bytes = encoder.encode_lossless();

            // Add the page in the archive.
            cbz.start_file(&format!("{title}/{filename}"), options)
                .with_context(|| format!("add image {}", filename))?;
            cbz.write_all(&bytes)
                .with_context(|| format!("write image {}", filename))?;

            progress_bar.inc(1);
        }
        cbz.finish().expect("close in-memory zip");
    }

    // Atomic write of the CBZ.
    let path = [directory, media.filename().as_path()]
        .into_iter()
        .collect::<PathBuf>();
    fs::atomic_write(&path, &buf).context("save CBZ")
}

/// Configures the progress bar for the pages.
fn setup_page_progress_bar(progress_bar: &ProgressBar) {
    progress_bar.set_style(
        ProgressStyle::default_bar()
            .template("{msg:10}    [{bar:40.cyan/blue}] {pos:>4}/{len:4} ETA: {eta_precise}")
            .progress_chars("##-"),
    );
    progress_bar.set_message("pages");
}

/// Computes the list of media to download.
///
/// Filter out (and log) unavailable and already downloaded media, only retains
/// the selection.
fn compute_media_list<'a>(
    media: impl Iterator<Item = &'a Media>,
    media_type: MediaType,
    selection: &[u16],
    destination: &Path,
) -> Result<Vec<&'a Media>> {
    // Index media by their number.
    let media_list = media.fold(HashMap::new(), |mut acc, m| {
        acc.insert(m.number(), m);
        acc
    });
    // Check that the selected media exists.
    let selection_len = selection.len();
    let selection = selection.iter().fold(HashSet::new(), |mut acc, number| {
        if media_list.contains_key(number) {
            acc.insert(number);
        } else {
            termio::print_err(&format!("{media_type} {number} not found"));
        }
        acc
    });
    ensure!(
        selection.len() == selection_len,
        "some selected {media_type} doesn't exists"
    );
    // Skip unselected media and filter out unavailable/already downloaded ones.
    Ok(media_list
        .into_values()
        .filter(|media| {
            if !selection.contains(&media.number()) {
                return false;
            }
            if media.is_present_at(destination) {
                termio::print_ok(&format!(
                    "{media_type} {} already downloaded",
                    media.number()
                ));
                return false;
            }
            if !media.is_available() {
                termio::print_warn(&format!(
                    "{media_type} {} not available",
                    media.number()
                ));
                return false;
            }

            true
        })
        .collect())
}

/// CLI options.
#[derive(Parser)]
#[clap(author, version, about)]
pub struct Opts {
    /// Path to the output directory.
    #[clap(short, long, default_value = ".")]
    output: PathBuf,

    /// Serie ID.
    #[clap(short, long)]
    serie: SerieID,

    /// Episode or volume number.
    #[clap(short, long)]
    number: Vec<u16>,

    /// Media type to download.
    #[clap(short, long = "type", arg_enum, value_parser, default_value_t = MediaType::Episode)]
    r#type: MediaType,

    /// Email to login.
    #[clap(short, long)]
    user: Option<String>,

    /// Max number of retry for HTTP requests.
    #[clap(long, default_value_t = 3)]
    retry: u8,
}
