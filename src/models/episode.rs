//! Mininal model of the data returned by viewer web page.

use serde::Deserialize;
use url::Url;

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
    pub viewer: Viewer,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Viewer {
    pub p_data: Data,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Data {
    pub is_scrambled: bool,
    pub img: Vec<Image>,
}

#[derive(Debug, Deserialize)]
pub struct Image {
    pub path: Url,
}
