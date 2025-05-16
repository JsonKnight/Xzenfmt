use anyhow::Result;
use std::collections::HashSet;
use std::process::Command;

pub struct ToolInfo {
    pub name: &'static str,
    pub check_command: &'static [&'static str],
    pub install_hint: &'static str,
    pub languages: &'static [&'static str],
}

const TOOLS: &[ToolInfo] = &[
    ToolInfo {
        name: "rustfmt",
        check_command: &["rustfmt", "--version"],
        install_hint: "Install rustfmt component (e.g., 'rustup component add rustfmt')",
        languages: &["rust"],
    },
    ToolInfo {
        name: "astyle",
        check_command: &["astyle", "--version"],
        install_hint: "Install astyle (e.g., 'sudo apt install astyle', 'brew install astyle')",
        languages: &["c", "cpp"],
    },
    ToolInfo {
        name: "rubocop",
        check_command: &["rubocop", "--version"],
        install_hint: "Install rubocop (e.g., 'gem install rubocop')",
        languages: &["ruby"],
    },
    ToolInfo {
        name: "taplo",
        check_command: &["taplo", "--version"],
        install_hint: "Install taplo CLI (e.g., 'cargo install taplo-cli')",
        languages: &["toml"],
    },
    ToolInfo {
        name: "prettierd",
        check_command: &["prettierd", "--version"],
        install_hint: "Install prettierd (e.g., 'npm install -g prettier prettierd')",
        languages: &[
            "javascript",
            "typescript",
            "json",
            "yaml",
            "css",
            "scss",
            "less",
            "markdown",
        ],
    },
    ToolInfo {
        name: "asmfmt",
        check_command: &["asmfmt", "--version"],
        install_hint: "Install asmfmt (check project repo)",
        languages: &["assembly"],
    },
    ToolInfo {
        name: "crystal",
        check_command: &["crystal", "--version"],
        install_hint: "Install Crystal (crystal-lang.org)",
        languages: &["crystal"],
    },
    ToolInfo {
        name: "fish_indent",
        check_command: &["fish_indent", "--version"],
        install_hint: "Install fish shell (includes fish_indent)",
        languages: &["fish"],
    },
    ToolInfo {
        name: "shfmt",
        check_command: &["shfmt", "--version"],
        install_hint: "Install shfmt (e.g., 'apt install shfmt', 'brew install shfmt')",
        languages: &["shell"],
    },
    ToolInfo {
        name: "stylua",
        check_command: &["stylua", "--version"],
        install_hint: "Install stylua (e.g., 'cargo install stylua')",
        languages: &["lua"],
    },
    ToolInfo {
        name: "black",
        check_command: &["black", "--version"],
        install_hint: "Install black (e.g., 'pip install black')",
        languages: &["python"],
    },
    ToolInfo {
        name: "perltidy",
        check_command: &["perltidy", "--version"],
        install_hint: "Install perltidy (e.g., 'cpanm Perl::Tidy', package manager)",
        languages: &["perl"],
    },
    ToolInfo {
        name: "gofmt",
        check_command: &["gofmt", "-h"],
        install_hint: "Install Go (includes gofmt): https://golang.org/doc/install",
        languages: &["go"],
    },
    ToolInfo {
        name: "elm-format",
        check_command: &["elm-format", "--help"],
        install_hint: "Install elm-format (e.g., 'npm install -g elm-format')",
        languages: &["elm"],
    },
    ToolInfo {
        name: "ormolu",
        check_command: &["ormolu", "--version"],
        install_hint: "Install ormolu (e.g., 'cabal install ormolu')",
        languages: &["haskell"],
    },
    ToolInfo {
        name: "cabal-fmt",
        check_command: &["cabal-fmt", "--version"],
        install_hint: "Install cabal-fmt (e.g., 'cabal install cabal-fmt')",
        languages: &["cabal"],
    },
    ToolInfo {
        name: "tidy",
        check_command: &["tidy", "-v"],
        install_hint: "Install tidy-html5 (e.g., 'apt install tidy', 'brew install tidy-html5')",
        languages: &["html", "xml"],
    },
    ToolInfo {
        name: "nginxfmt",
        check_command: &["nginxfmt", "--version"],
        install_hint: "Install nginxfmt (check project repo)",
        languages: &["conf"],
    },
    ToolInfo {
        name: "nixfmt",
        check_command: &["nixfmt", "--version"],
        install_hint: "Install nixfmt (check project repo or Nix packages)",
        languages: &["nix"],
    },
    ToolInfo {
        name: "ktlint",
        check_command: &["ktlint", "--version"],
        install_hint: "Install ktlint (check project repo)",
        languages: &["kotlin"],
    },
    ToolInfo {
        name: "google-java-format",
        check_command: &["google-java-format", "--version"],
        install_hint: "Install google-java-format (download JAR or build plugins)",
        languages: &["java"],
    },
    ToolInfo {
        name: "swift-format",
        check_command: &["swift-format", "--version"],
        install_hint: "Install swift-format (check Swift toolchain/GitHub)",
        languages: &["swift"],
    },
    ToolInfo {
        name: "dockfmt",
        check_command: &["dockfmt", "--version"],
        install_hint: "Install dockfmt (check project repo)",
        languages: &["dockerfile"],
    },
    ToolInfo {
        name: "djlint",
        check_command: &["djlint", "--version"],
        install_hint: "Install djlint (e.g., 'pip install djlint')",
        languages: &["twig"],
    },
];

fn check_tool_command(command_parts: &[&str]) -> bool {
    if command_parts.is_empty() {
        return false;
    }
    Command::new(command_parts[0])
        .args(&command_parts[1..])
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .is_ok_and(|status| status.success())
}

fn check_single_tool(tool: &ToolInfo) -> (bool, String) {
    let found = check_tool_command(tool.check_command);
    let message = if found {
        format!("{}: OK", tool.name)
    } else {
        format!("{}: Not found. {}", tool.name, tool.install_hint)
    };
    (found, message)
}

pub fn check_dependencies(filter_langs: &[String]) -> Result<()> {
    println!("Checking dependencies:");
    let mut all_ok = true;
    let mut checked_tools = HashSet::new();
    let languages_to_filter: HashSet<&str> = filter_langs.iter().map(|s| s.as_str()).collect();
    let check_all = languages_to_filter.is_empty();
    let mut tools_needing_check = Vec::new();

    for tool in TOOLS {
        let lang_match = tool
            .languages
            .iter()
            .any(|lang| languages_to_filter.contains(lang));
        let tool_is_relevant = check_all || lang_match;
        if tool_is_relevant && checked_tools.insert(tool.name) {
            tools_needing_check.push(tool);
        }
    }

    if tools_needing_check.is_empty() {
        if !check_all {
            println!(
                "No specific formatter dependencies found for language(s): {:?}",
                filter_langs
            );
        } else {
            println!("No specific formatter dependencies found to check.");
        }
        return Ok(());
    }

    for tool in tools_needing_check {
        let (found, message) = check_single_tool(tool);
        println!("  {}", message);
        if !found {
            all_ok = false;
        }
    }

    if !all_ok {
        anyhow::bail!("One or more required formatters are missing.");
    } else {
        println!("All checked dependencies seem satisfied.");
    }
    Ok(())
}
