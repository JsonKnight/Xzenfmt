use crate::command_runner::*;
use crate::stripper::{self, StripError};
use anyhow::Result;
use rayon::prelude::*;
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OperationMode {
    Format,
    Strip,
    StripWhitespace,
    StripNewlines,
    All,
}

#[derive(Debug)]
pub struct ProcessedFileResult {
    pub path: PathBuf,
    pub error: Option<String>,
}

fn remove_trailing_whitespace(text: &str) -> String {
    if text.is_empty() {
        return String::new();
    }
    let mut r = String::with_capacity(text.len());
    for (i, l) in text.lines().enumerate() {
        if i > 0 {
            r.push('\n');
        }
        r.push_str(l.trim_end_matches(|c: char| c.is_whitespace() && c != '\n'));
    }
    if text.ends_with('\n') && !r.is_empty() && !r.ends_with('\n') {
        r.push('\n');
    }
    if text.trim().is_empty() && text.ends_with('\n') && !text.is_empty() {
        return "\n".to_string();
    }

    r
}

fn collapse_blank_lines(text: &str) -> String {
    let mut result = String::with_capacity(text.len());
    let mut consecutive_blank_lines = 0;
    let mut first_line = true;

    for line in text.lines() {
        let trimmed_line = line.trim();
        let is_blank = trimmed_line.is_empty();

        if is_blank {
            consecutive_blank_lines += 1;
        } else {
            consecutive_blank_lines = 0;
        }

        if consecutive_blank_lines <= 1 {
            if !first_line {
                result.push('\n');
            }
            result.push_str(line);
            first_line = false;
        }
    }

    if text.ends_with('\n') && !result.is_empty() && !result.ends_with('\n') {
        result.push('\n');
    }
    if text.chars().all(|c| c == '\n') && !text.is_empty() {
        return "\n".to_string();
    }

    result
}

fn get_language_from_path(path: &Path) -> Option<&str> {
    if let Some(n) = path.file_name() {
        let s = n.to_string_lossy();
        match s.as_ref() {
            "Rakefile" | "Gemfile" => return Some("ruby"),
            "Dockerfile" => return Some("dockerfile"),
            _ => {}
        }
    }
    path.extension()
        .and_then(|e| e.to_str())
        .map(|e| e.to_lowercase())
        .and_then(|e| match e.as_str() {
            "rs" => Some("rust"),
            "c" | "h" => Some("c"),
            "cpp" | "cxx" | "cc" | "hpp" => Some("cpp"),
            "rb" | "rake" => Some("ruby"),
            "toml" => Some("toml"),
            "json" | "jsonc" => Some("json"),
            "yaml" | "yml" => Some("yaml"),
            "py" => Some("python"),
            "go" => Some("go"),
            "lua" => Some("lua"),
            "sh" | "bash" => Some("shell"),
            "fish" => Some("fish"),
            "pl" | "pm" => Some("perl"),
            "hs" | "lhs" => Some("haskell"),
            "cabal" => Some("cabal"),
            "elm" => Some("elm"),
            "cr" => Some("crystal"),
            "java" => Some("java"),
            "kt" | "kts" => Some("kotlin"),
            "swift" => Some("swift"),
            "md" | "markdown" => Some("markdown"),
            "html" | "htm" => Some("html"),
            "xml" | "xhtml" => Some("xml"),
            "css" => Some("css"),
            "scss" => Some("scss"),
            "less" => Some("less"),
            "nix" => Some("nix"),
            "twig" => Some("twig"),
            "conf" => Some("conf"),
            "asm" | "s" => Some("assembly"),
            _ => None,
        })
}
fn map_err_to_string<E: std::fmt::Display>(p: &Path, c: &str) -> impl Fn(E) -> String {
    let d = p.display().to_string();
    move |e| format!("{} failed for {}: {}", c, d, e)
}

