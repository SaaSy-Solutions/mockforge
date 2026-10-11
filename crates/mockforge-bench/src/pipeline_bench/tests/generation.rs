//! Generation regression cases.

use super::*;

#[test]
fn targets_keep_query_ports_and_ipv6_but_omit_fragments() {
    assert_eq!(
        parse_http_target("http://[::1]:8123/upload?x=1#fragment").unwrap(),
        ("::1".into(), 8123, "/upload?x=1".into())
    );
    assert_eq!(
        parse_http_target("http://localhost?x=1").unwrap(),
        ("localhost".into(), 80, "/?x=1".into())
    );
    assert!(parse_http_target("http://user:password@localhost/").is_err());
}

#[tokio::test]
async fn multipart_mime_and_methods_reject_header_injection_before_connecting() {
    for file_type in [
        "text/plain\r\nX-Injected: yes",
        "not-a-mime",
        "image/png/extra",
        "",
    ] {
        assert!(run_with_multipart_content_type(
            socket_config(1, BodyKind::Multipart, 1024, Duration::from_millis(1)),
            file_type,
        )
        .await
        .is_err());
    }
    let mut cfg = socket_config(1, BodyKind::Json, 1024, Duration::from_millis(1));
    cfg.method = "POST\r\nX-Injected: yes".into();
    assert!(run(cfg).await.is_err());
    for size in ["NaN", "inf", "1e100GB", "-1"] {
        assert!(parse_body_size(size).is_err());
    }
}

#[test]
fn early_responses_are_reported_without_unsigned_underflow() {
    let result = PipelineBenchResult {
        responses_received: 1,
        ..Default::default()
    };
    assert!(render_report(&result).contains("before their request bodies were fully written"));
}

#[test]
fn nine_hundred_gigabyte_bodies_allocate_only_a_bounded_chunk() {
    for kind in [
        BodyKind::Json,
        BodyKind::Xml,
        BodyKind::UrlEncoded,
        BodyKind::Multipart,
    ] {
        let mut body = StreamBody::new(kind, 900_000_000_000).unwrap();
        assert_eq!(body.total_len(), 900_000_000_000);
        for _ in 0..3 {
            assert!(body.next_chunk().unwrap().len() <= 64 * 1024);
        }
    }
}

#[test]
fn body_size_parses_suffixes() {
    assert_eq!(parse_body_size("4096").unwrap(), 4096);
    assert_eq!(parse_body_size("500KB").unwrap(), 500_000);
    assert_eq!(parse_body_size("1MB").unwrap(), 1_000_000);
    assert_eq!(parse_body_size("2GB").unwrap(), 2_000_000_000);
    assert_eq!(parse_body_size("1KiB").unwrap(), 1024);
    assert_eq!(parse_body_size("1.5KB").unwrap(), 1500);
    assert!(parse_body_size("abc").is_err());
    assert!(parse_body_size("0").is_err());
}

#[test]
fn body_kinds_parse() {
    assert_eq!(parse_body_kind("json").unwrap(), BodyKind::Json);
    assert_eq!(
        parse_body_kind("application/x-www-form-urlencoded").unwrap(),
        BodyKind::UrlEncoded
    );
    assert!(parse_body_kind("grpc").is_err());
}

#[test]
fn stream_body_is_exact_size_and_well_formed() {
    for (kind, size) in [
        (BodyKind::Json, 1000u64),
        (BodyKind::Xml, 2048u64),
        (BodyKind::UrlEncoded, 777u64),
        (BodyKind::Multipart, 50_000u64),
    ] {
        let mut body = StreamBody::new(kind, size).unwrap();
        let mut total = 0usize;
        let mut assembled: Vec<u8> = Vec::new();
        while let Some(chunk) = body.next_chunk() {
            total += chunk.len();
            if assembled.len() < 512 {
                assembled.extend_from_slice(&chunk);
            }
        }
        assert_eq!(total as u64, size, "{kind:?} must produce exactly {size}");
        assert!(body.exhausted());
        match kind {
            BodyKind::Json => {
                assert!(assembled.starts_with(b"{\""));
            }
            BodyKind::Xml => assert!(assembled.starts_with(b"<root>")),
            BodyKind::UrlEncoded => {}
            BodyKind::Multipart => assert!(assembled.starts_with(b"--mockforge-pipeline")),
        }
    }
}

#[test]
fn json_body_roundtrips_through_parser() {
    let size = 4096u64;
    let mut body = StreamBody::new(BodyKind::Json, size).unwrap();
    let mut full: Vec<u8> = Vec::with_capacity(size as usize);
    while let Some(chunk) = body.next_chunk() {
        full.extend_from_slice(&chunk);
    }
    assert_eq!(full.len() as u64, size);
    let v: serde_json::Value = serde_json::from_slice(&full).expect("well-formed JSON");
    assert!(v.get("data").and_then(|d| d.as_str()).is_some());
}

#[test]
fn form_body_roundtrips_as_a_nonempty_value() {
    let mut body = StreamBody::new(BodyKind::UrlEncoded, 1024).unwrap();
    let mut full = Vec::new();
    while let Some(chunk) = body.next_chunk() {
        full.extend(chunk);
    }
    assert_eq!(full.len(), 1024);
    let query = std::str::from_utf8(&full).unwrap();
    let url = reqwest::Url::parse(&format!("http://localhost/?{query}")).unwrap();
    let pairs: Vec<_> = url.query_pairs().collect();
    assert_eq!(pairs.len(), 1);
    assert_eq!(pairs[0].0, "data");
    assert_eq!(pairs[0].1.len(), 1019);
}

#[test]
fn too_small_body_rejected() {
    assert!(StreamBody::new(BodyKind::Json, 4).is_err());
}

#[test]
fn target_parses() {
    let (h, p, path) = parse_http_target("http://localhost:3000/upload").unwrap();
    assert_eq!((h.as_str(), p, path.as_str()), ("localhost", 3000, "/upload"));
    let (h, p, path) = parse_http_target("http://example.com").unwrap();
    assert_eq!((h.as_str(), p, path.as_str()), ("example.com", 80, "/"));
    assert!(parse_http_target("https://x/").is_err());
}
