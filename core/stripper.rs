pub mod c_family;
pub mod common;
pub mod crystal;
pub mod fish;
pub mod haskell_elm;
pub mod json;
pub mod lua;
pub mod nix;
pub mod python;
pub mod ruby;
pub mod rust;
pub mod shell;
pub mod toml;
pub mod twig;
pub mod yaml;

pub use common::{CommentMatch, StripError, remove_matches};

use std::path::Path;

pub fn find_language_comments(
    content: &str,
    lang: &str,
    _path: &Path,
) -> Result<Vec<CommentMatch>, StripError> {
    match lang {
        "c" | "cpp" | "go" | "javascript" | "typescript" | "css" | "scss" | "less" | "java"
        | "kotlin" | "swift" => c_family::find_comments(content),

        "rust" => rust::find_comments(content),

        "json" => json::find_comments(content),

        "ruby" => ruby::find_comments(content),

        "crystal" => crystal::find_comments(content),

        "fish" => fish::find_comments(content),

        "shell" | "dockerfile" | "conf" | "perl" | "bash" => shell::find_comments(content),

        "python" => python::find_comments(content),

        "yaml" | "yml" => yaml::find_comments(content),

        "toml" => toml::find_comments(content),

        "lua" => lua::find_comments(content),

        "haskell" | "elm" => haskell_elm::find_comments(content),

        "nix" => nix::find_comments(content),

        "twig" => twig::find_comments(content),

        _ => Ok(Vec::new()),
    }
}
