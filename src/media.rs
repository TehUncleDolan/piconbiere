use crate::{fs, models, Client, PageIterator, SerieID, NEXT_DATA_SELECTOR};
use clap::ArgEnum;
use eyre::{bail, ensure, eyre, Result, WrapErr};
use kuchiki::traits::*;
use once_cell::sync::Lazy;
use regex::Regex;
use serde::Deserialize;
use std::{
    fmt,
    path::{Path, PathBuf},
    str::FromStr,
};
use url::Url;

/// Match the episode title's prefix (#<number>).
pub static EPISODE_TITLE_PREFIX: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"^#\d+ ").expect("invalid episode title prefix"));

// -----------------------------------------------------------------------------

/// Media access type.
#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub enum AccessType {
    /// Free media, anyone can read it.
    Free,
    /// Media temporarily unlocked through the `WaitUntilFree` (WUF).
    TemporaryFree,
    /// Media that can be read if the user uses its WUF.
    WaitUntilFree,
    /// Media that must be bought.
    Paywalled,
    /// Media already bought.
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

// -----------------------------------------------------------------------------

/// Type of media.
#[derive(Debug, Clone, Copy, Eq, PartialEq, ArgEnum, Deserialize)]
pub enum MediaType {
    /// An episode or a chapter of the serie.
    #[serde(rename = "E")]
    Episode,
    /// A complete volume of the serie.
    #[serde(rename = "V")]
    Volume,
}

impl fmt::Display for MediaType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{}",
            match self {
                Self::Episode => "episode",
                Self::Volume => "volume",
            }
        )
    }
}

// -----------------------------------------------------------------------------

/// A media (an episode or a volume).
#[derive(Debug)]
pub struct Media {
    /// Title.
    title: String,
    /// Media ID
    id: MediaID,
    /// Serie ID.
    serie_id: SerieID,
    /// Number in the serie.
    number: u16,
    /// Access type.
    access: AccessType,
    /// Number of pages.
    page_count: u16,
}

impl Media {
    /// Returns the episode ID.
    pub fn id(&self) -> MediaID {
        self.id
    }

    /// Returns the episode number.
    pub fn number(&self) -> u16 {
        self.number
    }

    /// Returns the media title.
    pub fn title(&self) -> &str {
        &self.title
    }

    /// Returns the number of pages.
    pub fn page_count(&self) -> u16 {
        self.page_count
    }

    /// Tests if the media is accessible by the current account.
    pub fn is_available(&self) -> bool {
        matches!(
            self.access,
            AccessType::Free | AccessType::TemporaryFree | AccessType::Paid
        )
    }

    /// Tests if the media is already present on disk.
    pub fn is_present_at(&self, path: &Path) -> bool {
        let filepath = [path, &self.filename()].iter().collect::<PathBuf>();

        filepath.is_file()
    }

    /// Returns the media filename.
    pub fn filename(&self) -> PathBuf {
        let mut filename = fs::sanitize_name(self.title());
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
        let data = serde_json::from_str::<models::viewer::NextData>(&payload)
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

    fn viewer_url(&self) -> Url {
        Url::parse(&format!(
            "https://piccoma.com/fr/viewer/{}/{}",
            self.serie_id, self.id,
        ))
        .expect("valid media URL")
    }
}

impl TryFrom<models::serie::Media> for Media {
    type Error = eyre::Report;

    fn try_from(value: models::serie::Media) -> Result<Self, Self::Error> {
        let number = match value.media_type {
            MediaType::Episode => value.order_value,
            MediaType::Volume => value.volume,
        };
        let title = match value.media_type {
            MediaType::Episode => {
                if value.title.is_empty() {
                    format!("Episode {:03}", number)
                } else {
                    format!(
                        "{:03} - {}",
                        number,
                        EPISODE_TITLE_PREFIX.replace(&value.title, "")
                    )
                }
            },
            MediaType::Volume => format!("Tome {:02}", number),
        };

        Ok(Self {
            title,
            id: value.id.into(),
            serie_id: value.product_id.into(),
            access: value.use_type.parse().context("parse access type")?,
            number,
            page_count: value.page_count,
        })
    }
}

// -----------------------------------------------------------------------------

/// Media ID on Piccoma.
#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub struct MediaID(u32);

impl fmt::Display for MediaID {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl From<u32> for MediaID {
    fn from(value: u32) -> Self {
        Self(value)
    }
}

impl FromStr for MediaID {
    type Err = eyre::Report;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        value.parse::<u32>().context("invalid media ID").map(Self)
    }
}
