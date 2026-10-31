//! Real Qdrant contract. Run through scripts/ci/test_qdrant_storage.py.
#![cfg(feature = "qdrant")]
use mockforge_data::rag::{
    engine::DocumentChunk,
    storage::{SearchMethod, SearchParams, StorageFactory},
};
use std::{collections::HashMap, path::PathBuf};

fn config(collection: &str) -> HashMap<String, String> {
    let url = std::env::var("QDRANT_TEST_URL").expect("owned Qdrant fixture required");
    let parsed = url::Url::parse(&url).unwrap();
    assert!(matches!(parsed.host_str(), Some("127.0.0.1" | "localhost" | "::1" | "[::1]")));
    HashMap::from([
        ("provider".into(), "qdrant".into()),
        ("url".into(), url),
        ("collection".into(), collection.into()),
        ("dimensions".into(), "3".into()),
        (
            "api_key".into(),
            std::env::var("QDRANT_TEST_API_KEY").expect("fixture key required"),
        ),
    ])
}
fn chunk(id: &str, doc: &str, vector: Vec<f32>, position: usize) -> DocumentChunk {
    DocumentChunk {
        id: id.into(),
        document_id: doc.into(),
        embedding: vector,
        content: format!("content for {id}"),
        metadata: HashMap::new(),
        position,
        length: 10,
    }
}
fn backup_path() -> PathBuf {
    PathBuf::from(std::env::var("QDRANT_TEST_WORK_DIR").expect("owned fixture directory required"))
        .join(format!("{}.snapshot", uuid::Uuid::new_v4()))
}

#[tokio::test]
#[ignore = "requires the owned Qdrant fixture script"]
async fn qdrant_lifecycle_and_snapshot() {
    let cfg = config(&format!("lifecycle_{}", uuid::Uuid::new_v4().simple()));
    let mut initializers = tokio::task::JoinSet::new();
    for _ in 0..8 {
        let config = cfg.clone();
        initializers.spawn(async move {
            StorageFactory::create_vector_db(config).unwrap().store_chunks(Vec::new()).await
        });
    }
    while let Some(result) = initializers.join_next().await {
        result.unwrap().unwrap();
    }
    let storage = StorageFactory::create_vector_db(cfg.clone()).unwrap();
    let mut a = chunk("a/任意", "doc-a", vec![0.8, 0.2, 0.0], 0);
    a.metadata.insert("tag.value".into(), "wanted".into());
    let mut b = chunk("b", "doc-b", vec![1.0, 0.0, 0.0], 0);
    b.metadata.insert("tag.value".into(), "wrong".into());
    b.metadata.insert("other".into(), "wanted".into());
    let mut chunks = vec![a.clone(), b];
    for i in 0..70 {
        chunks.push(chunk(&format!("paged-{i}"), "doc-c", vec![0.0, 1.0, 0.0], i));
    }
    storage.store_chunks(chunks).await.unwrap();
    storage.store_chunks(vec![a.clone()]).await.unwrap();
    assert_eq!(storage.get_total_chunks().await.unwrap(), 72, "upsert must not duplicate IDs");
    assert_eq!(storage.search_similar(&[1.0, 0.0, 0.0], 1).await.unwrap()[0].id, "b");
    let params = SearchParams {
        top_k: 1,
        threshold: 0.0,
        document_filter: Some("doc-a".into()),
        ..Default::default()
    };
    assert_eq!(storage.search_with_params(&[1.0, 0.0, 0.0], params).await.unwrap()[0].id, a.id);
    let params = SearchParams {
        top_k: 1,
        threshold: 0.0,
        metadata_filter: Some(a.metadata.clone()),
        include_metadata: false,
        ..Default::default()
    };
    let hits = storage.search_with_params(&[1.0, 0.0, 0.0], params).await.unwrap();
    assert_eq!(hits[0].id, a.id);
    assert!(hits[0].metadata.is_empty());
    let loaded = storage.get_chunk(&a.id).await.unwrap().unwrap();
    assert_eq!(loaded.embedding, a.embedding);
    assert_eq!(loaded.metadata, a.metadata);
    assert!(storage.get_chunk("absent").await.unwrap().is_none());
    let pages = storage.get_chunks_by_document("doc-c").await.unwrap();
    assert_eq!(pages.len(), 70);
    assert_eq!(pages[69].position, 69);
    assert_eq!(storage.list_documents().await.unwrap(), vec!["doc-a", "doc-b", "doc-c"]);
    let stats = storage.get_stats().await.unwrap();
    assert_eq!(stats.total_chunks, 72);
    assert_eq!(stats.total_documents, 3);
    assert_eq!(stats.backend_type, "qdrant");
    storage.optimize().await.unwrap();
    let backup = backup_path();
    storage.create_backup(backup.to_str().unwrap()).await.unwrap();
    assert!(std::fs::metadata(&backup).unwrap().len() > 0);
    assert_eq!(storage.delete_document("doc-c").await.unwrap(), 70);
    assert!(storage.delete_chunk("b").await.unwrap());
    assert!(!storage.delete_chunk("b").await.unwrap());
    storage.clear().await.unwrap();
    assert_eq!(storage.get_total_chunks().await.unwrap(), 0);
    storage.restore_backup(backup.to_str().unwrap()).await.unwrap();
    assert_eq!(storage.get_total_chunks().await.unwrap(), 72);
    drop(storage);
    let reopened = StorageFactory::create_vector_db(cfg.clone()).unwrap();
    assert_eq!(reopened.get_chunk(&a.id).await.unwrap().unwrap().content, a.content);
    assert_eq!(reopened.search_similar(&[1.0, 0.0, 0.0], 1).await.unwrap()[0].id, "b");
    let mut wrong = cfg.clone();
    wrong.insert("dimensions".into(), "4".into());
    assert!(StorageFactory::create_vector_db(wrong)
        .unwrap()
        .get_total_chunks()
        .await
        .is_err());
    let mut wrong = cfg;
    wrong.insert("api_key".into(), "incorrect-fixture-key".into());
    let error = StorageFactory::create_vector_db(wrong)
        .unwrap()
        .get_total_chunks()
        .await
        .unwrap_err()
        .to_string();
    assert!(error.contains("HTTP 401") || error.contains("HTTP 403"));
    assert!(!error.contains("incorrect-fixture-key"));
    assert_eq!(reopened.get_total_chunks().await.unwrap(), 72);
}

