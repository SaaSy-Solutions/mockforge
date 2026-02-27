//! Flamegraph generation for trace analysis
//!
//! Generates flamegraph visualizations from distributed traces to help identify
//! performance bottlenecks and understand call hierarchies.

use crate::{ReportingError, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs::File;
use std::io::Write;

/// Span data for flamegraph generation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TraceSpan {
    pub span_id: String,
    pub parent_span_id: Option<String>,
    pub operation_name: String,
    pub service_name: String,
    pub start_time: u64,
    pub duration_us: u64,
    pub tags: HashMap<String, String>,
}

/// Trace data containing multiple spans
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TraceData {
    pub trace_id: String,
    pub spans: Vec<TraceSpan>,
}

/// Flamegraph generator
pub struct FlamegraphGenerator {
    collapse_threshold_us: u64,
}

#[allow(clippy::only_used_in_recursion)]
impl FlamegraphGenerator {
    /// Create a new flamegraph generator
    pub fn new() -> Self {
        Self {
            collapse_threshold_us: 100, // Collapse spans shorter than 100μs
        }
    }

    /// Set the collapse threshold in microseconds
    pub fn with_threshold(mut self, threshold_us: u64) -> Self {
        self.collapse_threshold_us = threshold_us;
        self
    }

    /// Generate flamegraph from trace data
    pub fn generate(&self, trace: &TraceData, output_path: &str) -> Result<()> {
        // Build span hierarchy
        let hierarchy = self.build_hierarchy(trace)?;

        // Generate folded stack format
        let folded_stacks = self.generate_folded_stacks(&hierarchy, trace);

        // Write to intermediate file
        let folded_path = format!("{}.folded", output_path);
        let mut file = File::create(&folded_path)?;
        for stack in &folded_stacks {
            writeln!(file, "{}", stack)?;
        }

        // Generate SVG flamegraph
        self.generate_svg(&folded_path, output_path)?;

        Ok(())
    }

    /// Build span hierarchy from flat list
    fn build_hierarchy(&self, trace: &TraceData) -> Result<SpanNode> {
        let mut span_map: HashMap<String, &TraceSpan> = HashMap::new();
        let mut root_spans = Vec::new();

        // First pass: index all spans
        for span in &trace.spans {
            span_map.insert(span.span_id.clone(), span);
        }

        // Second pass: find roots and build tree
        for span in &trace.spans {
            if span.parent_span_id.is_none() {
                root_spans.push(span);
            }
        }

        if root_spans.is_empty() {
            return Err(ReportingError::Analysis("No root spans found in trace".to_string()));
        }

        // Use first root span as the trace root
        let root_span = root_spans[0];
        let root_node = self.build_node(root_span, &span_map, trace);

        Ok(root_node)
    }

    /// Build a span node recursively
    fn build_node(
        &self,
        span: &TraceSpan,
        _span_map: &HashMap<String, &TraceSpan>,
        trace: &TraceData,
    ) -> SpanNode {
        let mut children = Vec::new();

        // Find child spans
        for candidate in &trace.spans {
            if let Some(parent_id) = &candidate.parent_span_id {
                if parent_id == &span.span_id {
                    let child_node = self.build_node(candidate, _span_map, trace);
                    children.push(child_node);
                }
            }
        }

        SpanNode {
            span: span.clone(),
            children,
        }
    }

    /// Generate folded stack representation
    fn generate_folded_stacks(&self, root: &SpanNode, _trace: &TraceData) -> Vec<String> {
        let mut stacks = Vec::new();
        self.collect_stacks(root, String::new(), &mut stacks);
        stacks
    }

    /// Recursively collect stack traces
    fn collect_stacks(&self, node: &SpanNode, prefix: String, stacks: &mut Vec<String>) {
        let label = format!("{}::{}", node.span.service_name, node.span.operation_name);
        let current_stack = if prefix.is_empty() {
            label.clone()
        } else {
            format!("{};{}", prefix, label)
        };

        if node.children.is_empty() {
            // Leaf node - emit stack with duration
            stacks.push(format!("{} {}", current_stack, node.span.duration_us));
        } else {
            // Internal node - recurse to children
            for child in &node.children {
                self.collect_stacks(child, current_stack.clone(), stacks);
            }
        }
    }

    /// Generate SVG flamegraph from folded stacks using inferno
    fn generate_svg(&self, folded_path: &str, output_path: &str) -> Result<()> {
        use std::io::BufReader;

        let folded_file = File::open(folded_path)?;
        let reader = BufReader::new(folded_file);
        let mut output_file = File::create(output_path)?;

        let mut opts = inferno::flamegraph::Options::default();
        opts.title = "Flamegraph - Trace Visualization".to_string();
        opts.count_name = "microseconds".to_string();

        inferno::flamegraph::from_reader(&mut opts, reader, &mut output_file)
            .map_err(|e| ReportingError::Io(std::io::Error::other(e.to_string())))?;

        Ok(())
    }
}

