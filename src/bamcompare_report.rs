//! bamcompare_graphml.rs

use std::collections::HashMap;
use std::fs::File;
use std::io::{self, BufWriter, Write};
use std::path::Path;
use serde::Serialize;

use crate::bamcompare::AlignmentGraph;

// ============================================================================
// GML Report 
// ============================================================================

/// Helper to escape XML special characters in target names
fn escape_xml(s: &str) -> String {
    s.replace('&', "&amp;")
     .replace('<', "&lt;")
     .replace('>', "&gt;")
     .replace('"', "&quot;")
     .replace('\'', "&apos;")
}

// Struct to hold node attributes
struct NodeInfo<'a> {
    bam_label: &'a str,
    target: &'a str,
    total_alignments: usize,
}

pub(crate) fn write_gml(
    graph: &AlignmentGraph,
    output_prefix: &Path,
    label1: &str,
    label2: &str,
) -> io::Result<()> {
    let out_path: std::path::PathBuf = output_prefix.with_extension("graphml");
    let file: File = File::create(out_path)?;
    let mut writer: BufWriter<File> = BufWriter::new(file);

    // Write headers
    writeln!(writer, r#"<?xml version="1.0" encoding="UTF-8"?>"#)?;
    writeln!(writer, r#"<graphml xmlns="http://graphml.graphdrawing.org/xmlns""#)?;
    writeln!(writer, r#"         xmlns:xsi="http://www.w3.org/2001/XMLSchema-instance""#)?;
    writeln!(writer, r#"         xsi:schemaLocation="http://graphml.graphdrawing.org/xmlns"#)?;
    writeln!(writer, r#"         http://graphml.graphdrawing.org/xmlns/1.0/graphml.xsd">"#)?;

    // Define attributes
    writeln!(writer, r#"  <key id="v_name" for="node" attr.name="name" attr.type="string"/>"#)?;
    writeln!(writer, r#"  <key id="v_bam" for="node" attr.name="bam_source" attr.type="string"/>"#)?;
    writeln!(writer, r#"  <key id="v_alignments" for="node" attr.name="alignments" attr.type="long"/>"#)?;
    writeln!(writer, r#"  <key id="e_weight" for="edge" attr.name="weight" attr.type="long"/>"#)?;

    // Start graph
    writeln!(writer, r#"  <graph id="G" edgedefault="undirected">"#)?;

    // Collect unique nodes
    let mut unique_nodes: HashMap<String, NodeInfo> = HashMap::new();
    
    // Sum edge weights for each node to get total alignmets attribute
    for (edge, &weight) in graph {
        // BAM 1 Node Aggregation
        let id1: String = format!("{}::{}", label1, edge.target_bam1);
        let node1: &mut NodeInfo<'_> = unique_nodes.entry(id1).or_insert(NodeInfo {
            bam_label: label1,
            target: &edge.target_bam1,
            total_alignments: 0,
        });
        node1.total_alignments += weight;

        // BAM 2 Node Aggregation
        let id2: String = format!("{}::{}", label2, edge.target_bam2);
        let node2: &mut NodeInfo<'_> = unique_nodes.entry(id2).or_insert(NodeInfo {
            bam_label: label2,
            target: &edge.target_bam2,
            total_alignments: 0,
        });
        node2.total_alignments += weight;
    }

    // Write nodes
    for (node_id, info) in unique_nodes {
        let escaped_id: String = escape_xml(&node_id);
        let escaped_target: String = escape_xml(info.target);
        
        writeln!(writer, r#"    <node id="{}">"#, escaped_id)?;
        writeln!(writer, r#"      <data key="v_name">{}</data>"#, escaped_target)?;
        writeln!(writer, r#"      <data key="v_bam">{}</data>"#, escape_xml(info.bam_label))?;
        writeln!(writer, r#"      <data key="v_alignments">{}</data>"#, info.total_alignments)?;
        writeln!(writer, r#"    </node>"#)?;
    }

    // Write edges
    let mut edge_id_counter: i32 = 0;
    for (edge, &weight) in graph {
        let source_id: String = format!("{}::{}", label1, edge.target_bam1);
        let target_id: String = format!("{}::{}", label2, edge.target_bam2);
        
        writeln!(
            writer, 
            r#"    <edge id="e{}" source="{}" target="{}">"#, 
            edge_id_counter, escape_xml(&source_id), escape_xml(&target_id)
        )?;
        writeln!(writer, r#"      <data key="e_weight">{}</data>"#, weight)?;
        writeln!(writer, r#"    </edge>"#)?;
        
        edge_id_counter += 1;
    }

    // Close GML
    writeln!(writer, r#"  </graph>"#)?;
    writeln!(writer, r#"</graphml>"#)?;

    Ok(())
}

// ============================================================================
// JSON Report 
// ============================================================================

#[derive(Serialize, Clone)]
pub struct Node {
    pub id: String,
    pub bam: String,
    pub target: String,
    pub total_alignments: usize,
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
    let mut unique_nodes: HashMap<String, Node> = HashMap::new();
    let mut links: Vec<Link> = Vec::with_capacity(graph.len());

    for (edge, &weight) in graph {
        let id1: String = format!("{}::{}", label1, edge.target_bam1);
        let id2: String = format!("{}::{}", label2, edge.target_bam2);

        // Accumulate alignments for BAM 1 Node
        let node1: &mut Node = unique_nodes.entry(id1.clone()).or_insert_with(|| Node {
            id: id1.clone(),
            bam: label1.to_string(),
            target: edge.target_bam1.clone(),
            total_alignments: 0,
        });
        node1.total_alignments += weight;

        // Accumulate alignments for BAM 2 Node
        let node2: &mut Node = unique_nodes.entry(id2.clone()).or_insert_with(|| Node {
            id: id2.clone(),
            bam: label2.to_string(),
            target: edge.target_bam2.clone(),
            total_alignments: 0,
        });
        node2.total_alignments += weight;

        // Create link
        links.push(Link {
            source: id1,
            target: id2,
            weight,
        });
    }

    let nodes: Vec<Node> = unique_nodes.into_values().collect();
    
    let graph_json: GraphJson = GraphJson { nodes, links };

    let out_path: std::path::PathBuf = output_prefix.with_extension("json");
    let file: File = File::create(out_path)?;
    let writer: BufWriter<File> = BufWriter::new(file);

    serde_json::to_writer_pretty(writer, &graph_json)?;

    Ok(())
}


// ============================================================================
// TSV Report 
// ============================================================================

pub(crate) fn write_tsv(
    graph: &AlignmentGraph,
    output_prefix: &Path,
    label1: &str,
    label2: &str,
) -> io::Result<()> {
    let out_path: std::path::PathBuf = output_prefix.with_extension("tsv");
    let file: File = File::create(out_path)?;
    let mut writer: BufWriter<File> = BufWriter::new(file);

    // Write header
    writeln!(
        writer,
        "target_{}\ttarget_{}\tshared_reads\tstatus",
        label1, label2
    )?;
    // Write edges
    for (edge, weight) in graph {
        let status: String = if edge.target_bam1 != "unmapped" && edge.target_bam2 != "unmapped" {
            "mapped_both".to_string()
        } else if edge.target_bam1 != "unmapped" {
            format!("{}_only", label1)
        } else {
            format!("{}_only", label2)
        };

        writeln!(
            writer,
            "{}\t{}\t{}\t{}",
            edge.target_bam1, edge.target_bam2, weight, status
        )?;
    }

    Ok(())
}
