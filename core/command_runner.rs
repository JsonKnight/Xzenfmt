use anyhow::{Context, Result, anyhow};
use std::io::Write;
use std::path::Path;
use std::process::{Command, Stdio};

const ASTYLE: &str = "astyle";
const RUSTFMT: &str = "rustfmt";
const RUBOCOP: &str = "rubocop";
const TAPLO: &str = "taplo";
const PRETTIERD: &str = "prettierd";
const ASMFMT: &str = "asmfmt";
const CRYSTAL: &str = "crystal";
const FISH_INDENT: &str = "fish_indent";
const SHFMT: &str = "shfmt";
const STYLUA: &str = "stylua";
const BLACK: &str = "black";
const PERLTIDY: &str = "perltidy";
const GOFMT: &str = "gofmt";
const ELM_FORMAT: &str = "elm-format";
const ORMOLU: &str = "ormolu";
const CABAL_FMT: &str = "cabal-fmt";
const TIDY: &str = "tidy";
const KTLINT: &str = "ktlint";
const GOOGLE_JAVA_FORMAT: &str = "google-java-format";
const SWIFT_FORMAT: &str = "swift-format";
const DOCKFMT: &str = "dockfmt";
const NGINXFMT: &str = "nginxfmt";
const NIXFMT: &str = "nixfmt";
const DJLINT: &str = "djlint";

fn run_formatter(tool: &str, base_args: &[&str], file_path: &Path) -> Result<()> {
    let mut cmd = Command::new(tool);
    cmd.args(base_args);
    cmd.arg(file_path);

    cmd.stdin(Stdio::null());
    cmd.stdout(Stdio::null());
    cmd.stderr(Stdio::piped());

    let process = cmd
        .spawn()
        .with_context(|| format!("Failed to spawn formatter '{}'", tool))?;

    let output = process
        .wait_with_output()
        .with_context(|| format!("Formatter '{}' failed to run", tool))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        anyhow::bail!(
            "Formatter '{}' failed for {}:\n{}",
            tool,
            file_path.display(),
            stderr.trim()
        );
    }
    Ok(())
}

fn run_formatter_stdin_stdout(tool: &str, base_args: &[&str], file_path: &Path) -> Result<()> {
    let input_content = std::fs::read(file_path).with_context(|| {
        format!(
            "Failed to read file {} for stdin formatter",
            file_path.display()
        )
    })?;

    let mut cmd = Command::new(tool);
    cmd.args(base_args);

    cmd.stdin(Stdio::piped());
    cmd.stdout(Stdio::piped());
    cmd.stderr(Stdio::piped());

    let mut process = cmd
        .spawn()
        .with_context(|| format!("Failed to spawn stdin formatter '{}'", tool))?;

    let mut stdin = process
        .stdin
        .take()
        .ok_or_else(|| anyhow!("Failed to open stdin for {}", tool))?;

    let write_thread = std::thread::spawn(move || {
        let _ = stdin.write_all(&input_content);
    });

    let output = process
        .wait_with_output()
        .with_context(|| format!("Stdin formatter '{}' failed to run", tool))?;

    let _ = write_thread.join();

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        anyhow::bail!(
            "Stdin formatter '{}' failed for {}:\n{}",
            tool,
            file_path.display(),
            stderr.trim()
        );
    }

    let parent_dir = file_path
        .parent()
        .ok_or_else(|| anyhow!("Failed to get parent dir for {}", file_path.display()))?;
    let mut temp_file = tempfile::Builder::new()
        .prefix(&format!(
            ".{}_fmt_",
            file_path
                .file_name()
                .map(|s| s.to_string_lossy())
                .unwrap_or_default()
        ))
        .suffix(".tmp")
        .tempfile_in(parent_dir)
        .with_context(|| format!("Failed to create temp file for writing {} output", tool))?;

    temp_file
        .write_all(&output.stdout)
        .with_context(|| format!("Failed to write {} output to temp file", tool))?;

    temp_file.persist(file_path).map_err(|persist_error| {
        anyhow!(
            "Failed to overwrite original file {} with temp file {}: {}",
            file_path.display(),
            persist_error.file.path().display(),
            persist_error.error
        )
    })?;

    Ok(())
}

pub fn run_rustfmt(file_path: &Path) -> Result<()> {
    run_formatter(RUSTFMT, &[], file_path)
        .with_context(|| format!("rustfmt failed for {}", file_path.display()))
}

pub fn run_astyle(file_path: &Path, lang: &str) -> Result<()> {
    let style_arg = match lang {
        "c" => "--style=kr",
        "cpp" => "--style=google",
        _ => "--style=kr",
    };
    run_formatter(ASTYLE, &[style_arg, "-n"], file_path)
        .with_context(|| format!("astyle failed for {}", file_path.display()))
}