impl Default for FlamegraphGenerator {
    fn default() -> Self {
        Self::new()
    }
}

/// Span node in the hierarchy tree
#[derive(Debug, Clone)]
struct SpanNode {
    span: TraceSpan,
    children: Vec<SpanNode>,
}

/// Flamegraph statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FlamegraphStats {
    pub total_spans: usize,
    pub max_depth: usize,
    pub total_duration_us: u64,
    pub hottest_path: Vec<String>,
}

impl FlamegraphGenerator {
    /// Generate statistics from trace
    pub fn generate_stats(&self, trace: &TraceData) -> Result<FlamegraphStats> {
        let hierarchy = self.build_hierarchy(trace)?;

        let total_spans = trace.spans.len();
        let max_depth = self.calculate_max_depth(&hierarchy, 0);
        let total_duration_us = hierarchy.span.duration_us;
        let hottest_path = self.find_hottest_path(&hierarchy);

        Ok(FlamegraphStats {
            total_spans,
            max_depth,
            total_duration_us,
            hottest_path,
        })
    }

    /// Calculate maximum depth of span tree
    #[allow(clippy::only_used_in_recursion)]
    fn calculate_max_depth(&self, node: &SpanNode, current_depth: usize) -> usize {
        if node.children.is_empty() {
            current_depth
        } else {
            node.children
                .iter()
                .map(|child| self.calculate_max_depth(child, current_depth + 1))
                .max()
                .unwrap_or(current_depth)
        }
    }

    /// Find the path with the longest cumulative duration
    fn find_hottest_path(&self, root: &SpanNode) -> Vec<String> {
        let mut path = Vec::new();
        let mut current = root;

        loop {
            path.push(format!("{}::{}", current.span.service_name, current.span.operation_name));

            if current.children.is_empty() {
                break;
            }

            // Follow the child with the longest duration
            current = current.children.iter().max_by_key(|child| child.span.duration_us).unwrap();
        }

        path
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_flamegraph_generation() {
        let trace = TraceData {
            trace_id: "trace-123".to_string(),
            spans: vec![
                TraceSpan {
                    span_id: "span-1".to_string(),
                    parent_span_id: None,
                    operation_name: "api_request".to_string(),
                    service_name: "api-gateway".to_string(),
                    start_time: 0,
                    duration_us: 10000,
                    tags: HashMap::new(),
                },
                TraceSpan {
                    span_id: "span-2".to_string(),
                    parent_span_id: Some("span-1".to_string()),
                    operation_name: "database_query".to_string(),
                    service_name: "postgres".to_string(),
                    start_time: 1000,
                    duration_us: 5000,
                    tags: HashMap::new(),
                },
                TraceSpan {
                    span_id: "span-3".to_string(),
                    parent_span_id: Some("span-1".to_string()),
                    operation_name: "cache_lookup".to_string(),
                    service_name: "redis".to_string(),
                    start_time: 6000,
                    duration_us: 1000,
                    tags: HashMap::new(),
                },
            ],
        };

        let generator = FlamegraphGenerator::new();
        let stats = generator.generate_stats(&trace).unwrap();

        assert_eq!(stats.total_spans, 3);
        assert!(stats.max_depth >= 1);
        assert_eq!(stats.total_duration_us, 10000);
    }

    #[test]
    fn test_hottest_path() {
        let trace = TraceData {
            trace_id: "trace-456".to_string(),
            spans: vec![
                TraceSpan {
                    span_id: "span-1".to_string(),
                    parent_span_id: None,
                    operation_name: "root".to_string(),
                    service_name: "service-a".to_string(),
                    start_time: 0,
                    duration_us: 20000,
                    tags: HashMap::new(),
                },
                TraceSpan {
                    span_id: "span-2".to_string(),
                    parent_span_id: Some("span-1".to_string()),
                    operation_name: "slow_operation".to_string(),
                    service_name: "service-b".to_string(),
                    start_time: 1000,
                    duration_us: 15000,
                    tags: HashMap::new(),
                },
                TraceSpan {
                    span_id: "span-3".to_string(),
                    parent_span_id: Some("span-1".to_string()),
                    operation_name: "fast_operation".to_string(),
                    service_name: "service-c".to_string(),
                    start_time: 16000,
                    duration_us: 1000,
                    tags: HashMap::new(),
                },
            ],
        };

        let generator = FlamegraphGenerator::new();
        let stats = generator.generate_stats(&trace).unwrap();

        // Hottest path should follow the slow_operation
        assert!(stats.hottest_path.contains(&"service-b::slow_operation".to_string()));
    }
}
