use super::*;

fn config() -> HashMap<String, String> {
    HashMap::from([
        ("url".into(), "http://127.0.0.1:6333".into()),
        ("collection".into(), "test".into()),
        ("dimensions".into(), "3".into()),
    ])
}

#[test]
fn rejects_invalid_configuration_without_network_io() {
    for (key, value) in [
        ("url", "file:///tmp"),
        ("url", "https://user:secret@example.com"),
        ("url", "https://example.com/?token=secret"),
        ("collection", "../other"),
        ("dimensions", "0"),
        ("dimensions", "65537"),
        ("distance", "unknown"),
        ("timeout_seconds", "0"),
        ("api_key", "invalid\nheader"),
    ] {
        let mut cfg = config();
        cfg.insert(key.into(), value.into());
        let error = QdrantStorage::new(&cfg).err().expect("invalid configuration accepted");
        assert!(!error.to_string().contains("secret"));
    }
}

#[test]
fn arbitrary_chunk_ids_are_stable_and_distinct() {
    assert_eq!(QdrantStorage::point_id("chunk/one"), QdrantStorage::point_id("chunk/one"));
    assert_ne!(QdrantStorage::point_id("chunk/one"), QdrantStorage::point_id("chunk/two"));
    assert!(uuid::Uuid::parse_str(&QdrantStorage::point_id("任意の識別子")).is_ok());
}

#[test]
fn invalid_vectors_are_rejected() {
    let storage = QdrantStorage::new(&config()).unwrap();
    for vector in [
        vec![],
        vec![1.0, 2.0],
        vec![f32::NAN, 0.0, 1.0],
        vec![0.0, f32::INFINITY, 1.0],
    ] {
        assert!(storage.validate_vector(&vector).is_err());
    }
    storage.validate_vector(&[1.0, 0.0, 0.0]).unwrap();
}

#[test]
fn proxy_prefix_and_path_segments_are_preserved() {
    let mut cfg = config();
    cfg.insert("url".into(), "https://example.com/qdrant/".into());
    let storage = QdrantStorage::new(&cfg).unwrap();
    assert_eq!(
        storage.endpoint(&["snapshots", "name/with/slashes"]).as_str(),
        "https://example.com/qdrant/collections/test/snapshots/name%2Fwith%2Fslashes"
    );
}

#[test]
fn factory_selects_feature_enabled_qdrant_without_network_io() {
    let mut cfg = config();
    cfg.insert("provider".into(), "qdrant".into());
    assert!(StorageFactory::create_vector_db(cfg).is_ok());
}
