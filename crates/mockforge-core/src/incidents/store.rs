//! In-memory and persistent storage for incidents
//!
//! This module provides both in-memory caching and database persistence for incidents.

use crate::incidents::types::{DriftIncident, IncidentQuery, IncidentStatus};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

/// In-memory store for active incidents
#[derive(Debug, Clone)]
pub struct IncidentStore {
    /// In-memory cache of incidents (indexed by ID)
    cache: Arc<RwLock<HashMap<String, DriftIncident>>>,
    /// Index by endpoint+method for fast lookup
    endpoint_index: Arc<RwLock<HashMap<String, Vec<String>>>>,
    /// Index by status for fast filtering
    status_index: Arc<RwLock<HashMap<IncidentStatus, Vec<String>>>>,
}

impl IncidentStore {
    /// Create a new incident store
    pub fn new() -> Self {
        Self {
            cache: Arc::new(RwLock::new(HashMap::new())),
            endpoint_index: Arc::new(RwLock::new(HashMap::new())),
            status_index: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Store an incident
    pub async fn store(&self, incident: DriftIncident) {
        let id = incident.id.clone();
        let endpoint_key = format!("{} {}", incident.method, incident.endpoint);
        let status = incident.status;

        // Store in cache
        {
            let mut cache = self.cache.write().await;
            cache.insert(id.clone(), incident);
        }

        // Update endpoint index
        {
            let mut index = self.endpoint_index.write().await;
            index.entry(endpoint_key).or_insert_with(Vec::new).push(id.clone());
        }

        // Update status index
        {
            let mut index = self.status_index.write().await;
            index.entry(status).or_insert_with(Vec::new).push(id);
        }
    }

    /// Get an incident by ID
    pub async fn get(&self, id: &str) -> Option<DriftIncident> {
        let cache = self.cache.read().await;
        cache.get(id).cloned()
    }

    /// Update an incident
    pub async fn update(&self, incident: DriftIncident) {
        let id = incident.id.clone();
        let old_status = {
            let cache = self.cache.read().await;
            cache.get(&id).map(|i| i.status)
        };

        // Update cache
        {
            let mut cache = self.cache.write().await;
            cache.insert(id.clone(), incident);
        }

        // Update status index if status changed
        if let Some(old_status) = old_status {
            let new_status = {
                let cache = self.cache.read().await;
                cache.get(&id).map(|i| i.status)
            };

            if let Some(new_status) = new_status {
                if old_status != new_status {
                    // Remove from old status index
                    {
                        let mut index = self.status_index.write().await;
                        if let Some(ids) = index.get_mut(&old_status) {
                            ids.retain(|x| x != &id);
                        }
                    }

                    // Add to new status index
                    {
                        let mut index = self.status_index.write().await;
                        index.entry(new_status).or_insert_with(Vec::new).push(id);
                    }
                }
            }
        }
    }

    /// Query incidents
    pub async fn query(&self, query: IncidentQuery) -> Vec<DriftIncident> {
        let cache = self.cache.read().await;
        let mut results: Vec<DriftIncident> = cache.values().cloned().collect();

        // Apply filters
        if let Some(status) = query.status {
            results.retain(|incident| incident.status == status);
        }

        if let Some(severity) = query.severity {
            results.retain(|incident| incident.severity == severity);
        }

        if let Some(ref endpoint) = query.endpoint {
            results.retain(|incident| incident.endpoint == *endpoint);
        }

        if let Some(ref method) = query.method {
            results.retain(|incident| incident.method == *method);
        }

        if let Some(incident_type) = query.incident_type {
            results.retain(|incident| incident.incident_type == incident_type);
        }

        if let Some(ref workspace_id) = query.workspace_id {
            results.retain(|incident| {
                incident.workspace_id.as_ref().map(|w| w == workspace_id).unwrap_or(false)
            });
        }

        if let Some(start_date) = query.start_date {
            results.retain(|incident| incident.detected_at >= start_date);
        }

        if let Some(end_date) = query.end_date {
            results.retain(|incident| incident.detected_at <= end_date);
        }

        // Sort by detected_at descending (newest first)
        results.sort_by(|a, b| b.detected_at.cmp(&a.detected_at));

        // Apply pagination
        let offset = query.offset.unwrap_or(0);
        let limit = query.limit.unwrap_or(100);

        results.into_iter().skip(offset).take(limit).collect()
    }

    /// Get all incidents
    pub async fn get_all(&self) -> Vec<DriftIncident> {
        let cache = self.cache.read().await;
        cache.values().cloned().collect()
    }

    /// Get incidents by status
    pub async fn get_by_status(&self, status: IncidentStatus) -> Vec<DriftIncident> {
        let status_index = self.status_index.read().await;
        let cache = self.cache.read().await;

        status_index
            .get(&status)
            .map(|ids| ids.iter().filter_map(|id| cache.get(id).cloned()).collect())
            .unwrap_or_default()
    }

    /// Remove an incident
    pub async fn remove(&self, id: &str) -> Option<DriftIncident> {
        let incident = {
            let cache = self.cache.read().await;
            cache.get(id).cloned()
        };

        if let Some(ref incident) = incident {
            let endpoint_key = format!("{} {}", incident.method, incident.endpoint);
            let status = incident.status;

            // Remove from cache
            {
                let mut cache = self.cache.write().await;
                cache.remove(id);
            }

            // Remove from endpoint index
            {
                let mut index = self.endpoint_index.write().await;
                if let Some(ids) = index.get_mut(&endpoint_key) {
                    ids.retain(|x| x != id);
                }
            }

            // Remove from status index
            {
                let mut index = self.status_index.write().await;
                if let Some(ids) = index.get_mut(&status) {
                    ids.retain(|x| x != id);
                }
            }
        }

        incident
    }

    /// Clean up resolved incidents older than specified days
    pub async fn cleanup_old_resolved(&self, retention_days: u32) {
        let cutoff = chrono::Utc::now().timestamp() - (retention_days as i64 * 86400);
        let cache = self.cache.read().await;

        let ids_to_remove: Vec<String> = cache
            .values()
            .filter(|incident| {
                incident.status == IncidentStatus::Resolved
                    || incident.status == IncidentStatus::Closed
            })
            .filter(|incident| incident.resolved_at.map(|t| t < cutoff).unwrap_or(false))
            .map(|incident| incident.id.clone())
            .collect();

        drop(cache);

        for id in ids_to_remove {
            self.remove(&id).await;
        }
    }

    /// Get count of incidents by status
    pub async fn count_by_status(&self) -> HashMap<IncidentStatus, usize> {
        let status_index = self.status_index.read().await;
        status_index.iter().map(|(status, ids)| (*status, ids.len())).collect()
    }
}

impl Default for IncidentStore {
    fn default() -> Self {
        Self::new()
    }
}
