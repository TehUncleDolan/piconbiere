//! HTTP client to interact with Piccoma website.

use eyre::{Result, WrapErr};
use kuchiki::traits::*;
use rand::prelude::*;
use serde::de::DeserializeOwned;
use std::{io::Read, thread, time::Duration};
use url::Url;

/// Use the website URL as referer.
const REFERER: &str = "https://piccoma.com/fr";
/// User agent to reduce our visibility (trying at least...)
const USER_AGENT: &str =
    "Mozilla/5.0 (Windows NT 10.0; Win64; x64; rv:92.0) Gecko/20100101 Firefox/92.0";

/// A simple HTTP client, handle retry and delay.
#[derive(Clone)]
pub struct Client {
    /// HTTP client.
    agent: ureq::Agent,
    /// Delay between each request.
    delay: Duration,
    /// Max number of retry for each request.
    retry: u8,
}

impl Client {
    /// Initialize a new client.
    pub fn new(retry: u8) -> Self {
        Self {
            agent: ureq::builder().user_agent(USER_AGENT).build(),
            /// 1s ought to be enough to avoid detection...
            delay: Duration::from_secs(1),
            retry,
        }
    }

    /// Tests if the client is logged in as a user.
    pub fn is_logged_in(&self) -> bool {
        self.agent
            .cookie_store()
            .contains("piccoma.com", "/", "access_token")
    }

    /// Logs into the website using the specified credential.
    pub fn login(&self, email: &str, password: &str) -> Result<()> {
        let request = self
            .agent
            .request("POST", "https://piccoma.com/fr/api/auth/signin")
            .set("accept", "text/html");

        request
            .send_json(ureq::json!({
                "email": email,
                "password": password,
                "redirect": REFERER,
            }))
            .context("login")?;

        Ok(())
    }

    /// Retrieves and parses the HTML at `url`.
    pub fn get_html(&self, url: &Url) -> Result<kuchiki::NodeRef> {
        let request = self
            .agent
            .request_url("GET", url)
            .set("accept", "text/html");

        let response = self.call(request).context("get HTML")?;
        let html = response.into_string().context("read HTML")?;

        Ok(kuchiki::parse_html().one(html))
    }

    /// Calls `url` and parses the JSON response.
    pub fn get_json<T>(&self, url: &Url) -> Result<T>
    where
        T: DeserializeOwned,
    {
        let request = self
            .agent
            .request_url("GET", url)
            .set("accept", "application/json");
        let response = self.call(request).context("get JSON")?;

        serde_json::from_reader(response.into_reader()).context("read JSON")
    }

    /// Downloads the specified page in the given buffer.
    pub fn get_image(&self, url: &Url, buf: &mut Vec<u8>) -> Result<()> {
        let request =
            self.agent.request_url("GET", url).set("accept", "image/*");

        let response = self.call(request).context("get image")?;
        response
            .into_reader()
            .read_to_end(buf)
            .context("read image")?;

        Ok(())
    }

    /// Executes a request and handle retries.
    fn call(&self, request: ureq::Request) -> Result<ureq::Response> {
        // Wait a bit, don't overload the site.
        let mut rng = rand::thread_rng();
        let jiffy = Duration::from_millis(rng.gen_range(0u32..1000).into());
        thread::sleep(self.delay + jiffy);

        // Set referer to looks kinda legit.
        let request = request.set("Referer", REFERER);

        let mut i = 0;
        loop {
            i += 1;

            let res = request.clone().call();

            if let Err(ureq::Error::Status(code, ref response)) = res {
                // If we got a retryable error, we try again!
                if is_request_retryable(code) && i <= self.retry {
                    let delay = self.retry_delay(response);

                    thread::sleep(delay);
                    continue;
                }
            }

            return res.context("HTTP request failed");
        }
    }

    /// Computes the delay to wait before retrying a failed request.
    fn retry_delay(&self, response: &ureq::Response) -> Duration {
        response
            .header("retry-after")
            .and_then(|h| h.parse::<u64>().ok())
            .map_or(self.delay, Duration::from_secs)
    }
}

/// Tests if request failed with a retryable error.
fn is_request_retryable(http_status: u16) -> bool {
    // 429 is Too Many Requests
    (500..=599).contains(&http_status) || http_status == 429
}
