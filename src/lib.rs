pub mod fs;
pub mod termio;

mod client;
mod media;
mod models;
mod page;
mod selectors;
mod serie;

pub use client::Client;
pub use media::{Media, MediaType};
pub use page::PageIterator;
pub use serie::{Serie, SerieID};

use selectors::NEXT_DATA_SELECTOR;
