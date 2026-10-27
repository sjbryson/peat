//! Bam-compare:
//! Main run function called by the bam-compare subcommand. 

use std::cmp::Ordering;
use std::collections::HashMap;
use std::io;

use n2bio::bam::{BamHeader, BamReader, BamRecord, BamFlags};

use crate::cli::{BamCompareArgs, ThresholdMetrics};
use crate::alignmentfilters::{highpass_bamfilter, threshold_args};

use crate::bamcompare_report::{ write_gml, write_json, write_tsv };

#[derive(Hash, Eq, PartialEq, Clone, Debug)]
pub struct EdgeKey {
    pub target_bam1: String,
    pub target_bam2: String,
}

pub type AlignmentGraph = HashMap<EdgeKey, usize>;

#[derive(Default, Debug)]
struct ReadGroup {
    qname: Vec<u8>,
    r1: Option<BamRecord>,
    r2: Option<BamRecord>,
}

pub(crate) fn run(args: BamCompareArgs) -> io::Result<()> {
    // Init readers
    let mut bam1_reader: BamReader = BamReader::open(args.bam1.to_str().unwrap())?;
    let header1: BamHeader = bam1_reader.read_header()?;

    let mut bam2_reader: BamReader = BamReader::open(args.bam2.to_str().unwrap())?;
    let header2: BamHeader = bam2_reader.read_header()?;

    // Set threshold evaluator
    let threshold_active: bool = threshold_args(&args.thresholds);
    let mut graph: AlignmentGraph = HashMap::new();

    
    let mut rec1: BamRecord = BamRecord::default();
    let mut rec2: BamRecord = BamRecord::default();
    let mut has_rec1: bool = bam1_reader.read_record(&mut rec1)?;
    let mut has_rec2: bool = bam2_reader.read_record(&mut rec2)?;

    let mut group1: Option<ReadGroup> = next_read_group(&mut bam1_reader, &mut rec1, &mut has_rec1)?;
    let mut group2: Option<ReadGroup> = next_read_group(&mut bam2_reader, &mut rec2, &mut has_rec2)?;

    // Dual-stream join (requires name-sorted BAMs)
    while group1.is_some() || group2.is_some() {
        match (&group1, &group2) {
            (Some(g1), Some(g2)) => {
                match g1.qname.cmp(&g2.qname) {
                    Ordering::Equal => {
                        evaluate_and_add_edge(
                            Some(g1), Some(g2), 
                            &header1, &header2, 
                            &args.thresholds, threshold_active, &mut graph
                        );
                        group1 = next_read_group(&mut bam1_reader, &mut rec1, &mut has_rec1)?;
                        group2 = next_read_group(&mut bam2_reader, &mut rec2, &mut has_rec2)?;
                    }
                    Ordering::Less => {
                        // g1 qname comes first alphabetically, meaning it's missing in BAM2
                        evaluate_and_add_edge(
                            Some(g1), None, 
                            &header1, &header2, 
                            &args.thresholds, threshold_active, &mut graph
                        );
                        group1 = next_read_group(&mut bam1_reader, &mut rec1, &mut has_rec1)?;
                    }
                    Ordering::Greater => {
                        // g2 qname comes first alphabetically, meaning it's missing in BAM1
                        evaluate_and_add_edge(
                            None, Some(g2), 
                            &header1, &header2, 
                            &args.thresholds, threshold_active, &mut graph
                        );
                        group2 = next_read_group(&mut bam2_reader, &mut rec2, &mut has_rec2)?;
                    }
                }
            }
            (Some(g1), None) => {
                evaluate_and_add_edge(
                    Some(g1), None, 
                    &header1, &header2, 
                    &args.thresholds, threshold_active, &mut graph
                );
                group1 = next_read_group(&mut bam1_reader, &mut rec1, &mut has_rec1)?;
            }
            (None, Some(g2)) => {
                evaluate_and_add_edge(
                    None, Some(g2), 
                    &header1, &header2, 
                    &args.thresholds, threshold_active, &mut graph
                );
                group2 = next_read_group(&mut bam2_reader, &mut rec2, &mut has_rec2)?;
            }
            (None, None) => break,
        }
    }

    // Optional outputs
    if args.graphml {
        write_gml(&graph, &args.output, &args.label1, &args.label2)?;
        println!("Writing GraphML to {}.graphml (placeholder)", args.output.display());
    }
    
    if args.tsv {
        write_tsv(&graph, &args.output, &args.label1, &args.label2)?;
        println!("Writing TSV to {}.tsv (placeholder)", args.output.display());
    }

    if args.json {
        write_json(&graph, &args.output, &args.label1, &args.label2)?;
        println!("Writing JSON to {}.json (placeholder)", args.output.display());
    }

    Ok(())
}

