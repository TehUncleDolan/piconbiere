use once_cell::sync::Lazy;

/// Select `<script id="__NEXT_DATA__">`
pub(crate) static NEXT_DATA_SELECTOR: Lazy<kuchiki::Selectors> =
    Lazy::new(|| {
        kuchiki::Selectors::compile("script#__NEXT_DATA__")
            .expect("invalid serie JSON payload selector")
    });
