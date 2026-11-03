use super::*;

impl QdrantStorage {
    /// Build storage without network I/O. Existing collections must use the
    /// configured dimensions/distance; incompatible collections are rejected.
    pub fn new(config: &HashMap<String, String>) -> Result<Self> {
        let required = |key| {
            config
                .get(key)
                .filter(|v| !v.trim().is_empty())
                .ok_or_else(|| Error::config(format!("Qdrant requires {key}")))
        };
        let base =
            url::Url::parse(required("url")?).map_err(|_| Error::config("Invalid Qdrant URL"))?;
        if !matches!(base.scheme(), "http" | "https")
            || base.host_str().is_none()
            || !base.username().is_empty()
            || base.password().is_some()
            || base.query().is_some()
            || base.fragment().is_some()
        {
            return Err(Error::config(
                "Qdrant URL requires HTTP(S), without credentials, query or fragment",
            ));
        }
        let collection = required("collection")?.clone();
        if collection.len() > 128
            || !collection.bytes().all(|c| c.is_ascii_alphanumeric() || c == b'_' || c == b'-')
        {
            return Err(Error::config(
                "Qdrant collection requires 1-128 letters, digits, underscores or hyphens",
            ));
        }
        let dimensions = required("dimensions")?
            .parse::<usize>()
            .ok()
            .filter(|n| *n > 0 && *n <= 65536)
            .ok_or_else(|| Error::config("Qdrant dimensions must be between 1 and 65536"))?;
        let distance = config.get("distance").map(String::as_str).unwrap_or("Cosine");
        if !matches!(distance, "Cosine" | "Euclid" | "Dot" | "Manhattan") {
            return Err(Error::config("Unsupported Qdrant distance"));
        }
        let timeout = config
            .get("timeout_seconds")
            .map(|v| v.parse::<u64>())
            .transpose()
            .map_err(|_| Error::config("Invalid Qdrant timeout_seconds"))?
            .unwrap_or(60);
        if !(1..=3600).contains(&timeout) {
            return Err(Error::config("Qdrant timeout_seconds must be between 1 and 3600"));
        }
        let mut headers = reqwest::header::HeaderMap::new();
        if let Some(key) = config.get("api_key") {
            let mut value = reqwest::header::HeaderValue::from_str(key)
                .map_err(|_| Error::config("Invalid Qdrant API key header"))?;
            value.set_sensitive(true);
            headers.insert("api-key", value);
        }
        let client = reqwest::Client::builder()
            .default_headers(headers)
            .redirect(reqwest::redirect::Policy::none())
            .connect_timeout(Duration::from_secs(10))
            .timeout(Duration::from_secs(timeout))
            .build()
            .map_err(|_| Error::config("Could not create Qdrant client"))?;
        Ok(Self {
            client,
            base,
            collection,
            dimensions,
            distance: distance.into(),
            ready: OnceCell::new(),
        })
    }
}
