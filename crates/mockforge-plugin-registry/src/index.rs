//! Plugin index management

use crate::{RegistryEntry, SearchQuery, SearchResults, SortOrder};
use std::cmp::Ordering as CmpOrdering;

/// Search and sort plugins
pub fn search_plugins(entries: &[&RegistryEntry], query: &SearchQuery) -> SearchResults {
    let mut results: Vec<RegistryEntry> = entries
        .iter()
        .filter(|entry| filter_entry(entry, query))
        .map(|&e| e.clone())
        .collect();

    // Sort results
    sort_results(&mut results, &query.sort);

    // Paginate
    let total = results.len();
    let start = query.page * query.per_page;
    let end = (start + query.per_page).min(total);

    let plugins = if start < total {
        results[start..end].to_vec()
    } else {
        vec![]
    };

    SearchResults {
        plugins,
        total,
        page: query.page,
        per_page: query.per_page,
    }
}

/// Filter a single entry
fn filter_entry(entry: &RegistryEntry, query: &SearchQuery) -> bool {
    // Filter by query string
    if let Some(q) = &query.query {
        let q_lower = q.to_lowercase();
        let matches = entry.name.to_lowercase().contains(&q_lower)
            || entry.description.to_lowercase().contains(&q_lower)
            || entry.tags.iter().any(|tag| tag.to_lowercase().contains(&q_lower));

        if !matches {
            return false;
        }
    }

    // Filter by category
    if let Some(cat) = &query.category {
        if !matches_category(&entry.category, cat) {
            return false;
        }
    }

    // Filter by tags
    if !query.tags.is_empty() && !query.tags.iter().any(|tag| entry.tags.contains(tag)) {
        return false;
    }

    true
}

/// Check if categories match
fn matches_category(entry_cat: &crate::PluginCategory, query_cat: &crate::PluginCategory) -> bool {
    std::mem::discriminant(entry_cat) == std::mem::discriminant(query_cat)
}

/// Sort results by the specified order
fn sort_results(results: &mut [RegistryEntry], sort: &SortOrder) {
    results.sort_by(|a, b| match sort {
        SortOrder::Relevance => {
            // For relevance, prioritize by downloads and rating
            let score_a = a.downloads as f64 + a.rating * 1000.0;
            let score_b = b.downloads as f64 + b.rating * 1000.0;
            score_b.partial_cmp(&score_a).unwrap_or(CmpOrdering::Equal)
        }
        SortOrder::Downloads => b.downloads.cmp(&a.downloads),
        SortOrder::Rating => b.rating.partial_cmp(&a.rating).unwrap_or(CmpOrdering::Equal),
        SortOrder::Recent => b.updated_at.cmp(&a.updated_at),
        SortOrder::Name => a.name.cmp(&b.name),
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{AuthorInfo, PluginCategory};

    fn create_test_entry(name: &str, downloads: u64, rating: f64) -> RegistryEntry {
        RegistryEntry {
            name: name.to_string(),
            description: "Test".to_string(),
            version: "1.0.0".to_string(),
            versions: vec![],
            author: AuthorInfo {
                name: "Test".to_string(),
                email: None,
                url: None,
            },
            tags: vec!["test".to_string()],
            category: PluginCategory::Auth,
            downloads,
            rating,
            reviews_count: 0,
            repository: None,
            homepage: None,
            license: "MIT".to_string(),
            created_at: "2025-01-01T00:00:00Z".to_string(),
            updated_at: "2025-01-01T00:00:00Z".to_string(),
        }
    }

    #[test]
    fn test_search_and_sort() {
        let entries = vec![
            create_test_entry("plugin-a", 100, 4.0),
            create_test_entry("plugin-b", 200, 4.5),
            create_test_entry("plugin-c", 50, 5.0),
        ];

        let entry_refs: Vec<&RegistryEntry> = entries.iter().collect();

        let query = SearchQuery {
            sort: SortOrder::Downloads,
            ..Default::default()
        };

        let results = search_plugins(&entry_refs, &query);
        assert_eq!(results.plugins[0].name, "plugin-b");
        assert_eq!(results.plugins[1].name, "plugin-a");
        assert_eq!(results.plugins[2].name, "plugin-c");
    }

    #[test]
    fn test_filter_by_query() {
        let entry = create_test_entry("auth-jwt", 100, 4.0);
        let entry_ref = &entry;

        let query = SearchQuery {
            query: Some("auth".to_string()),
            ..Default::default()
        };

        assert!(filter_entry(entry_ref, &query));

        let query = SearchQuery {
            query: Some("template".to_string()),
            ..Default::default()
        };
        assert!(!filter_entry(entry_ref, &query));
    }
}
