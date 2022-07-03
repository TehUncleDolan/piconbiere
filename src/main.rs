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
use eyre::{bail, ensure, eyre, Result, WrapErr};
use indicatif::{MultiProgress, ProgressBar, ProgressStyle};
use piconbiere::{self, fs, termio, Client, Episode, Serie, SerieID};
use std::{
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

    // Fetch serie info and episodes list.
    let serie = Serie::new(&client, opts.serie).context("get serie")?;
    let destination = [opts.output, fs::sanitize_name(serie.title())]
        .iter()
        .collect::<PathBuf>();

    // Create output directory, if necessary.
    fs::mkdir_p(&destination).context("create serie directory")?;

    // Download the pages.
    match opts.episode {
        Some(episode) => {
            download_one(&client, &destination, &serie, episode).with_context(
                || format!("download serie {} episode {episode}", opts.serie),
            )?;
        },
        None => {
            download_all(&client, &destination, &serie)
                .with_context(|| format!("download serie {}", opts.serie))?;
        },
    }

    Ok(())
}

/// Downloads a single episode.
fn download_one(
    client: &Client,
    destination: &Path,
    serie: &Serie,
    episode_number: u16,
) -> Result<()> {
    // Select the requested episode.
    let episode = match serie
        .episodes()
        .find(|episode| episode.number() == episode_number)
    {
        Some(episode) => episode,
        None => bail!("episode not found"),
    };
    // Check its availability (and if its already present).
    ensure!(
        episode.is_available(),
        "episode not available (not logged in?)"
    );
    if episode.is_present_at(destination) {
        termio::print_ok("episode already downloaded: nothing to do");
        return Ok(());
    }

    // Setup the progress bar.
    println!("Downloading {} {:03}", serie.title(), episode.number());
    let progress_bar = ProgressBar::new(episode.page_count().into());
    setup_page_progress_bar(&progress_bar);

    // Download o/
    download_episode(client, episode, destination, &progress_bar)
        .with_context(|| format!("download episode {:03}", episode.number()))?;

    progress_bar.finish();

    Ok(())
}

/// Downloads an entire serie.
fn download_all(
    client: &Client,
    destination: &Path,
    serie: &Serie,
) -> Result<()> {
    // Filter out (and log) unavailable and already downloaded episodes.
    let episodes = serie
        .episodes()
        .filter(|episode| {
            if !episode.is_available() {
                termio::print_warn(&format!(
                    "episode {:03} not available (not logged in?)",
                    episode.number()
                ));
                return false;
            }
            if episode.is_present_at(destination) {
                termio::print_ok(&format!(
                    "episode {:03} already downloaded",
                    episode.number()
                ));
                return false;
            }

            true
        })
        .collect::<Vec<_>>();

    // Setup the progress bars (for episodes and pages).
    println!("Downloading {}", serie.title());
    let progress_bars = MultiProgress::new();
    let episode_pb = progress_bars.add(ProgressBar::new(episodes.len() as u64));
    episode_pb.set_style(
        ProgressStyle::default_bar()
            .template("{msg:10}    [{bar:40.cyan/blue}] {pos:>4}/{len:4}")
            .progress_chars("##-"),
    );
    episode_pb.set_message("episodes");
    let page_pb = progress_bars.add(ProgressBar::new(
        episodes
            .iter()
            .map(|episode| u64::from(episode.page_count()))
            .sum(),
    ));
    setup_page_progress_bar(&page_pb);
    thread::spawn(move || {
        // Must be spawned in a dedicated thread to move forward/update.
        progress_bars.join().expect("wait for progress bars");
    });

    // Download every page of every (available) episode o/
    for episode in episodes {
        download_episode(client, episode, destination, &page_pb).with_context(
            || format!("download episode {:03}", episode.number()),
        )?;
        episode_pb.inc(1);
    }

    page_pb.finish();
    episode_pb.finish();

    Ok(())
}

/// Downloads the specified episode as CBZ.
fn download_episode(
    client: &Client,
    episode: &Episode,
    directory: &Path,
    progress_bar: &ProgressBar,
) -> Result<()> {
    let title = episode.title();
    let mut buf = Vec::new();

    // Download every image and make a CBZ out of them, all in-memory.
    {
        let mut cbz = ZipWriter::new(Cursor::new(&mut buf));
        let options = FileOptions::default();

        // Add the episode directory in the archive.
        cbz.add_directory(&title, options)
            .context("create episode directory")?;

        // XXX: we can use enumerate because the pages are sorted.
        for (i, page) in episode.fetch_pages(client.clone())?.enumerate() {
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
    let path = [directory, episode.filename().as_path()]
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

    /// Episode number.
    #[clap(short, long)]
    episode: Option<u16>,

    /// Email to login.
    #[clap(short, long)]
    user: Option<String>,

    /// Max number of retry for HTTP requests.
    #[clap(long, default_value_t = 3)]
    retry: u8,
}
