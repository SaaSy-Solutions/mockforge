use chrono::{Duration as ChronoDuration, Utc};
use once_cell::sync::OnceCell;
use rand::{rng, Rng};
use regex::Regex;
use serde_json::Value;
use std::sync::Arc;

/// Expand templating tokens in a JSON value recursively.
pub fn expand_tokens(v: &Value) -> Value {
    match v {
        Value::String(s) => Value::String(expand_str(s)),
        Value::Array(a) => Value::Array(a.iter().map(expand_tokens).collect()),
        Value::Object(o) => {
            let mut map = serde_json::Map::new();
            for (k, vv) in o {
                map.insert(k.clone(), expand_tokens(vv));
            }
            Value::Object(map)
        }
        _ => v.clone(),
    }
}

/// Expand templating tokens in a string.
pub fn expand_str(input: &str) -> String {
    // Basic replacements first (fast paths)
    let mut out = input.replace("{{uuid}}", &uuid::Uuid::new_v4().to_string());
    out = out.replace("{{now}}", &Utc::now().to_rfc3339());

    // now±Nd (days), now±Nh (hours), now±Nm (minutes), now±Ns (seconds)
    out = replace_now_offset(&out);

    // Randoms
    if out.contains("{{rand.int}}") {
        let n: i64 = rng().random_range(0..=1_000_000);
        out = out.replace("{{rand.int}}", &n.to_string());
    }
    if out.contains("{{rand.float}}") {
        let n: f64 = rng().random();
        out = out.replace("{{rand.float}}", &format!("{:.6}", n));
    }
    out = replace_randint_ranges(&out);

    // Faker tokens (can be disabled for determinism)
    let faker_enabled = std::env::var("MOCKFORGE_FAKE_TOKENS")
        .map(|v| v == "1" || v.eq_ignore_ascii_case("true"))
        .unwrap_or(true);
    if faker_enabled {
        out = replace_faker_tokens(&out);
    }

    out
}

fn replace_faker_tokens(input: &str) -> String {
    // If a provider is registered (e.g., from mockforge-data), use it; else fallback
    if let Some(provider) = FAKER_PROVIDER.get() {
        return replace_with_provider(input, provider.as_ref());
    }
    replace_with_fallback(input)
}

fn replace_with_provider(input: &str, p: &dyn FakerProvider) -> String {
    let mut out = input.to_string();
    let map = [
        ("{{faker.uuid}}", p.uuid()),
        ("{{faker.email}}", p.email()),
        ("{{faker.name}}", p.name()),
        ("{{faker.address}}", p.address()),
        ("{{faker.phone}}", p.phone()),
        ("{{faker.company}}", p.company()),
        ("{{faker.url}}", p.url()),
        ("{{faker.ip}}", p.ip()),
        ("{{faker.color}}", p.color()),
        ("{{faker.word}}", p.word()),
        ("{{faker.sentence}}", p.sentence()),
        ("{{faker.paragraph}}", p.paragraph()),
    ];
    for (pat, val) in map {
        if out.contains(pat) {
            out = out.replace(pat, &val);
        }
    }
    out
}

fn replace_with_fallback(input: &str) -> String {
    let mut out = input.to_string();
    if out.contains("{{faker.uuid}}") {
        out = out.replace("{{faker.uuid}}", &uuid::Uuid::new_v4().to_string());
    }
    if out.contains("{{faker.email}}") {
        let user: String = (0..8).map(|_| (b'a' + (rng().random::<u8>() % 26)) as char).collect();
        let dom: String = (0..6).map(|_| (b'a' + (rng().random::<u8>() % 26)) as char).collect();
        out = out.replace("{{faker.email}}", &format!("{}@{}.example", user, dom));
    }
    if out.contains("{{faker.name}}") {
        let firsts = ["Alex", "Sam", "Taylor", "Jordan", "Casey", "Riley"];
        let lasts = ["Smith", "Lee", "Patel", "Garcia", "Kim", "Brown"];
        let fi: i64 = rng().random_range(0..firsts.len() as i64);
        let li: i64 = rng().random_range(0..lasts.len() as i64);
        out = out
            .replace("{{faker.name}}", &format!("{} {}", firsts[fi as usize], lasts[li as usize]));
    }
    out
}

// Provider wiring (optional)
static FAKER_PROVIDER: OnceCell<Arc<dyn FakerProvider + Send + Sync>> = OnceCell::new();

pub trait FakerProvider {
    fn uuid(&self) -> String {
        uuid::Uuid::new_v4().to_string()
    }
    fn email(&self) -> String {
        format!("user{}@example.com", rng().random_range(1000..=9999))
    }
    fn name(&self) -> String {
        "Alex Smith".to_string()
    }
    fn address(&self) -> String {
        "1 Main St".to_string()
    }
    fn phone(&self) -> String {
        "+1-555-0100".to_string()
    }
    fn company(&self) -> String {
        "Example Inc".to_string()
    }
    fn url(&self) -> String {
        "https://example.com".to_string()
    }
    fn ip(&self) -> String {
        "192.168.1.1".to_string()
    }
    fn color(&self) -> String {
        "blue".to_string()
    }
    fn word(&self) -> String {
        "word".to_string()
    }
    fn sentence(&self) -> String {
        "A sample sentence.".to_string()
    }
    fn paragraph(&self) -> String {
        "A sample paragraph.".to_string()
    }
}

pub fn register_faker_provider(provider: Arc<dyn FakerProvider + Send + Sync>) {
    let _ = FAKER_PROVIDER.set(provider);
}

fn replace_randint_ranges(input: &str) -> String {
    // Supports {{randInt a b}} and {{rand.int a b}}
    let re = Regex::new(r"\{\{\s*(?:randInt|rand\.int)\s+(-?\d+)\s+(-?\d+)\s*\}\}").unwrap();
    let mut s = input.to_string();
    loop {
        let mat = re.captures(&s);
        if let Some(caps) = mat {
            let a: i64 = caps.get(1).unwrap().as_str().parse().unwrap_or(0);
            let b: i64 = caps.get(2).unwrap().as_str().parse().unwrap_or(100);
            let (lo, hi) = if a <= b { (a, b) } else { (b, a) };
            let n: i64 = rng().random_range(lo..=hi);
            s = re.replace(&s, n.to_string()).to_string();
        } else {
            break;
        }
    }
    s
}

fn replace_now_offset(input: &str) -> String {
    // {{ now+1d }}, {{now-2h}}, {{now+30m}}, {{now-10s}}
    let re = Regex::new(r"\{\{\s*now\s*([+-])\s*(\d+)\s*([smhd])\s*\}\}").unwrap();
    re.replace_all(input, |caps: &regex::Captures| {
        let sign = caps.get(1).unwrap().as_str();
        let amount: i64 = caps.get(2).unwrap().as_str().parse().unwrap_or(0);
        let unit = caps.get(3).map(|m| m.as_str()).unwrap_or("d");
        let dur = match unit {
            "s" => ChronoDuration::seconds(amount),
            "m" => ChronoDuration::minutes(amount),
            "h" => ChronoDuration::hours(amount),
            _ => ChronoDuration::days(amount),
        };
        let ts = if sign == "+" {
            Utc::now() + dur
        } else {
            Utc::now() - dur
        };
        ts.to_rfc3339()
    })
    .to_string()
}