fn strip_comments_smart(input: &str, lang: &str) -> Result<String, StripError> {
    let matches = stripper::find_language_comments(input, lang, Path::new(""))?;
    if matches.is_empty()
        && !input.contains("\n//")
        && !input.contains("\n/*")
        && !input.contains("\n#")
    {
        return Ok(input.to_string());
    }

    let mut lines_to_keep = Vec::new();
    let mut line_start_indices = vec![0];
    line_start_indices.extend(input.match_indices('\n').map(|(i, _)| i + 1));
    if !input.is_empty() && !input.ends_with('\n') {
        line_start_indices.push(input.len());
    }

    for window in line_start_indices.windows(2) {
        let line_start = window[0];
        let line_end = window[1];

        if line_start >= line_end {
            continue;
        }

        let line_content_with_ending = &input[line_start..line_end];
        let line_content = line_content_with_ending.trim_end_matches('\n');
        let trimmed_line = line_content.trim();

        if trimmed_line.starts_with("#!")
            || (lang == "ruby" && trimmed_line == "# frozen_string_literal: true")
        {
            lines_to_keep.push(line_content_with_ending);
            continue;
        }

        let trimmed_start_offset = line_content.find(trimmed_line).unwrap_or(0);
        let absolute_trimmed_start = line_start + trimmed_start_offset;
        let absolute_trimmed_end = absolute_trimmed_start + trimmed_line.len();

        let is_comment_only = !trimmed_line.is_empty()
            && matches
                .iter()
                .any(|m| m.from <= absolute_trimmed_start && m.to >= absolute_trimmed_end);

        if !is_comment_only {
            lines_to_keep.push(line_content_with_ending);
        }
    }

    let filtered_content = lines_to_keep.join("");

    let final_matches = stripper::find_language_comments(&filtered_content, lang, Path::new(""))?;

    stripper::remove_matches(filtered_content, final_matches)
}
fn process_single_file(path: &Path, mode: OperationMode) -> Result<(), String> {
    let lang = match get_language_from_path(path) {
        Some(l) => l,
        None => return Ok(()),
    };
    let can_format = !matches!(
        lang,
        "assembly" | "cabal" | "conf" | "erb" | "elisp" | "svelte" | "vue"
    );
    let can_strip = !matches!(lang, "assembly" | "erb" | "cabal" | "svelte" | "vue");
    let can_clean_whitespace = true;
    let can_clean_newlines = true;

    match mode {
        OperationMode::Format | OperationMode::All if !can_format => return Ok(()),
        OperationMode::Strip if !can_strip => return Ok(()),
        OperationMode::StripWhitespace if !can_clean_whitespace => return Ok(()),
        OperationMode::StripNewlines if !can_clean_newlines => return Ok(()),
        _ => {}
    }

    let original_content = fs::read_to_string(path).map_err(map_err_to_string(path, "Read"))?;
    let mut current_content = original_content.clone();
    let mut modified = false;

    match mode {
        OperationMode::Format => {
            if !can_format {
                return Ok(());
            }
            run_formatter_for_lang(path, lang)?;
            current_content = fs::read_to_string(path)
                .map_err(map_err_to_string(path, "Re-read after format"))?;
            if current_content != original_content {
                modified = true;
            }
        }
        OperationMode::Strip => {
            if !can_strip {
                return Ok(());
            }
            let stripped_content = strip_comments_smart(&current_content, lang)
                .map_err(|e| format!("Smart stripping failed: {}", e))?;
            if stripped_content != current_content {
                current_content = stripped_content;
                modified = true;
            }
        }
        OperationMode::StripWhitespace => {
            if !can_clean_whitespace {
                return Ok(());
            }
            let stripped_content = remove_trailing_whitespace(&current_content);
            if stripped_content != current_content {
                current_content = stripped_content;
                modified = true;
            }
        }
        OperationMode::StripNewlines => {
            if !can_clean_newlines {
                return Ok(());
            }
            let stripped_content = collapse_blank_lines(&current_content);
            if stripped_content != current_content {
                current_content = stripped_content;
                modified = true;
            }
        }
        OperationMode::All => {
            if !can_format {
                return Ok(());
            }

            run_formatter_for_lang(path, lang)?;
            current_content = fs::read_to_string(path)
                .map_err(map_err_to_string(path, "Re-read after format 1"))?;
            if current_content != original_content {
                modified = true;
            }
            let content_after_fmt1 = current_content.clone();

            if !can_strip {
                if modified {
                    fs::write(path, content_after_fmt1).map_err(map_err_to_string(
                        path,
                        "Write after format 1 (cannot strip)",
                    ))?;
                }
                return Ok(());
            }
            let stripped_content = strip_comments_smart(&content_after_fmt1, lang)
                .map_err(|e| format!("Smart stripping failed for --all: {}", e))?;
            if stripped_content != content_after_fmt1 {
                current_content = stripped_content;
                modified = true;
            }
            let content_after_strip = current_content.clone();

            if modified && can_format {
                let parent_dir = path
                    .parent()
                    .ok_or_else(|| format!("Failed to get parent dir for {}", path.display()))?;
                let suffix = path
                    .extension()
                    .map(|s| format!(".{}", s.to_string_lossy()))
                    .unwrap_or_default();
                let mut temp_file = tempfile::Builder::new()
                    .prefix(".xzen_fmt2_")
                    .suffix(&suffix)
                    .tempfile_in(parent_dir)
                    .map_err(map_err_to_string(path, "Create format 2 temp file"))?;
                temp_file
                    .write_all(content_after_strip.as_bytes())
                    .map_err(map_err_to_string(path, "Write format 2 temp file"))?;

                let temp_path_obj = temp_file.into_temp_path();
                run_formatter_for_lang(&temp_path_obj, lang)
                    .map_err(|e| format!("Final format failed for --all: {}", e))?;
                current_content = fs::read_to_string(&temp_path_obj)
                    .map_err(map_err_to_string(path, "Read temp file after final format"))?;
                drop(temp_path_obj);

                if current_content != content_after_strip {
                    modified = true;
                }
            }
        }
    }

    if modified {
        fs::write(path, current_content).map_err(map_err_to_string(path, "Write final result"))?;
    }
    Ok(())
}

