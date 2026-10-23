//! bamcompare_tsv.rs

use std::fs::File;
use std::io::{self, BufWriter, Write};
use std::path::Path;

use crate::bamcompare::AlignmentGraph;

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
