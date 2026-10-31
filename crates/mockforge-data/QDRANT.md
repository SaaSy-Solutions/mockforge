# Qdrant RAG storage

Enable the optional backend in a Rust application:

```toml
mockforge-data = { path = "../mockforge/crates/mockforge-data", features = ["qdrant"] }
```

The backend is currently an Unreleased source feature; use the repository path
until a release containing this change is published. Qdrant's REST query API
is used for server-side search. Acceptance uses Qdrant 1.19.2.

```rust,no_run
use mockforge_data::rag::storage::StorageFactory;
use std::collections::HashMap;

let config = HashMap::from([
    ("provider".into(), "qdrant".into()),
    ("url".into(), "http://127.0.0.1:6333".into()),
    ("collection".into(), "my_documents".into()),
    ("dimensions".into(), "1536".into()),
]);
let storage = StorageFactory::create_vector_db(config)?;
# Ok::<(), mockforge_data::Error>(())
```

Optional configuration:

| Key | Meaning |
| --- | --- |
| `api_key` | Qdrant API key, sent as a sensitive header. Load it from your application's credential configuration. |
| `distance` | `Cosine` (default), `Euclid`, `Dot`, or `Manhattan`. |
| `timeout_seconds` | End-to-end request timeout, 1–3600 seconds; default 60. Increase it for large snapshots. |

Construction does no network I/O. The first operation creates the collection if
it is absent, checks its dimensions and metric, and creates a document-ID payload
index. A collection is dedicated to this backend's point format. Incompatible
collections and authentication failures return errors; there is no memory fallback.
Configure HTTPS for remote authenticated deployments. Redirects are disabled.

Chunk IDs can be arbitrary strings. Stable SHA-256-derived UUID point IDs make
upserts idempotent; the original ID, vector, content and metadata round-trip in the
payload. Validate all embeddings before writing: dimensions must match and every
value must be finite. Each serialized chunk is limited to 1 MiB; writes use batches
of 32. Separate batches are not a cross-batch transaction.

Search runs in Qdrant. Document and metadata filters apply before top-k selection.
Metadata keys containing dots work as literal keys. `search_with_params` must use
the collection's metric; its threshold follows Qdrant's score semantics (minimum
similarity for Cosine/Dot, maximum distance for Euclid/Manhattan). `search_similar`
applies no threshold. `top_k=0` returns no results; the maximum is 10,000.
Document reads page through Qdrant and return chunks in document-position order.
Document listing scans only document IDs, retaining only distinct IDs locally.
Deletion counts describe the matching set counted immediately before deletion;
concurrent writers can change that set.

`create_backup(path)` streams a native Qdrant snapshot to a temporary file and
renames it after a successful download. Its temporary server snapshot is removed
on success or failure; a cleanup failure is reported in tracing. `restore_backup`
uploads a native snapshot to Qdrant with snapshot priority. Restore only snapshots
from a compatible collection, and coordinate restore with other writers. Native
snapshots follow Qdrant's server-version and deployment-mode compatibility rules.
They are not interchangeable with the file backend's JSON snapshots.

`get_total_chunks` uses an exact server count. Statistics list documents by paging;
remote disk-space and index-size measurements are unavailable and reported as zero.
The statistics timestamp is the observation time. Qdrant optimizes segments in the
background; `optimize` checks optimizer health rather than forcing compaction.

Run real-server acceptance, including backup/restore and a container restart:

```bash
python3 scripts/ci/test_qdrant_storage.py
```

This requires Cargo and a local Unix-socket Docker context. It starts an
authenticated, loopback-only Qdrant container with a uniquely labelled persistent
volume, runs the ignored integration tests, restarts Qdrant to verify persistence,
and removes only its own labelled resources. `--receipt path.json` writes a
credential-free result. Ordinary unit tests need no Qdrant server:

```bash
cargo test --locked -p mockforge-data --lib
cargo test --locked -p mockforge-data --features qdrant --lib
```

The REST contract is documented in [Qdrant's OpenAPI specification](https://github.com/qdrant/qdrant/blob/master/docs/redoc/master/openapi.json).
