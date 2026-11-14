//! Bam-Clust:
//! Main run function called by the bam-clust subcommand. 
 
use std::collections::{ HashMap, HashSet };
use std::io;
use n2bio::bam::{ BamHeader, BamReader, BamRecord, BamFlags };

use crate::bamclust_report;
use crate::bamclust_clusters;
use crate::cli::BamClustArgs;
use crate::alignmentfilters::{ highpass_bamfilter, threshold_args };

// ============================================================================
// Graph components 
// ============================================================================

 /// Unified EdgeKey to store both directed and undirected edges in a single graph
#[derive(Hash, Eq, PartialEq, Clone, Debug)]
pub enum EdgeKey {
    /// Directed edge: from primary target -> secondary target
    Secondary { from: String, to: String },
    /// Undirected edge: discordant primary targets (alphabetically sorted)
    PrimaryDiscordant { t1: String, t2: String },
}

impl EdgeKey {
    pub fn new_primary_discordant(mut t1: String, mut t2: String) -> Self {
        // Ensure undirected edges are always stored in a consistent order
        if t1 > t2 {
            std::mem::swap(&mut t1, &mut t2);
        }
        EdgeKey::PrimaryDiscordant { t1, t2 }
    }
    
    pub fn new_secondary(from: String, to: String) -> Self {
        EdgeKey::Secondary { from, to }
    }
}

#[derive(Default, Clone, Debug)]
pub struct NodeStats {
    pub primary_count: usize,
    pub secondary_count: usize,
}

pub type ClusterGraph = HashMap<EdgeKey, usize>;
pub type NodeMap = HashMap<String, NodeStats>;

// ============================================================================
// BamClust run() 
// ============================================================================

pub(crate) fn run(args: BamClustArgs) -> io::Result<()> {
    let mut bam_reader: BamReader = BamReader::open(args.bam.to_str().unwrap())?;
    let header: BamHeader = bam_reader.read_header()?;

    let threshold_active: bool = threshold_args(&args.thresholds);
    
    let mut graph: ClusterGraph = HashMap::new();
    let mut nodes: NodeMap = HashMap::new();

    let mut current_rec: BamRecord = BamRecord::default();
    let mut has_more: bool = bam_reader.read_record(&mut current_rec)?;

    // Streaming loop (grouped by read name)
    while has_more {
        let current_qname: Vec<u8> = current_rec.read_name.clone();
        
        let mut primary_targets: HashSet<String> = HashSet::new();
        let mut secondary_targets: HashSet<String> = HashSet::new();

        // Consume all records for the current qname (R1, R2, and all secondary/supplementary)
        while has_more && current_rec.read_name == current_qname {
            
            if !current_rec.is_unmapped() && highpass_bamfilter(&current_rec, &args.thresholds, threshold_active) {
                if let Some(target_name) = current_rec.target_name(&header) {
                    let target: String = target_name.to_string();
                    let stats: &mut NodeStats = nodes.entry(target.clone()).or_insert_with(NodeStats::default);
                    
                    if current_rec.is_primary() {
                        stats.primary_count += 1;
                        primary_targets.insert(target);
                    } else {
                        stats.secondary_count += 1;
                        secondary_targets.insert(target);
                    }
                }
            }
            has_more = bam_reader.read_record(&mut current_rec)?;
        }

        // Undirected Primary Discordant Edges
        // Create edges between different targets mapped by discoradant primary R1/R2 alignments
        if primary_targets.len() > 1 {
            let p_targets: Vec<String> = primary_targets.iter().cloned().collect();
            for i in 0..p_targets.len() {
                for j in (i + 1)..p_targets.len() {
                    let edge: EdgeKey = EdgeKey::new_primary_discordant(p_targets[i].clone(), p_targets[j].clone());
                    *graph.entry(edge).or_insert(0) += 1;
                }
            }
        }

        // Directed Secondary Edges
        // Create edges from every primary target to every secondary target for this read
        for p_target in &primary_targets {
            for s_target in &secondary_targets {
                // Prevent self-loops (e.g. read has primary and split-read to the same target)
                if p_target != s_target {
                    let edge: EdgeKey = EdgeKey::new_secondary(p_target.clone(), s_target.clone());
                    *graph.entry(edge).or_insert(0) += 1;
                }
            }
        }
    }

    // Filter edges by min_weight: default = 1
    graph.retain(|_, &mut weight| weight >= args.min_weight);

    // Run optional output exporters
    if args.graphml { bamclust_report::write_gml(&graph, &nodes, &args.output)?; }
    if args.tsv { bamclust_report::write_tsv(&graph, &nodes, &args.output)?; }
    if args.json { bamclust_report::write_json(&graph, &nodes, &args.output)?; }
    if args.clusters { bamclust_clusters::write_clusters(&graph, &args.output)?; }

    Ok(())
}
