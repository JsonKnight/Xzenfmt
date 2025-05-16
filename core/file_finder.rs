use anyhow::{Context, Result};
use clap::{Parser, Subcommand};
use ignore::overrides::OverrideBuilder;
use ignore::{DirEntry, WalkBuilder};
use std::collections::{HashMap, HashSet};
use std::path::PathBuf;

#[derive(Debug, Parser, Clone)]
#[clap(
    version = "0.1.0",
    author = "json",
    about = "Minimalist code formatter and comment stripper (main arguments)",
    long_about = "These are the main arguments for formatting and stripping operations."
)]
pub struct XzenfmtArgs {
    #[clap(help = "Path to the file or directory to process", default_value = ".")]
    pub path: PathBuf,

    #[clap(
        long = "code-format",
        help = "Format code only [DEFAULT if no other mode specified and not 'completion']",
        group = "mode"
    )]
    pub code_format: bool,

    #[clap(
        long = "strip-comments",
        help = "Strip comments only (removes comment-only lines)",
        group = "mode"
    )]
    pub strip_comments: bool,

    #[clap(
        long = "strip-whitespace",
        help = "Strip trailing whitespace only",
        group = "mode"
    )]
    pub strip_whitespace: bool,

    #[clap(
        long = "strip-newlines",
        help = "Strip extra blank lines only",
        group = "mode"
    )]
    pub strip_newlines: bool,

    #[clap(
        long = "all",
        help = "Run: Format -> Strip Comments -> Format",
        group = "mode"
    )]
    pub all: bool,

    #[clap( long, value_name = "LANG", help = "Restrict to specific languages [multiple allowed]", action = clap::ArgAction::Append )]
    pub lang: Vec<String>,
    #[clap(long, help = "Skip the confirmation prompt")]
    pub no_confirm: bool,
    #[clap(long, help = "Check if required external tools are installed")]
    pub check_dependencies: bool,
    #[clap( long, value_name = "PATTERN", help = "Glob pattern for files to include [multiple allowed]", action = clap::ArgAction::Append )]
    pub include: Vec<String>,
    #[clap( long, value_name = "PATTERN", help = "Glob pattern for files/directories to exclude [multiple allowed]", action = clap::ArgAction::Append )]
    pub exclude: Vec<String>,
}

impl Default for XzenfmtArgs {
    fn default() -> Self {
        XzenfmtArgs {
            path: PathBuf::from("."),
            code_format: true,
            strip_comments: false,
            strip_whitespace: false,
            strip_newlines: false,
            all: false,
            lang: Vec::new(),
            no_confirm: false,
            check_dependencies: false,
            include: Vec::new(),
            exclude: Vec::new(),
        }
    }
}

#[derive(Debug, Subcommand, Clone)]
pub enum Command {
    #[clap(about = "Generate shell completion scripts")]
    Completion(CompletionArgs),
}

#[derive(Debug, Parser, Clone)]
pub struct CompletionArgs {
    #[clap(value_parser = clap::value_parser!(clap_complete::Shell))]
    pub shell: clap_complete::Shell,
}

#[derive(Debug, Parser, Clone)]
#[clap(
    name = "xzenfmt",
    version = "0.1.0",
    author = "json",
    about = "Minimalist code formatter and comment stripper",
    long_about = "Formats code and optionally strips comments/whitespace for specified languages.\nPrioritizes simplicity, speed, and a zen-like experience.",
    propagate_version = true
)]
pub struct CliArgs {
    #[clap(subcommand)]
    pub command: Option<Command>,

    #[clap(flatten)]
    pub main_opts: XzenfmtArgs,
}

const RUST_EXTENSIONS: &[&str] = &["rs"];
const C_EXTENSIONS: &[&str] = &["c", "h"];
const CPP_EXTENSIONS: &[&str] = &["cpp", "cxx", "cc", "hpp"];
const RUBY_EXTENSIONS: &[&str] = &["rb", "rake"];
const RUBY_SPECIAL_FILES: &[&str] = &["Rakefile", "Gemfile"];
const TOML_EXTENSIONS: &[&str] = &["toml"];
const JSON_EXTENSIONS: &[&str] = &["json", "jsonc"];
const YAML_EXTENSIONS: &[&str] = &["yaml", "yml"];
const PYTHON_EXTENSIONS: &[&str] = &["py"];
const GO_EXTENSIONS: &[&str] = &["go"];
const LUA_EXTENSIONS: &[&str] = &["lua"];
const SHELL_EXTENSIONS: &[&str] = &["sh", "bash"];
const FISH_EXTENSIONS: &[&str] = &["fish"];
const PERL_EXTENSIONS: &[&str] = &["pl", "pm"];
const HASKELL_EXTENSIONS: &[&str] = &["hs", "lhs"];
const CABAL_EXTENSIONS: &[&str] = &["cabal"];
const ELM_EXTENSIONS: &[&str] = &["elm"];
const CRYSTAL_EXTENSIONS: &[&str] = &["cr"];
const JAVA_EXTENSIONS: &[&str] = &["java"];
const KOTLIN_EXTENSIONS: &[&str] = &["kt", "kts"];
const SWIFT_EXTENSIONS: &[&str] = &["swift"];
const MARKDOWN_EXTENSIONS: &[&str] = &["md", "markdown"];
const HTML_EXTENSIONS: &[&str] = &["html", "htm"];
const XML_EXTENSIONS: &[&str] = &["xml", "xhtml"];
const CSS_EXTENSIONS: &[&str] = &["css"];
const SCSS_EXTENSIONS: &[&str] = &["scss"];
const LESS_EXTENSIONS: &[&str] = &["less"];
const NIX_EXTENSIONS: &[&str] = &["nix"];
const TWIG_EXTENSIONS: &[&str] = &["twig"];
const DOCKERFILE_FILENAMES: &[&str] = &["Dockerfile"];
const CONF_EXTENSIONS: &[&str] = &["conf"];
const ASSEMBLY_EXTENSIONS: &[&str] = &["asm", "s"];

