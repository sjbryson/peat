
<p align="center"><b><u>P</u>aired-<u>E</u>nd <u>A</u>lignment <u>T</u>ools</b></p>

PEAT is a suite of tools for working with paired-end alignments and is under active development. It started as independent [tools](https://github.com/sjbryson/n2bio) developed in my viral metagenomics research. I wanted to simplify alignment based human read filtering and viral detection pipelines while enabling simple switching between different alignment tools and reference sequence libraries.

#### There are several subcommands for peat:

```
Usage: peat <COMMAND>

Commands:
  filter     Parse SAM records from stdin and filter to create a filtered paired-end fastq.gz library (r1.fq.gz & r2.fq.gz)
  coverage   Parse SAM records from stdin and calculate coverage for each reference in the sam/bam header
  bam-rep    Read a name sorted bam file and generate an interactive report
  bin-reads  Read a name sorted bam file and bin read pairs for each target
  help       Print this message or the help of the given subcommand(s)

Options:
  -h, --help     Print help
  -V, --version  Print version
  ```
---
### peat filter:

Tool to parse SAM formatted stdout from aligners like minimap2, bowtie2, bwa, etc. and write paired reads that pass filter to <prefix>_r1.fq.gz and <prefix>_r2.fq.gz. Summary stats are written to <report>.json. This tool was originally developed to use in pipelines like host read filtering, eliminating some of the common time consuming write-sort-read-filter steps. In the example below unaligned read pairs that pass optional thresholds are witten to r1 and r2 fq.gz files.

**Pipeline example:**

```
minimap2 -ax sr --eqx --secondary=no -t <threads> <input_mmi> <r1.fq.gz> <r2.fq.gz> | \
peat filter -t <threads> --filter_mode lowpass --prefix <fq prefix> --report <json report> \
--AS <ALIGN_SCORE> --AL <ALIGN_LENGTH> --BS <BASE_SCORE> --AP <ALIGN_PROP> --AI <ALIGN_IDENT> --MQ <MAPQ>
```

Filter mode is set using the --filter_mode <FILTER_MODE> option.
- lowpass - all read pairs that are unmapped or pass all defined maximum threshold values are retained.
- highpass - all read pairs that are mapped and pass all defined minimum threshold values are retained.

```
Usage: peat filter [OPTIONS] --prefix <PREFIX> --report <REPORT> --filter_mode <FILTER_MODE>

Options:
  -t, --threads <THREADS>            Number of worker threads for parsing and pairing [default: 4]
  -s, --shards <SHARDS>              Number of shards for the mate pairing hash map (recommend 4-8x threads) [default: 32]
  -p, --prefix <PREFIX>              Prefix for output files (e.g. 'out' -> out.r1.fq.gz, out.r2.fq.gz)
  -r, --report <REPORT>              Name of the run/sample for the JSON report -> creates {report}.json
  -m, --filter_mode <FILTER_MODE>    Retain unmapped and use max thresholds (lowpass) or keep mapped and use min thresholds (highpass)
          Possible values:
          - lowpass:  Low Pass: process SAM records that are below defined thresholds
          - highpass: High Pass: process SAM records that are above defined thresholds
      --align_score <ALIGN_SCORE>    Optional: Alignment Score - sam.get_int_tag("AS")
      --align_length <ALIGN_LENGTH>  Optional: Alignment Lenth - sam.calculate_alignment_length()
      --base_score <BASE_SCORE>      Optional: Per base alignment score (BS = AS/AL, avg. align_score per covered base) - sam.calculate_base_score()
      --align_prop <ALIGN_PROP>      Optional: Alignment Proportion - sam.calculate_alignment_proportion()
      --align_ident <ALIGN_IDENT>    Optional: Alignment Percent Identity - sam.calculate_alignment_identity()
      --mapq <MAPQ>                  Optional: MAPQ score - sam.mapq()
  -h, --help                         Print help (see a summary with '-h')
```

---
### peat coverage:

Another tool to parse SAM formatted stdout from aligners like minimap2, bowtie2, bwa, etc. Use in metagenomics pipeline for target identification. Parses SAM records in stdout from aligner, calculates target coverage (per base) and stats. SAM records are passed through to stdout and can be used as input for samtools or written to file. Run and target level stats are writen to <report>.json. All paired primary and secondary alignments that score above all optional minimum thresholds (using the highpass filter) are writtten to primary and secondary coverage arrays. Mismatch counts are also stored in a mismatch array.

**Pipeline example:**

```
minimap2 -ax sr --eqx -t {threads} {input_mmi} {r1} {r2} | \
peat coverage -t {threads} -r {sample} --AS {min_as} | \
samtools sort -t {threads} - -o {sample}.sorted.bam
```

Or if you don't want to save the sam/bam file - pipe to /dev/null:

```
minimap2 -ax sr --eqx -t {threads} {input_mmi} {r1} {r2} | \
peat coverage -t {threads} -r {sample} --AS {min_as} > /dev/null
```

And if you want to work from an existing sam/bam file:

```
samtools view -h file.bam | peat coverage -t {threads} -r {sample} --AS {min_as} > /dev/null
```

An optional metadata file (--metadata or -m) can be used to add additional information for each reference sequence in the coverage report. The --metadata_key or -k option tells fastcov which column or field in the metadata file corresponds to the reference sequence identifier - e.g. a column named "accession" could refer to the accessions in the reference database that was aligned to - these should match what you would see in a sam/bam header. All additional fields and values associated with each key will be included in the report.json file.

```
Usage: peat coverage [OPTIONS] --report <REPORT>

Options:
  -t, --threads <THREADS>            Number of worker threads for parsing [default: 4]
  -r, --report <REPORT>              Name of the run/sample for the JSON report -> creates {report}.json
  -m, --metadata <METADATA>          Optional path to a metadata file
  -k, --metadata-key <METADATA_KEY>  Optional metadata keyword
      --align_score <ALIGN_SCORE>    Optional: Alignment Score - sam.get_int_tag("AS")
      --align_length <ALIGN_LENGTH>  Optional: Alignment Lenth - sam.calculate_alignment_length()
      --base_score <BASE_SCORE>      Optional: Per base alignment score (BS = AS/AL, avg. align_score per covered base) - sam.calculate_base_score()
      --align_prop <ALIGN_PROP>      Optional: Alignment Proportion - sam.calculate_alignment_proportion()
      --align_ident <ALIGN_IDENT>    Optional: Alignment Percent Identity - sam.calculate_alignment_identity()
      --mapq <MAPQ>                  Optional: MAPQ score - sam.mapq()
  -h, --help                         Print help
```

---
### peat bam-rep:

Tool to summarize alignment stats. *Input must be a **name sorted bam file** - position or unsorted bam files will not work.* Histograms are built for insert sizes and seperately for the following alignment stats for R1 and R2 reads:
- MAPQ scores
- Alignment Score (AS): Scoring may depend on the specific aligner used.
- Alignment Length (AL): Calculated from the CIGAR string. Matches, mismatches, and indels are counted; clipped regions are not
- Per Base Alignment Score (BS): The record's alignment score divided by the alignment length (AS/AL)
- Alignment Proportion (AP): The record's alignment length divided by the read length (AL/RL).
Alignment Identity (AI): Percentage equal to the number of matches (sam/bam tag "NM") divided by the alignment length (100 * NM/AL).

A json formatted report with raw distributions is automatically created. An interactive html report is generated when using the optional "--html" argument.

```
Usage: peat bam-rep [OPTIONS] --bam <BAM> --report <REPORT>

Options:
  -b, --bam <BAM>            Path to an input name-sorted BAM file to evaluate
  -r, --report <REPORT>      Report file prefix - creates {report}.json and optional {report}.html
      --html                 Generate html report
  -q, --min-mapq <MIN_MAPQ>  Minimum MAPQ score for insert size calculation [default: 40]
  -i, --max-ins <MAX_INS>    Max insert size to use for summary stats calculation [default: 1000]
  -l, --max-len <MAX_LEN>    Max read length to use [default: 150]
  -h, --help                 Print help
  ```
---
### peat bin-reads:

This tool was developed to allow aligned reads to be binned based on a user supplied target mapping file - a two column tsv file with target id's in the first column (these are the sequence identifiers such as accession ids in the reference database) and a desired bin name in the second column. *The input bam file must be **name sorted** - position or unsorted bam files will not work.*

```
Usage: peat bin-reads [OPTIONS] --bam <BAM> --output-dir <OUTPUT_DIR> --report <REPORT>

Options:
  -b, --bam <BAM>                      Path to an input name-sorted BAM file to evaluate
  -m, --reference-map <REFERENCE_MAP>  Path to a TSV mapping file: referenc_ id --> bin_id
  -o, --output-dir <OUTPUT_DIR>        Directory for output files (e.g. 'dir' -> dir/out.r1.fq.gz, dir/out.r2.fq.gz)
  -r, --report <REPORT>                Report file prefix - creates {report}.json
  -t, --threads <THREADS>              Number of worker threads for parsing and pairing [default: 4]
      --align_score <ALIGN_SCORE>      Optional: Alignment Score - bam.get_int_tag("AS")
      --align_length <ALIGN_LENGTH>    Optional: Alignment Lenth - bam.calculate_alignment_length()
      --base_score <BASE_SCORE>        Optional: Per base alignment score (BS = AS/AL, avg. align_score per covered base) - bam.calculate_base_score()
      --align_prop <ALIGN_PROP>        Optional: Alignment Proportion - bam.calculate_alignment_proportion()
      --align_ident <ALIGN_IDENT>      Optional: Alignment Percent Identity - bam.calculate_alignment_identity()
      --mapq <MAPQ>                    Optional: MAPQ score - bam.mapq
  -h, --help                           Print help
```
---
