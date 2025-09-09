//! Record/replay listing for HTTP/gRPC/WS fixtures.
use serde::Serialize;
use globwalk::GlobWalkerBuilder;

#[derive(Debug, Serialize)]
pub struct ReplayItem {
    pub protocol: String,
    pub operation_id: String,
    pub saved_at: String,
    pub path: String,
}

pub fn list_all(fixtures_root: &str) -> anyhow::Result<Vec<ReplayItem>> {
    let mut out = Vec::new();
    for proto in ["http", "grpc", "ws"] {
        let pat = format!("{root}/{proto}/**/*.json", root=fixtures_root, proto=proto);
        for entry in GlobWalkerBuilder::from_patterns(".", &[&pat]).build()? {
            let p = entry?.path().to_path_buf();
            if p.extension().map(|e| e=="json").unwrap_or(false) {
                let comps: Vec<_> = p.components().map(|c| c.as_os_str().to_string_lossy().to_string()).collect();
                let len = comps.len();
                let (op_id, ts) = if len >= 2 {
                    (comps[len-2].clone(), comps[len-1].replace(".json",""))
                } else { ("unknown".into(), "unknown".into()) };
                out.push(ReplayItem { protocol: proto.to_string(), operation_id: op_id, saved_at: ts, path: p.to_string_lossy().to_string() });
            }
        }
    }
    out.sort_by(|a,b| b.saved_at.cmp(&a.saved_at));
    Ok(out)
}
