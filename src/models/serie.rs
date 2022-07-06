//! Mininal model of the data returned by `/api/web/v3/product/<ID>/episodes`.

use crate::MediaType;
use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct Data {
    pub product: Product,
    #[serde(rename = "episode_list")]
    pub media_list: Vec<Media>,
}

#[derive(Debug, Deserialize)]
pub struct Product {
    // Title
    pub title: String,
}

#[derive(Debug, Deserialize)]
pub struct Media {
    // Episode ID
    pub id: u32,
    // Serie ID.
    pub product_id: u32,
    // Volume number.
    pub volume: u16,
    // Title
    pub title: String,
    // Media order.
    pub order_value: u16,
    // Page count.
    pub page_count: u16,
    // Usage type.
    pub use_type: String,
    // Media type
    #[serde(rename = "episode_type")]
    pub media_type: MediaType,
}

// From the API {{{

#[derive(Debug, Deserialize)]
pub struct ApiResponse {
    pub data: Data,
}

// }}}
// From the web page {{{

#[derive(Debug, Deserialize)]
pub struct NextData {
    pub props: Props,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Props {
    pub page_props: PageProps,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PageProps {
    pub initial_state: InitialState,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct InitialState {
    pub product_home: ProductHome,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProductHome {
    pub product_home: Data,
}

// }}}
