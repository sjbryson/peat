//! bamclust_report.rs

use std::collections::HashSet;
use std::fs::File;
use std::io::{ self, BufWriter, Write };
use std::path::Path;
use serde::Serialize;

use crate::bamclust::{ ClusterGraph, EdgeKey, NodeMap };

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

pub(crate) fn write_gml(
    graph: &ClusterGraph,
    nodes: &NodeMap,
    output_prefix: &Path,
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

    // Define node attribute
    writeln!(writer, r#"  <key id="v_name" for="node" attr.name="name" attr.type="string"/>"#)?;
    writeln!(writer, r#"  <key id="v_primary" for="node" attr.name="primary_alignments" attr.type="long"/>"#)?;
    writeln!(writer, r#"  <key id="v_secondary" for="node" attr.name="secondary_alignments" attr.type="long"/>"#)?;
    
    // Define edge attributes
    writeln!(writer, r#"  <key id="e_weight" for="edge" attr.name="weight" attr.type="long"/>"#)?;
    writeln!(writer, r#"  <key id="e_type" for="edge" attr.name="edge_type" attr.type="string"/>"#)?;

    // Start graph (undirected)
    writeln!(writer, r#"  <graph id="G" edgedefault="undirected">"#)?;

    // Collect unique nodes
    let mut unique_nodes: HashSet<&String> = HashSet::new();
    for edge in graph.keys() {
        match edge {
            EdgeKey::Secondary { from, to } => {
                unique_nodes.insert(from);
                unique_nodes.insert(to);
            }
            EdgeKey::PrimaryDiscordant { t1, t2 } => {
                unique_nodes.insert(t1);
                unique_nodes.insert(t2);
            }
        }
    }

    // Write nodes
    for target in unique_nodes {
        let escaped_id: String = escape_xml(target);
        // The node must exist in the nodes map if it has an edge
        let stats: &crate::bamclust::NodeStats = nodes.get(target).expect("Edge target missing from NodeMap");
        
        writeln!(writer, r#"    <node id="{}">"#, escaped_id)?;
        writeln!(writer, r#"      <data key="v_name">{}</data>"#, escaped_id)?;
        writeln!(writer, r#"      <data key="v_primary">{}</data>"#, stats.primary_count)?;
        writeln!(writer, r#"      <data key="v_secondary">{}</data>"#, stats.secondary_count)?;
        writeln!(writer, r#"    </node>"#)?;
    }

    // Write edges
    let mut edge_id_counter: i32 = 0;
    for (edge, &weight) in graph {
        // Resolve variants
        let (source, target, is_directed, e_type) = match edge {
            EdgeKey::Secondary { from, to } => {
                (from, to, true, "secondary")
            }
            EdgeKey::PrimaryDiscordant { t1, t2 } => {
                (t1, t2, false, "primary_discordant")
            }
        };

        // Mixed graph by specifies `directed` as edge attribute
        let dir_attr: &str = if is_directed { r#" directed="true""# } else { r#" directed="false""# };
        
        writeln!(
            writer, 
            r#"    <edge id="e{}" source="{}" target="{}"{}>"#, 
            edge_id_counter, escape_xml(source), escape_xml(target), dir_attr
        )?;
        writeln!(writer, r#"      <data key="e_weight">{}</data>"#, weight)?;
        writeln!(writer, r#"      <data key="e_type">{}</data>"#, e_type)?;
        writeln!(writer, r#"    </edge>"#)?;
        
        edge_id_counter += 1;
    }

    // Close GraphML
    writeln!(writer, r#"  </graph>"#)?;
    writeln!(writer, r#"</graphml>"#)?;

    Ok(())
}

// ============================================================================
// JSON Report 
// ============================================================================

#[derive(Serialize)]
pub struct Node {
    pub id: String,
    pub primary_alignments: usize,
    pub secondary_alignments: usize,
}

#[derive(Serialize)]
pub struct Link {
    pub source: String,
    pub target: String,
    pub weight: usize,
    pub edge_type: String,
    pub is_directed: bool,
}

#[derive(Serialize)]
pub struct GraphJson {
    pub nodes: Vec<Node>,
    pub links: Vec<Link>,
}

pub(crate) fn write_json(
    graph: &ClusterGraph,
    nodes: &NodeMap,
    output_prefix: &Path,
) -> io::Result<()> {
    let mut unique_node_ids: HashSet<String> = HashSet::new();
    let mut links: Vec<Link> = Vec::with_capacity(graph.len());

    // Set edges and collect active nodes
    for (edge, &weight) in graph {
        let (source, target, is_directed, edge_type) = match edge {
            EdgeKey::Secondary { from, to } => {
                (from, to, true, "secondary")
            }
            EdgeKey::PrimaryDiscordant { t1, t2 } => {
                (t1, t2, false, "primary_discordant")
            }
        };

        unique_node_ids.insert(source.clone());
        unique_node_ids.insert(target.clone());

        links.push(Link {
            source: source.clone(),
            target: target.clone(),
            weight,
            edge_type: edge_type.to_string(),
            is_directed,
        });
    }

    // Set nodes
    let mut output_nodes: Vec<Node> = Vec::with_capacity(unique_node_ids.len());
    for node_id in unique_node_ids {
        let stats: &crate::bamclust::NodeStats = nodes.get(&node_id).expect("Node missing from NodeMap");
        output_nodes.push(Node {
            id: node_id,
            primary_alignments: stats.primary_count,
            secondary_alignments: stats.secondary_count,
        });
    }

    // Serialize to JSON
    let graph_json: GraphJson = GraphJson {
        nodes: output_nodes,
        links,
    };

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
    graph: &ClusterGraph,
    nodes: &NodeMap,
    output_prefix: &Path,
) -> io::Result<()> {
    let out_path: std::path::PathBuf = output_prefix.with_extension("tsv");
    let file: File = File::create(out_path)?;
    let mut writer: BufWriter<File> = BufWriter::new(file);

    // Write TSV header
    writeln!(
        writer,
        "source\ttarget\tedge_type\tweight\tsource_primary_alignments\tsource_secondary_alignments\ttarget_primary_alignments\ttarget_secondary_alignments"
    )?;

    // Iterate over the filtered graph
    for (edge, &weight) in graph {
        // Resolve edge variants
        let (source, target, edge_type) = match edge {
            EdgeKey::Secondary { from, to } => {
                (from, to, "secondary")
            }
            EdgeKey::PrimaryDiscordant { t1, t2 } => {
                (t1, t2, "primary_discordant")
            }
        };

        // Get node stats
        let source_stats: &crate::bamclust::NodeStats = nodes.get(source).expect("Source target missing from NodeMap");
        let target_stats: &crate::bamclust::NodeStats = nodes.get(target).expect("Target missing from NodeMap");

        // Write row
        writeln!(
            writer,
            "{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}",
            source,
            target,
            edge_type,
            weight,
            source_stats.primary_count,
            source_stats.secondary_count,
            target_stats.primary_count,
            target_stats.secondary_count
        )?;
    }

    Ok(())
}