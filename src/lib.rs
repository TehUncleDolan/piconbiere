pub mod fs;
pub mod termio;

mod client;
mod episode;
mod models;
mod page;
mod selectors;
mod serie;

pub use client::Client;
pub use episode::{Episode, EpisodeID};
pub use page::PageIterator;
pub use serie::{Serie, SerieID};

use selectors::NEXT_DATA_SELECTOR;
