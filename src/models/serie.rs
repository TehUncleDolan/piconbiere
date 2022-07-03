//! Mininal model of the data returned by `/api/web/v3/product/<ID>/episodes`.

use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct Product {
    // Title
    pub title: String,
}

#[derive(Debug, Deserialize)]
pub struct Episode {
    // Episode ID
    pub id: u32,
    // Serie ID.
    pub product_id: u32,
    // Volume number.
    pub volume: u16,
    // Title
    pub title: String,
    // Episode order.
    pub order_value: u16,
    // Page count.
    pub page_count: u16,
    // Usage type.
    pub use_type: String,
}

// From the API {{{

#[derive(Debug, Deserialize)]
pub struct ApiResponse {
    pub data: Data,
}

#[derive(Debug, Deserialize)]
pub struct Data {
    pub product: Product,
    pub episode_list: Vec<Episode>,
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
