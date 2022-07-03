use crate::Client;
use braque::{scramble, BlockSize};
use eyre::{eyre, Result, WrapErr};
use image::{io::Reader as ImageReader, DynamicImage};
use once_cell::sync::Lazy;
use regex::Regex;
use std::{borrow::Cow, io::Cursor};
use url::Url;

/// Match the page number in the URL.
pub static PAGE_NUMBER: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"i0*(?P<number>[0-9]+)\..{3,4}$")
        .expect("invalid page number regex")
});

/// An episode page.
pub struct Page {
    /// Image URL.
    url: Url,
    /// Image number in the episode.
    number: u16,
}

// https://cdn.fr.piccoma.com/308/9957/eeQoB4cdy4szeVhesSEpa2Sf7J9yoJM5dWFB5Zc/i00016.jpg?credential=\u0026expires=1656892800\u0026signature=YnD0viyplpST8e25GxQDzoirnKI%3D\u0026q=Q9IXT44J6FDRRZB3KFSBJ7

impl TryFrom<Url> for Page {
    type Error = eyre::Report;

    fn try_from(value: Url) -> Result<Self, Self::Error> {
        let captures = PAGE_NUMBER
            .captures(value.path())
            .ok_or_else(|| eyre!("page number not found"))?;
        let number = captures
            .name("number")
            .expect("capture group 'number'")
            .as_str()
            .parse()
            .expect("valid number"); // must be valid thanks to regex.

        Ok(Self { url: value, number })
    }
}

impl Page {
    /// Compute the page's scrambling seed.
    fn compute_seed(&self) -> Result<Vec<u8>> {
        let mut key = self.get_key().ok_or_else(|| eyre!("get key"))?;
        let pivot = self.compute_pivot(key.len()).context("compute pivot")?;

        // Split the key at `pivot` and stitch it back to get the seed.
        let mut seed = key.split_off(pivot);
        seed.extend(key);

        Ok(seed)
    }

    /// Returns the scrambling base key.
    fn get_key(&self) -> Option<Vec<u8>> {
        // Provided through the parameter `q`.
        let q = Cow::from("q");
        self.url.query_pairs().find_map(|(key, value)| {
            (key == q).then(|| value.into_owned().into_bytes())
        })
    }

    /// Computes and returns the key's pivot.
    fn compute_pivot(&self, keylen: usize) -> Result<usize> {
        // "checksum" provided through the parameter `expires`.
        let expires = Cow::from("expires");

        // First, compute the sum of "checksum" digits.
        let checksum = self
            .url
            .query_pairs()
            .find_map(|(key, value)| (key == expires).then(|| value))
            .and_then(|checksum| {
                checksum.chars().fold(Some(0), |acc, ch| {
                    ch.to_digit(10).and_then(|x| acc.map(|sum| sum + x))
                })
            })
            .ok_or_else(|| eyre!("invalid checksum"))?;

        // Then, make sure the checksum stay in range.
        let checksum = (checksum as usize) % keylen;

        // Finally, compute the pivot (the checksumth byte from the end).
        // Note the modulus to handle checksum=0.
        Ok((keylen - checksum) % keylen)
    }
}

/// Iterator on an episode's pages.
pub struct PageIterator {
    /// Client to retrieve the pages.
    client: Client,
    /// Are those page scrambled?
    use_scrambling: bool,
    /// Pages list, correctly ordered.
    pages: Vec<Page>,
    /// Scrambling block size.
    block_size: BlockSize,
    /// Reusable buffer to download the images.
    buffer: Vec<u8>,
}

impl PageIterator {
    pub fn new(
        client: Client,
        mut pages: Vec<Page>,
        use_scrambling: bool,
    ) -> Self {
        // Make sure the pages are correctly ordered.
        // i.e. from last to first, since we iter/pop from the end.
        pages.sort_unstable_by(|a, b| b.number.cmp(&a.number));

        Self {
            client,
            use_scrambling,
            pages,
            // Block size is constant across the whole website (for now...)
            block_size: BlockSize::try_from(50).expect("valid block size"),
            buffer: Vec::new(),
        }
    }
}

impl Iterator for PageIterator {
    type Item = Result<DynamicImage>;

    fn next(&mut self) -> Option<Self::Item> {
        self.pages.pop().map(|page| {
            // Download the image.
            self.buffer.clear();
            self.client
                .get_image(&page.url, &mut self.buffer)
                .with_context(|| format!("download image from {}", page.url))?;

            // Decode it.
            let image = ImageReader::new(Cursor::new(&self.buffer))
                .with_guessed_format()
                .with_context(|| {
                    format!("determine image format from {}", page.url)
                })?
                .decode()
                .with_context(|| format!("decode image from {}", page.url))?;

            // Fix scrambling if necessary.
            if self.use_scrambling {
                let seed = page.compute_seed().with_context(|| {
                    format!("compute scrambling seed for {}", page.url)
                })?;

                return Ok(scramble(&image, self.block_size, &seed));
            }

            Ok(image)
        })
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        (self.pages.len(), Some(self.pages.len()))
    }
}

impl ExactSizeIterator for PageIterator {
    fn len(&self) -> usize {
        self.pages.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn compute_seed1() {
        // Trace episode 1.
        let url = "http://foo.com?expires=1656547200&q=IH7SKRE4KR9FHBRB81GVIX";
        let page = Page {
            url: Url::parse(url).expect("valid URL"),
            number: 0,
        };
        let expected = b"KR9FHBRB81GVIXIH7SKRE4";

        let res = page.compute_seed().expect("seed");

        assert_eq!(&res, expected);
    }

    #[test]
    fn compute_seed2() {
        // Trace episode 2.
        let url = "http://foo.com?expires=1656547200&q=266A59RRIVEPVNF7KSBYZ4";
        let page = Page {
            url: Url::parse(url).expect("valid URL"),
            number: 0,
        };
        let expected = b"IVEPVNF7KSBYZ4266A59RR";

        let res = page.compute_seed().expect("seed");

        assert_eq!(&res, expected);
    }

    #[test]
    fn compute_seed3() {
        // Spy x Family volume 1
        let url = "http://foo.com?expires=1656547200&q=PQ5I0CDCTBSLV030DAZSA1";
        let page = Page {
            url: Url::parse(url).expect("valid URL"),
            number: 0,
        };
        let expected = b"TBSLV030DAZSA1PQ5I0CDC";

        let res = page.compute_seed().expect("seed");

        assert_eq!(&res, expected);
    }

    #[test]
    fn missing_key() {
        let url = "http://foo.com?expires=1656547200&p=PQ5I0CDCTBSLV030DAZSA1";
        let page = Page {
            url: Url::parse(url).expect("valid URL"),
            number: 0,
        };

        let res = page.compute_seed();

        assert!(res.is_err());
    }

    #[test]
    fn missing_pivot() {
        let url = "http://foo.com?pivot=1656547200&q=PQ5I0CDCTBSLV030DAZSA1";
        let page = Page {
            url: Url::parse(url).expect("valid URL"),
            number: 0,
        };

        let res = page.compute_seed();

        assert!(res.is_err());
    }

    #[test]
    fn invalid_pivot() {
        let url = "http://foo.com?expires=42ftw&q=PQ5I0CDCTBSLV030DAZSA1";
        let page = Page {
            url: Url::parse(url).expect("valid URL"),
            number: 0,
        };

        let res = page.compute_seed();

        assert!(res.is_err());
    }
}