fn build_language_extension_map() -> HashMap<&'static str, &'static [&'static str]> {
    let mut m = HashMap::new();
    m.insert("rust", RUST_EXTENSIONS);
    m.insert("c", C_EXTENSIONS);
    m.insert("cpp", CPP_EXTENSIONS);
    m.insert("ruby", RUBY_EXTENSIONS);
    m.insert("toml", TOML_EXTENSIONS);
    m.insert("json", JSON_EXTENSIONS);
    m.insert("yaml", YAML_EXTENSIONS);
    m.insert("python", PYTHON_EXTENSIONS);
    m.insert("go", GO_EXTENSIONS);
    m.insert("lua", LUA_EXTENSIONS);
    m.insert("shell", SHELL_EXTENSIONS);
    m.insert("fish", FISH_EXTENSIONS);
    m.insert("perl", PERL_EXTENSIONS);
    m.insert("haskell", HASKELL_EXTENSIONS);
    m.insert("cabal", CABAL_EXTENSIONS);
    m.insert("elm", ELM_EXTENSIONS);
    m.insert("crystal", CRYSTAL_EXTENSIONS);
    m.insert("java", JAVA_EXTENSIONS);
    m.insert("kotlin", KOTLIN_EXTENSIONS);
    m.insert("swift", SWIFT_EXTENSIONS);
    m.insert("markdown", MARKDOWN_EXTENSIONS);
    m.insert("html", HTML_EXTENSIONS);
    m.insert("xml", XML_EXTENSIONS);
    m.insert("css", CSS_EXTENSIONS);
    m.insert("scss", SCSS_EXTENSIONS);
    m.insert("less", LESS_EXTENSIONS);
    m.insert("nix", NIX_EXTENSIONS);
    m.insert("twig", TWIG_EXTENSIONS);
    m.insert("conf", CONF_EXTENSIONS);
    m.insert("assembly", ASSEMBLY_EXTENSIONS);
    m
}
fn build_special_filename_map() -> HashMap<&'static str, &'static str> {
    let mut m = HashMap::new();
    for &f in RUBY_SPECIAL_FILES {
        m.insert(f, "ruby");
    }
    for &f in DOCKERFILE_FILENAMES {
        m.insert(f, "dockerfile");
    }
    m
}

pub fn find_files(args: &XzenfmtArgs) -> Result<Vec<PathBuf>> {
    let r = &args.path;
    if !r.exists() {
        anyhow::bail!("Path not found: {}", r.display());
    }
    let l = build_language_extension_map();
    let s = build_special_filename_map();
    let mut k: HashSet<&str> = l.keys().cloned().collect();
    k.insert("dockerfile");
    let t: Vec<String> = if args.lang.is_empty() {
        k.iter().map(|&s_val| s_val.to_string()).collect()
    } else {
        let mut v = Vec::new();
        for i in &args.lang {
            if k.contains(i.as_str()) {
                v.push(i.clone());
            } else {
                eprintln!("Warning: Unsupported language specified, skipping: {}", i);
            }
        }
        if v.is_empty() && !args.lang.is_empty() {
            anyhow::bail!("No valid languages specified: {:?}", args.lang);
        }
        v
    };
    let mut w = WalkBuilder::new(r);
    w.standard_filters(true);
    w.hidden(false);
    let mut o = OverrideBuilder::new(r);
    for p in &args.exclude {
        let q = format!("!{}", p);
        o.add(&q).with_context(|| format!("Exclude: {}", p))?;
    }
    for p in &args.include {
        o.add(p).with_context(|| format!("Include: {}", p))?;
    }
    let v = o.build().context("Overrides")?;
    w.overrides(v);
    let mut f = Vec::new();
    let ts: HashSet<_> = t.iter().map(String::as_str).collect();
    for i in w.build() {
        match i {
            Ok(e) => {
                if e.file_type().is_some_and(|ft| ft.is_file()) && is_target_entry(&e, &ts, &l, &s)
                {
                    f.push(e.into_path());
                }
            }
            Err(e) => {
                eprintln!("Warn: {}", e);
            }
        }
    }
    if f.is_empty() && !t.is_empty() {
        println!("No files found for: {:?}", t);
    }
    f.sort();
    Ok(f)
}
fn is_target_entry(
    e: &DirEntry,
    t: &HashSet<&str>,
    l: &HashMap<&'static str, &'static [&'static str]>,
    s: &HashMap<&'static str, &'static str>,
) -> bool {
    let p = e.path();
    let n = match p.file_name() {
        Some(name_val) => name_val,
        None => return false,
    };
    let ns = n.to_string_lossy();
    let x = p.extension();
    if let Some(g) = s.get(ns.as_ref()) {
        return t.is_empty() || t.contains(g);
    }
    if let Some(o_val) = x {
        let xs = o_val.to_string_lossy().to_lowercase();
        for (g, e_val) in l {
            if e_val.contains(&xs.as_ref()) {
                return t.is_empty() || t.contains(g);
            }
        }
    }
    false
}