fn run_formatter_for_lang(p: &Path, l: &str) -> Result<(), String> {
    match l {
        "rust" => run_rustfmt(p),
        "c" => run_astyle(p, "c"),
        "cpp" => run_astyle(p, "cpp"),
        "ruby" => run_rubocop_autocorrect(p),
        "toml" => run_taplo_fmt(p),
        "json" | "yaml" | "css" | "scss" | "less" | "markdown" | "javascript" | "typescript" => {
            run_prettierd_fmt(p)
        }
        "python" => run_black(p),
        "go" => run_gofmt(p),
        "lua" => run_stylua(p),
        "shell" | "bash" => run_shfmt(p),
        "fish" => run_fish_indent(p),
        "perl" => run_perltidy(p),
        "haskell" => run_ormolu(p),
        "cabal" => run_cabal_fmt(p),
        "elm" => run_elm_format(p),
        "crystal" => run_crystal_format(p),
        "java" => run_google_java_format(p),
        "kotlin" => run_ktlint(p),
        "swift" => run_swift_format(p),
        "html" | "xml" => run_tidy(p),
        "nix" => run_nixfmt(p),
        "twig" => run_djlint(p),
        "conf" => run_nginxfmt(p),
        "dockerfile" => run_dockfmt(p),
        "assembly" => run_asmfmt(p),
        _ => Ok(()),
    }
    .map_err(|e| e.to_string())
}

pub fn process_files(files: Vec<PathBuf>, mode: OperationMode) -> Result<Vec<ProcessedFileResult>> {
    let r: Vec<ProcessedFileResult> = files
        .par_iter()
        .map(|p| {
            let o = process_single_file(p, mode);
            ProcessedFileResult {
                path: p.clone(),
                error: o.err(),
            }
        })
        .collect();
    Ok(r)
}
