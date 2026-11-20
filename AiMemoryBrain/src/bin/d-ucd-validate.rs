use clap::{Parser, Subcommand};

#[derive(Debug, Parser)]
#[command(name = "d-ucd-validate")]
#[command(about = "Validate D Unicode UCD lock and normalization evidence")]
struct Args {
    #[command(subcommand)]
    command: Command,
}

#[derive(Debug, Subcommand)]
enum Command {
    /// Verify manifest and every locked source/derived artifact.
    VerifyLock {
        #[arg(long, default_value = "ucd/15.1.0/ucd-lock.json")]
        lock: std::path::PathBuf,
        #[arg(long, default_value = ".")]
        root: std::path::PathBuf,
    },

    /// Run the complete Unicode 15.1.0 normalization corpus.
    VerifyNormalization {
        #[arg(long, default_value = "ucd/15.1.0/NormalizationTest.txt")]
        corpus: std::path::PathBuf,
    },
}

fn main() {
    let args = Args::parse();

    // Step 1 is an interface skeleton. Operational commands remain
    // deliberately unimplemented until Step 2.
    match args.command {
        Command::VerifyLock { lock, root } => {
            let _ = (lock, root);
            eprintln!("BLOCKED: lock verification implementation not landed");
            std::process::exit(2);
        }
        Command::VerifyNormalization { corpus } => {
            let _ = corpus;
            eprintln!("BLOCKED: normalization implementation not landed");
            std::process::exit(2);
        }
    }
}
