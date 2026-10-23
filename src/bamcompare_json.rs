//! bamcompare_json.rs

use serde::Serialize;
use std::collections::HashSet;
use std::fs::File;
use std::io::{self, BufWriter};
use std::path::Path;

use crate::bamcompare::AlignmentGraph;

#[derive(Serialize, Hash, Eq, PartialEq, Clone)]
pub(crate) struct Node {
    pub id: String,
    pub bam: String,
    pub target: String,
}

#[derive(Serialize, Clone)]
pub(crate) struct Link {
    pub source: String,
    pub target: String,
    pub weight: usize,
}

#[derive(Serialize)]
pub(crate) struct GraphJson {
    pub nodes: Vec<Node>,
    pub links: Vec<Link>,
}

pub(crate) fn write_json(
    graph: &AlignmentGraph,
    output_prefix: &Path,
    label1: &str,
    label2: &str,
) -> io::Result<()> {
    let mut unique_nodes: HashSet<Node> = HashSet::new();
    let mut links: Vec<Link> = Vec::with_capacity(graph.len());

    for (edge, &weight) in graph {
        let id1: String = format!("{}::{}", label1, edge.target_bam1);
        let id2: String = format!("{}::{}", label2, edge.target_bam2);

        // Track unique nodes for BAM 1
        unique_nodes.insert(Node {
            id: id1.clone(),
            bam: label1.to_string(),
            target: edge.target_bam1.clone(),
        });

        // Track unique nodes for BAM 2
        unique_nodes.insert(Node {
            id: id2.clone(),
            bam: label2.to_string(),
            target: edge.target_bam2.clone(),
        });

        // Create link
        links.push(Link {
            source: id1,
            target: id2,
            weight,
        });
    }

    // Convert HashSet to Vec
    let nodes: Vec<Node> = unique_nodes.into_iter().collect();
    
    let graph_json: GraphJson = GraphJson { nodes, links };

    let out_path: std::path::PathBuf = output_prefix.with_extension("json");
    let file: File = File::create(out_path)?;
    let writer: BufWriter<File> = BufWriter::new(file);

    serde_json::to_writer_pretty(writer, &graph_json)?;

    Ok(())
}