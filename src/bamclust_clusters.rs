//! bamclust_clusters.rs

use std::collections::{ HashMap, HashSet, VecDeque };
use std::fs::File;
use std::io::{ self, BufWriter, Write };
use std::path::Path;

use crate::bamclust::{ ClusterGraph, EdgeKey };

// ============================================================================
// Clusters TSV Report 
// ============================================================================

pub(crate) fn write_clusters(graph: &ClusterGraph, output_prefix: &Path) -> io::Result<()> {
    // Build adjacency list using references
    let mut adj_list: HashMap<&String, Vec<&String>> = HashMap::new();

    for edge in graph.keys() {
        let (u, v) = match edge {
            EdgeKey::Secondary { from, to } => (from, to),
            EdgeKey::PrimaryDiscordant { t1, t2 } => (t1, t2),
        };

        adj_list.entry(u).or_default().push(v);
        adj_list.entry(v).or_default().push(u);
    }

    // Breadth-First Search (BFS) for connected targets
    let mut visited: HashSet<&String> = HashSet::new();
    let mut clusters: Vec<Vec<&String>> = Vec::new();

    for &node in adj_list.keys() {
        if !visited.contains(node) {
            let mut current_cluster: Vec<&String> = Vec::new();
            let mut queue: VecDeque<&String> = VecDeque::new();
            
            queue.push_back(node);
            visited.insert(node);

            while let Some(curr) = queue.pop_front() {
                current_cluster.push(curr);

                if let Some(neighbors) = adj_list.get(curr) {
                    for &neighbor in neighbors {
                        if !visited.contains(neighbor) {
                            visited.insert(neighbor);
                            queue.push_back(neighbor);
                        }
                    }
                }
            }
            clusters.push(current_cluster);
        }
    }

    // Sort clusters (largest first)
    clusters.sort_by(|a, b| b.len().cmp(&a.len()));

    // Write TSV
    let out_path: std::path::PathBuf = output_prefix.with_extension("clusters.tsv");
    let file: File = File::create(out_path)?;
    let mut writer: BufWriter<File> = BufWriter::new(file);

    writeln!(writer, "target_id\tcluster_id")?;

    for (cluster_idx, cluster_nodes) in clusters.iter_mut().enumerate() {
        let cluster_id: usize = cluster_idx + 1;
        
        // Sort nodes alphabetically within each cluster
        cluster_nodes.sort();
        
        for node in cluster_nodes {
            writeln!(writer, "{}\t{}", node, cluster_id)?;
        }
    }

    Ok(())
}