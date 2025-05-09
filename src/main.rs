mod config;
mod errors;
mod feature_extraction;
mod image_io;
mod image_utils;
mod morphology;
mod output;
mod path_algorithms;
mod pipeline;
mod point_analysis;
mod font; 
mod gui; // GUI module


use std::path::PathBuf;
use std::time::Instant;
use std::fs;
use clap::{Parser, ValueEnum};
use rayon::prelude::*;

use config::Config;
use errors::{LeafComplexError, Result};
use image_io::{get_png_files_in_dir, load_image};
use pipeline::process_image;

/// Command-line arguments
#[derive(Parser, Debug)]
#[clap(author, version, about = "LeafComplexR - Leaf Morphology Analysis")]
struct Args {
    /// Path to input file or directory
    #[clap(short, long)]
    input: Option<String>,
    
    /// Path to output directory
    #[clap(short, long)]
    output: Option<String>,
    
    /// Path to configuration file
    #[clap(short, long, default_value = "config.toml")]
    config: String,
    
    /// Reference point choice (overwrites config)
    #[clap(short = 'r', long)]
    reference_point: Option<ReferencePointArg>,
    
    /// Enable debug mode (save intermediate images and print more info)
    #[clap(short, long)]
    debug: bool,
    
    /// Launch GUI visualization tool
    #[clap(long)]
    gui: bool,
}

#[derive(Debug, Clone, Copy, ValueEnum)]
enum ReferencePointArg {
    EP,
    COM,
}

/// Main function
fn main() -> Result<()> {
    // Parse command-line arguments
    let args = Args::parse();
    
    // Load configuration
    let mut config = Config::from_file(&args.config)?;
    
    // Override config with command-line arguments
    if let Some(input) = args.input.clone() {
        config.input_path = input;
    }
    
    if let Some(output) = args.output.clone() {
        config.output_base_dir = output;
    }
    
    if let Some(ref_point) = args.reference_point {
        config.reference_point_choice = match ref_point {
            ReferencePointArg::EP => config::ReferencePointChoice::Ep,
            ReferencePointArg::COM => config::ReferencePointChoice::Com,
        };
    }
    
    // Check if GUI mode is enabled
    if args.gui {
        // For GUI mode, we need a single input file
        let input_path = PathBuf::from(&config.input_path);
        
        if input_path.is_file() {
            println!("Launching GUI mode with image: {}", input_path.display());
            return gui::run_gui(input_path, config);
        } else {
            return Err(LeafComplexError::Config(
                "GUI mode requires a single input file, not a directory".to_string()
            ));
        }
    }
    
    // Validate configuration
    config.validate()?;
    
    // Start timing
    let start_time = Instant::now();
    
    // Create output directories
    let output_base = PathBuf::from(&config.output_base_dir);
    fs::create_dir_all(&output_base.join("LEC"))?;
    fs::create_dir_all(&output_base.join("LMC"))?;
    
    if args.debug {
        fs::create_dir_all(&output_base.join("debug"))?;
    }
    
    // Process input
    let input_path = PathBuf::from(&config.input_path);
    
    if input_path.is_file() {
        // Process single file
        println!("Processing single file: {}", input_path.display());
        let input_image = load_image(&input_path)?;
        process_image(input_image, &config, args.debug)?;
    } else if input_path.is_dir() {
        // Process all PNG files in directory
        println!("Processing directory: {}", input_path.display());
        let png_files = get_png_files_in_dir(&input_path)?;
        
        println!("Found {} PNG files", png_files.len());
        
        if config.use_parallel {
            // Process files in parallel
            png_files.par_iter()
                .map(|path| {
                    println!("Processing: {}", path.display());
                    match load_image(path) {
                        Ok(input_image) => process_image(input_image, &config, args.debug),
                        Err(e) => {
                            eprintln!("Error loading {}: {}", path.display(), e);
                            Err(e)
                        }
                    }
                })
                .collect::<Vec<_>>();
        } else {
            // Process files sequentially
            for path in &png_files {
                println!("Processing: {}", path.display());
                let input_image = load_image(path)?;
                process_image(input_image, &config, args.debug)?;
            }
        }
    } else {
        return Err(LeafComplexError::InvalidPath(input_path));
    }
    
    // Report elapsed time
    let elapsed = start_time.elapsed();
    println!("Processing completed in {:.2} seconds", elapsed.as_secs_f64());
    
    Ok(())
}