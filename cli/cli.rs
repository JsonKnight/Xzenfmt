use xzenfmt_core::{
    CliArgs, Command as CoreCommand, OperationMode, ProcessedFileResult, XzenfmtArgs,
    check_dependencies, find_files, process_files,
};
mod interaction;

use anyhow::Result;
use clap::{CommandFactory, Parser};
use console::style;
use std::io;
use std::process::ExitCode;

fn print_completions_cli(shell: clap_complete::Shell) {
    let mut cmd = CliArgs::command();
    let name = cmd.get_name().to_string();
    clap_complete::generate(shell, &mut cmd, name, &mut io::stdout());
}

fn determine_operation_mode(args: &XzenfmtArgs) -> OperationMode {
    if args.all {
        OperationMode::All
    } else if args.strip_comments {
        OperationMode::Strip
    } else if args.strip_whitespace {
        OperationMode::StripWhitespace
    } else if args.strip_newlines {
        OperationMode::StripNewlines
    } else if args.code_format {
        OperationMode::Format
    } else {
        OperationMode::Format
    }
}

#[tokio::main]
async fn main() -> Result<ExitCode> {
    let cli: CliArgs = CliArgs::parse();
    let mut exit_code = ExitCode::SUCCESS;

    if let Some(command_enum_val) = cli.command {
        match command_enum_val {
            CoreCommand::Completion(args) => {
                print_completions_cli(args.shell);
                return Ok(ExitCode::SUCCESS);
            }
        }
    }

    let main_app_args = cli.main_opts;

    if main_app_args.check_dependencies {
        match check_dependencies(&main_app_args.lang) {
            Ok(_) => return Ok(ExitCode::SUCCESS),
            Err(e) => {
                eprintln!("{}", style(format!("Dependency Check Error: {}", e)).red());
                return Ok(ExitCode::FAILURE);
            }
        }
    }

    let files_to_process = match find_files(&main_app_args) {
        Ok(files) => files,
        Err(e) => {
            eprintln!("{}", style(format!("Error finding files: {}", e)).red());
            return Ok(ExitCode::FAILURE);
        }
    };

    if files_to_process.is_empty() {
        println!("No files found matching the criteria.");
        return Ok(ExitCode::SUCCESS);
    }

    println!("Found {} files:", files_to_process.len());
    for file in files_to_process.iter().take(10) {
        println!("  {}", style(file.display()).dim());
    }
    if files_to_process.len() > 10 {
        println!("  ... and {} more.", files_to_process.len() - 10);
    }

    match interaction::confirm_processing(files_to_process.len(), main_app_args.no_confirm) {
        Ok(true) => {}
        Ok(false) => return Ok(ExitCode::SUCCESS),
        Err(e) => {
            eprintln!(
                "{}",
                style(format!("Error during confirmation: {}", e)).red()
            );
            return Ok(ExitCode::FAILURE);
        }
    }

    let operation_mode = determine_operation_mode(&main_app_args);
    println!("Processing files (Mode: {:?})...", operation_mode);

    let processing_results: Vec<ProcessedFileResult> =
        match process_files(files_to_process, operation_mode) {
            Ok(results) => results,
            Err(e) => {
                eprintln!(
                    "{}",
                    style(format!("Critical error during processing setup: {}", e)).red()
                );
                return Ok(ExitCode::FAILURE);
            }
        };

    let mut success_count = 0;
    let mut failure_count = 0;
    println!("\nProcessing complete.");
    for result in processing_results {
        match result.error {
            None => {
                success_count += 1;
            }
            Some(err_msg) => {
                eprintln!(
                    "  {} Failed: {} - {}",
                    style("⚠️").yellow(),
                    style(result.path.display()).dim(),
                    style(err_msg).red()
                );
                failure_count += 1;
            }
        }
    }
    println!(
        "Result: {} {} processed successfully, {} {} failed.",
        style(success_count).green(),
        if success_count == 1 { "file" } else { "files" },
        style(failure_count).red(),
        if failure_count == 1 { "file" } else { "files" }
    );
    if failure_count > 0 {
        exit_code = ExitCode::FAILURE;
    }

    Ok(exit_code)
}
