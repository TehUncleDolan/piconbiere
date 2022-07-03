use crate::{fs, models, Client, PageIterator, SerieID, NEXT_DATA_SELECTOR};
use eyre::{bail, ensure, eyre, Result, WrapErr};
use kuchiki::traits::*;
use once_cell::sync::Lazy;
use regex::Regex;
use std::{
    fmt,
    path::{Path, PathBuf},
    str::FromStr,
};
use url::Url;

/// Match the episode title's prefix (#<number>).
pub static TITLE_PREFIX: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"^#\d+ ").expect("invalid episode title prefix"));

/// A media episode.
pub struct Episode {
    /// Episode title (if any).
    title: Option<String>,
    /// Episode ID
    episode_id: EpisodeID,
    /// Serie ID.
    serie_id: SerieID,
    /// Episode number.
    episode_number: u16,
    /// Episode access type.
    access: AccessType,
    /// Number of pages.
    page_count: u16,
}

impl Episode {
    /// Returns the episode viewer URL.
    fn viewer_url(&self) -> Url {
        Url::parse(&format!(
            "https://piccoma.com/fr/viewer/{}/{}",
            self.serie_id, self.episode_id
        ))
        .expect("valid episode URL")
    }

    /// Returns the episode ID.
    pub fn id(&self) -> EpisodeID {
        self.episode_id
    }

    /// Returns the episode number.
    pub fn number(&self) -> u16 {
        self.episode_number
    }

    /// Returns the episode title.
    ///
    /// If the episode has no title, a generic one is generated from the episode
    /// number.
    pub fn title(&self) -> String {
        self.title.as_ref().map_or_else(
            || format!("Episode {:03}", self.episode_number),
            |title| format!("{:03} - {}", self.episode_number, title),
        )
    }

    /// Returns the number of pages.
    pub fn page_count(&self) -> u16 {
        self.page_count
    }

    /// Tests if the episode is accessible by the current account.
    pub fn is_available(&self) -> bool {
        matches!(
            self.access,
            AccessType::Free | AccessType::TemporaryFree | AccessType::Paid
        )
    }

    /// Tests if the episode is already present on disk.
    pub fn is_present_at(&self, path: &Path) -> bool {
        let filepath = [path, &self.filename()].iter().collect::<PathBuf>();

        filepath.is_file()
    }

    /// Returns the episode filename.
    pub fn filename(&self) -> PathBuf {
        let mut filename = fs::sanitize_name(&self.title());
        filename.set_extension("cbz");
        filename
    }

    /// Retrieves pages info and return a page iterator
    pub fn fetch_pages(&self, client: Client) -> Result<PageIterator> {
        // Fetch the viewer page.
        let html = client
            .get_html(&self.viewer_url())
            .context("get viewer page")?;

        // Extract and parse the JSON payload.
        let payload = NEXT_DATA_SELECTOR
            .filter(html.descendants().elements())
            .next()
            .ok_or_else(|| eyre!("look for PageIteratorepisode __NEXT_DATA__"))?
            .text_contents();
        let data = serde_json::from_str::<models::episode::NextData>(&payload)
            .context("parse episode __NEXT_DATA__")?
            .props
            .page_props
            .initial_state
            .viewer
            .p_data;

        // Make sure we got the expected number of pages!
        ensure!(
            data.img.len() == usize::from(self.page_count),
            "expected {} page, got {}",
            self.page_count,
            data.img.len(),
        );

        // Return the iterator to download the images.
        let pages = data
            .img
            .into_iter()
            .map(|img| img.path.try_into())
            .collect::<Result<Vec<_>, _>>()
            .context("invalid page URL")?;
        Ok(PageIterator::new(client, pages, data.is_scrambled))
    }
}

impl TryFrom<models::serie::Episode> for Episode {
    type Error = eyre::Report;

    fn try_from(value: models::serie::Episode) -> Result<Self, Self::Error> {
        Ok(Self {
            title: (!value.title.is_empty())
                .then(|| TITLE_PREFIX.replace(&value.title, "").into_owned()),
            episode_id: value.id.into(),
            serie_id: value.product_id.into(),
            access: value.use_type.parse().context("parse access type")?,
            episode_number: value.order_value,
            page_count: value.page_count,
        })
    }
}

/// Episode ID on Piccoma.
#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub struct EpisodeID(u32);

impl fmt::Display for EpisodeID {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl From<u32> for EpisodeID {
    fn from(value: u32) -> Self {
        Self(value)
    }
}

impl FromStr for EpisodeID {
    type Err = eyre::Report;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        value.parse::<u32>().context("invalid episode ID").map(Self)
    }
}

/// Episode access type.
#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub enum AccessType {
    /// Free episode, anyone can read it.
    Free,
    /// Episode temporarily unlocked through the WaitUntilFree (WUF).
    TemporaryFree,
    /// Episode that can be read if the user uses its WUF.
    WaitUntilFree,
    /// Episode that must be bought.
    Paywalled,
    /// Episode already bought.
    Paid,
}

impl FromStr for AccessType {
    type Err = eyre::Report;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        Ok(match &value[..2] {
            "FR" => Self::Free,
            "RD" => Self::TemporaryFree,
            "WF" => Self::WaitUntilFree,
            "PM" => Self::Paywalled,
            "AB" => Self::Paid,
            _ => bail!("{value} is not a valid access type"),
        })
    }
}
