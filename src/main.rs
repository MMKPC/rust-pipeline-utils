use clap::{Parser, Subcommand};

mod checksum;
mod batch_processor;
mod dep_graph;
mod usd_resolver;
mod config;
mod error;

use error::PipelineError;

#[derive(Parser)]
#[command(name = "pipeline")]
#[command(about = "MMKPC pipeline utilities for UE5/VFX workflows")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Generate or verify BLAKE3 checksums for asset files
    Checksum {
        /// File path to hash
        path: String,
        /// Expected hash to verify against (omit to just generate)
        #[arg(long)]
        expected: Option<String>,
    },
    /// Batch-process asset files in a directory
    Batch {
        /// Input directory
        input_dir: String,
        /// Output directory
        output_dir: String,
        /// File extension filter (e.g. usda, fbx)
        #[arg(long, default_value = "usda")]
        ext: String,
    },
    /// Analyse dependency graph of USD/asset files for cycles
    Graph {
        /// Root file or directory to analyse
        path: String,
        /// Emit JSON dependency graph
        #[arg(long)]
        json: bool,
    },
    /// Resolve USD layer stack references
    UsdResolve {
        /// Path to .usda file
        path: String,
    },
}

fn main() -> Result<(), PipelineError> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Checksum { path, expected } => {
            let hash = checksum::generate(&path)?;
            match expected {
                Some(ref exp) => {
                    let ok = checksum::verify(&path, exp)?;
                    if ok {
                        println!("OK  {hash}  {path}");
                    } else {
                        eprintln!("MISMATCH  got={hash}  expected={exp}");
                        std::process::exit(1);
                    }
                }
                None => println!("{hash}  {path}"),
            }
        }

        Commands::Batch { input_dir, output_dir, ext } => {
            let results = batch_processor::process_directory(&input_dir, &output_dir, &ext)?;
            println!("Processed {} file(s)", results.len());
            for r in &results {
                println!("  {} -> {}", r.source, r.dest);
            }
        }

        Commands::Graph { path, json } => {
            let graph = dep_graph::build_graph(&path)?;
            if json {
                println!("{}", serde_json::to_string_pretty(&graph)?);
            } else {
                for (node, deps) in &graph.edges {
                    println!("{node} -> {:?}", deps);
                }
                if graph.has_cycle {
                    eprintln!("WARNING: cycle detected in dependency graph");
                    std::process::exit(2);
                }
            }
        }

        Commands::UsdResolve { path } => {
            let layers = usd_resolver::resolve_layer_stack(&path)?;
            for layer in &layers {
                println!("{}: {}", layer.depth, layer.path);
            }
        }
    }

    Ok(())
}
