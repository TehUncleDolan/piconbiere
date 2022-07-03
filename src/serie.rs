use crate::{models, Client, Episode, NEXT_DATA_SELECTOR};
use eyre::{ensure, eyre, Result, WrapErr};
use kuchiki::traits::*;
use std::{fmt, str::FromStr};
use url::Url;

/// A media serie.
pub struct Serie {
    /// Serie title.
    title: String,
    /// Episode list.
    episodes: Vec<Episode>,
}

impl Serie {
    /// Initializes a new serie.
    pub fn new(client: &Client, id: SerieID) -> Result<Self> {
        // We have two way of extracting the list of episode:
        // - the API
        // - the embedded JSON payload
        //
        // API can only be used if you are logged in.
        // Embedded JSON payload only contains unread episode when you're logged
        // in, otherwise it's complete.
        //
        // So, if we're logged in we use the API and in guest mode we rely on
        // the JSON.
        let info = if client.is_logged_in() {
            get_info_from_api(client, id).context("get serie info from API")?
        } else {
            get_info_from_web(client, id).context("get serie info from web")?
        };

        info.try_into()
    }

    /// Returns the series title.
    pub fn title(&self) -> &str {
        &self.title
    }

    /// Returns the number of episodes.
    pub fn episodes_count(&self) -> usize {
        self.episodes.len()
    }

    /// Returns the episodes.
    pub fn episodes(
        &self,
    ) -> impl Iterator<Item = &Episode> + ExactSizeIterator + '_ {
        self.episodes.iter()
    }
}

/// Extract serie info from Piccoma API.
fn get_info_from_api(
    client: &Client,
    id: SerieID,
) -> Result<models::serie::Data> {
    let url = Url::parse(&format!("https://piccoma.com/fr/api/haribo/api/web/v3/product/{id}/episodes?episode_type=E&product_id={id}")).expect("valid serie API URL");

    Ok(client
        .get_json::<models::serie::ApiResponse>(&url)
        .context("call serie endpoint")?
        .data)
}

/// Extract serie info from Piccoma web page.
fn get_info_from_web(
    client: &Client,
    id: SerieID,
) -> Result<models::serie::Data> {
    // Fetch the serie page.
    let url =
        Url::parse(&format!("https://piccoma.com/fr/product/episode/{}", id))
            .expect("valid serie web URL");
    let html = client.get_html(&url).context("get series page")?;

    // Extract and parse the JSON payload.
    let payload = NEXT_DATA_SELECTOR
        .filter(html.descendants().elements())
        .next()
        .ok_or_else(|| eyre!("look for serie __NEXT_DATA__"))?
        .text_contents();
    let data = serde_json::from_str::<models::serie::NextData>(&payload)
        .context("parse serie __NEXT_DATA__")?;

    Ok(data
        .props
        .page_props
        .initial_state
        .product_home
        .product_home)
}

impl TryFrom<models::serie::Data> for Serie {
    type Error = eyre::Report;

    fn try_from(value: models::serie::Data) -> Result<Self, Self::Error> {
        ensure!(!value.product.title.is_empty(), "empty serie title");

        Ok(Self {
            title: value.product.title,
            episodes: value
                .episode_list
                .into_iter()
                .map(Episode::try_from)
                .collect::<Result<Vec<_>, _>>()
                .context("extract episodes")?,
        })
    }
}

/// Serie ID on Piccoma.
#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub struct SerieID(u32);

impl fmt::Display for SerieID {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl From<u32> for SerieID {
    fn from(value: u32) -> Self {
        Self(value)
    }
}

impl FromStr for SerieID {
    type Err = eyre::Report;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        value.parse::<u32>().context("invalid serie ID").map(Self)
    }
}