#[tokio::test]
#[ignore = "requires the owned Qdrant fixture script"]
async fn qdrant_metrics_and_vector_validation() {
    for (distance, method) in [
        ("Cosine", SearchMethod::Cosine),
        ("Euclid", SearchMethod::Euclidean),
        ("Dot", SearchMethod::DotProduct),
        ("Manhattan", SearchMethod::Manhattan),
    ] {
        let mut cfg = config(&format!("metric_{}", uuid::Uuid::new_v4().simple()));
        cfg.insert("distance".into(), distance.into());
        let storage = StorageFactory::create_vector_db(cfg).unwrap();
        storage
            .store_chunks(vec![
                chunk("near", "doc", vec![1.0, 0.0, 0.0], 0),
                chunk("far", "doc", vec![0.0, 1.0, 0.0], 1),
            ])
            .await
            .unwrap();
        assert_eq!(storage.search_similar(&[1.0, 0.0, 0.0], 1).await.unwrap()[0].id, "near");
        let threshold = if matches!(distance, "Euclid" | "Manhattan") {
            0.1
        } else {
            0.9
        };
        assert_eq!(
            storage
                .search_with_params(
                    &[1.0, 0.0, 0.0],
                    SearchParams {
                        threshold,
                        search_method: method,
                        ..Default::default()
                    }
                )
                .await
                .unwrap()
                .len(),
            1
        );
        assert!(storage.store_chunks(vec![chunk("bad", "doc", vec![1.0], 2)]).await.is_err());
        assert!(storage.search_similar(&[f32::NAN, 0.0, 0.0], 1).await.is_err());
        assert_eq!(storage.get_total_chunks().await.unwrap(), 2);
    }
}

#[tokio::test]
#[ignore = "fixture script runs seed, restarts the server, then runs verify"]
async fn qdrant_persistence_across_restart() {
    let collection =
        std::env::var("QDRANT_TEST_RESTART_COLLECTION").expect("restart fixture required");
    let storage = StorageFactory::create_vector_db(config(&collection)).unwrap();
    match std::env::var("QDRANT_TEST_PHASE").as_deref() {
        Ok("seed") => storage
            .store_chunks(vec![chunk("persisted/任意", "durable", vec![1.0, 0.0, 0.0], 0)])
            .await
            .unwrap(),
        Ok("verify") => {
            assert_eq!(storage.get_total_chunks().await.unwrap(), 1);
            let chunk = storage.get_chunk("persisted/任意").await.unwrap().unwrap();
            assert_eq!(chunk.document_id, "durable");
            assert_eq!(storage.search_similar(&[1.0, 0.0, 0.0], 1).await.unwrap()[0].id, chunk.id);
        }
        _ => panic!("run the owned Qdrant fixture script for restart acceptance"),
    }
}
