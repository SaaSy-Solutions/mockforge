//! Discover encoded and decoded URL userinfo and sensitive query values.
use super::classification::sensitive_name;
use std::collections::BTreeSet;

pub(super) fn collect_url_credentials(string: &str, secrets: &mut BTreeSet<String>) {
    if let Ok(url) = url::Url::parse(string) {
        for credential in [Some(url.username()), url.password()]
            .into_iter()
            .flatten()
            .filter(|s| !s.is_empty())
        {
            secrets.insert(credential.to_owned());
            if let Ok(decoded) = urlencoding::decode(credential) {
                secrets.insert(decoded.into_owned());
            }
        }
        for (key, value) in url.query_pairs() {
            if sensitive_name(&key) && !value.is_empty() {
                secrets.insert(value.into_owned());
            }
        }
    }
}