/// Consumes records from the reader as long as the `qname` matches the current record.
/// Returns a `ReadGroup` representing all primary alignments for the current `qname`.
fn next_read_group(
    reader: &mut BamReader,
    current_rec: &mut BamRecord,
    has_more: &mut bool,
) -> io::Result<Option<ReadGroup>> {
    if !*has_more {
        return Ok(None);
    }

    let mut group: ReadGroup = ReadGroup {
        qname: current_rec.read_name.clone(),
        r1: None,
        r2: None,
    };

    while *has_more && current_rec.read_name == group.qname {
        if current_rec.is_primary() {
            if current_rec.is_read1() || !current_rec.is_paired() {
                group.r1 = Some(current_rec.clone());
            } else if current_rec.is_read2() {
                group.r2 = Some(current_rec.clone());
            }
        }
        *has_more = reader.read_record(current_rec)?;
    }

    Ok(Some(group))
}

/// Evaluates a read group against the highpass filters and resolves its target.
/// Uses the R1 target by default.
fn resolve_target(
    group: Option<&ReadGroup>,
    header: &BamHeader,
    thresholds: &ThresholdMetrics,
    threshold_active: bool,
) -> String {
    let g: &ReadGroup = match group {
        Some(group) => group,
        None => return "unmapped".to_string(),
    };

    // Evaluate R1
    let r1_pass: bool = match &g.r1 {
        Some(r1) => highpass_bamfilter(r1, thresholds, threshold_active),
        None => false,
    };

    // Evaluate R2 (if it exists)
    let r2_pass: bool = match &g.r2 {
        Some(r2) => highpass_bamfilter(r2, thresholds, threshold_active),
        None => false,
    };

    // If neither passes, consider it unmapped for graph purposes
    if !r1_pass && !r2_pass {
        return "unmapped".to_string();
    }

    // Resolve target name, preferring R1 if it passed, else R2
    let rec: &BamRecord = if r1_pass { g.r1.as_ref() } else { g.r2.as_ref() }.unwrap();
    
    rec.target_name(header)
        .map(|s| s.to_string())
        .unwrap_or_else(|| "unmapped".to_string())
}

/// Resolves targets for both BAMs and increments the corresponding edge in the graph.
fn evaluate_and_add_edge(
    group1: Option<&ReadGroup>,
    group2: Option<&ReadGroup>,
    header1: &BamHeader,
    header2: &BamHeader,
    thresholds: &ThresholdMetrics,
    threshold_active: bool,
    graph: &mut AlignmentGraph,
) {
    let t1: String = resolve_target(group1, header1, thresholds, threshold_active);
    let t2: String = resolve_target(group2, header2, thresholds, threshold_active);

    // Skip entirely if neither BAM could align the read
    if t1 == "unmapped" && t2 == "unmapped" {
        return;
    }

    let edge: EdgeKey = EdgeKey {
        target_bam1: t1,
        target_bam2: t2,
    };

    *graph.entry(edge).or_insert(0) += 1;
}