pub fn run_rubocop_autocorrect(file_path: &Path) -> Result<()> {
    run_formatter(RUBOCOP, &["-A", "--fail-level", "error"], file_path)
        .with_context(|| format!("rubocop failed for {}", file_path.display()))
}

pub fn run_taplo_fmt(file_path: &Path) -> Result<()> {
    run_formatter(TAPLO, &["fmt"], file_path)
        .with_context(|| format!("taplo fmt failed for {}", file_path.display()))
}

pub fn run_prettierd_fmt(file_path: &Path) -> Result<()> {
    run_formatter(PRETTIERD, &[], file_path)
        .with_context(|| format!("prettierd failed for {}", file_path.display()))
}

pub fn run_asmfmt(file_path: &Path) -> Result<()> {
    run_formatter(ASMFMT, &[], file_path)
        .with_context(|| format!("asmfmt failed for {}", file_path.display()))
}

pub fn run_crystal_format(file_path: &Path) -> Result<()> {
    run_formatter_stdin_stdout(CRYSTAL, &["tool", "format", "-"], file_path)
        .with_context(|| format!("crystal tool format failed for {}", file_path.display()))
}

pub fn run_fish_indent(file_path: &Path) -> Result<()> {
    run_formatter(FISH_INDENT, &["-w"], file_path)
        .with_context(|| format!("fish_indent failed for {}", file_path.display()))
}

pub fn run_shfmt(file_path: &Path) -> Result<()> {
    run_formatter(SHFMT, &["-w", "-i", "4", "-ci"], file_path)
        .with_context(|| format!("shfmt failed for {}", file_path.display()))
}

pub fn run_stylua(file_path: &Path) -> Result<()> {
    run_formatter(STYLUA, &[], file_path)
        .with_context(|| format!("stylua failed for {}", file_path.display()))
}

pub fn run_black(file_path: &Path) -> Result<()> {
    run_formatter(BLACK, &["-q"], file_path)
        .with_context(|| format!("black failed for {}", file_path.display()))
}

pub fn run_perltidy(file_path: &Path) -> Result<()> {
    run_formatter_stdin_stdout(PERLTIDY, &["-st"], file_path)
        .with_context(|| format!("perltidy failed for {}", file_path.display()))
}

pub fn run_gofmt(file_path: &Path) -> Result<()> {
    run_formatter(GOFMT, &["-w"], file_path)
        .with_context(|| format!("gofmt failed for {}", file_path.display()))
}

pub fn run_elm_format(file_path: &Path) -> Result<()> {
    run_formatter(ELM_FORMAT, &["--yes"], file_path)
        .with_context(|| format!("elm-format failed for {}", file_path.display()))
}

pub fn run_ormolu(file_path: &Path) -> Result<()> {
    run_formatter(ORMOLU, &["-m", "inplace"], file_path)
        .with_context(|| format!("ormolu failed for {}", file_path.display()))
}

pub fn run_cabal_fmt(file_path: &Path) -> Result<()> {
    run_formatter(CABAL_FMT, &["-i"], file_path)
        .with_context(|| format!("cabal-fmt failed for {}", file_path.display()))
}

pub fn run_tidy(file_path: &Path) -> Result<()> {
    run_formatter(TIDY, &["-m", "-q", "-indent"], file_path)
        .with_context(|| format!("tidy failed for {}", file_path.display()))
}

pub fn run_ktlint(file_path: &Path) -> Result<()> {
    run_formatter(KTLINT, &["-F"], file_path)
        .with_context(|| format!("ktlint failed for {}", file_path.display()))
}

pub fn run_google_java_format(file_path: &Path) -> Result<()> {
    run_formatter(GOOGLE_JAVA_FORMAT, &["-i"], file_path)
        .with_context(|| format!("google-java-format failed for {}", file_path.display()))
}

pub fn run_swift_format(file_path: &Path) -> Result<()> {
    run_formatter(SWIFT_FORMAT, &["format", "--in-place"], file_path)
        .with_context(|| format!("swift-format failed for {}", file_path.display()))
}

pub fn run_dockfmt(file_path: &Path) -> Result<()> {
    run_formatter(DOCKFMT, &["format", "-w"], file_path)
        .with_context(|| format!("dockfmt failed for {}", file_path.display()))
}

pub fn run_nginxfmt(file_path: &Path) -> Result<()> {
    run_formatter(NGINXFMT, &[], file_path)
        .with_context(|| format!("nginxfmt failed for {}", file_path.display()))
}

pub fn run_nixfmt(file_path: &Path) -> Result<()> {
    run_formatter(NIXFMT, &[], file_path)
        .with_context(|| format!("nixfmt failed for {}", file_path.display()))
}

pub fn run_djlint(file_path: &Path) -> Result<()> {
    run_formatter(DJLINT, &["--reformat"], file_path)
        .with_context(|| format!("djlint failed for {}", file_path.display()))
}
