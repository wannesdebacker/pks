use std::{
    env, fs,
    path::{Path, PathBuf},
    process::{exit, Command},
};

use inquire::Select;
use serde_json::Value;

const HELP: &str = "\
pks - pick and run package.json scripts

USAGE:
    pks [OPTIONS]                    pick a script interactively
    pks [OPTIONS] <script> [ARGS]    run a script; if no exact match, open the picker pre-filtered
    pks [OPTIONS] -- [ARGS]          pick interactively, forward ARGS to the script

OPTIONS:
    -l, --list       print scripts and their commands, then exit
    -n, --dry-run    print the command instead of running it
    -w, --where      print the path(s) of the package.json in use, then exit
    -h, --help       show this help
    -V, --version    show version

In a monorepo, scripts are listed in sections: the nearest package.json
first, then the workspace root, then every other workspace package. Each
script runs from the directory of the package it belongs to.

A workspace root is a parent directory with a pnpm-workspace.yaml or a
package.json containing a \"workspaces\" field. Packages are found through
its package globs (\"packages/*\", \"apps/**\", \"!excluded\", ...).

The package manager (pnpm, yarn, bun, npm) is detected from the lockfile.";

#[derive(Default)]
struct Opts {
    list: bool,
    dry_run: bool,
    show_path: bool,
    script: Option<String>,
    extra: Vec<String>,
}

struct Script {
    name: String,
    command: String,
    dir: PathBuf,
    section: String,
}

fn parse_args() -> Opts {
    let mut opts = Opts::default();
    let mut args = env::args().skip(1);

    while let Some(arg) = args.next() {
        match arg.as_str() {
            "-h" | "--help" => {
                println!("{HELP}");
                exit(0);
            }
            "-V" | "--version" => {
                println!("pks {}", env!("CARGO_PKG_VERSION"));
                exit(0);
            }
            "-l" | "--list" => opts.list = true,
            "-n" | "--dry-run" => opts.dry_run = true,
            "-w" | "--where" => opts.show_path = true,
            "--" => {
                opts.extra = args.collect();
                break;
            }
            flag if flag.starts_with('-') => {
                eprintln!("unknown option: {flag}\n\n{HELP}");
                exit(2);
            }
            _ => {
                opts.script = Some(arg);
                opts.extra = args.collect();
                break;
            }
        }
    }

    opts
}

fn find_package_json(start: &Path) -> Option<PathBuf> {
    start
        .ancestors()
        .map(|dir| dir.join("package.json"))
        .find(|path| path.is_file())
}

fn read_package(path: &Path) -> Value {
    let raw = fs::read_to_string(path).unwrap_or_else(|e| {
        eprintln!("cannot read {}: {e}", path.display());
        exit(1);
    });

    serde_json::from_str(&raw).unwrap_or_else(|e| {
        eprintln!("invalid {}: {e}", path.display());
        exit(1);
    })
}

fn is_workspace_root(dir: &Path) -> bool {
    if dir.join("pnpm-workspace.yaml").is_file() {
        return true;
    }

    let manifest = dir.join("package.json");
    manifest.is_file() && read_package(&manifest).get("workspaces").is_some()
}

/// Nearest ancestor above `package_dir` that declares a workspace.
fn find_workspace_root(package_dir: &Path) -> Option<PathBuf> {
    package_dir
        .ancestors()
        .skip(1)
        .find(|dir| is_workspace_root(dir))
        .map(Path::to_path_buf)
}

fn scripts_of(json: &Value, dir: &Path, section: &str) -> Vec<Script> {
    json.get("scripts")
        .and_then(Value::as_object)
        .into_iter()
        .flatten()
        .map(|(name, command)| Script {
            name: name.clone(),
            command: command.as_str().unwrap_or("").to_string(),
            dir: dir.to_path_buf(),
            section: section.to_string(),
        })
        .collect()
}

fn package_manager(dir: &Path) -> &'static str {
    let lockfiles = [
        ("pnpm-lock.yaml", "pnpm"),
        ("yarn.lock", "yarn"),
        ("bun.lockb", "bun"),
        ("bun.lock", "bun"),
    ];

    dir.ancestors()
        .find_map(|d| lockfiles.iter().find(|(file, _)| d.join(file).exists()))
        .map_or("npm", |(_, pm)| pm)
}

fn label(script: &Script, width: usize, tag_width: Option<usize>) -> String {
    let base = format!("{:<width$}  {}", script.name, script.command);

    match tag_width {
        Some(tag_width) => format!("{:<tag_width$}  {base}", script.section),
        None => base,
    }
}

fn print_list(scripts: &[Script], width: usize, sectioned: bool) {
    if !sectioned {
        scripts
            .iter()
            .for_each(|s| println!("{}", label(s, width, None)));
        return;
    }

    let mut current: Option<&str> = None;
    for script in scripts {
        if current != Some(script.section.as_str()) {
            if current.is_some() {
                println!();
            }
            println!("{}", script.section);
            current = Some(script.section.as_str());
        }
        println!("  {:<width$}  {}", script.name, script.command);
    }
}

fn glob_segment(pattern: &str, text: &str) -> bool {
    match pattern.split_once('*') {
        None => pattern == text,
        Some((head, tail)) => {
            let Some(rest) = text.strip_prefix(head) else {
                return false;
            };
            (0..=rest.len())
                .filter(|i| rest.is_char_boundary(*i))
                .any(|i| glob_segment(tail, &rest[i..]))
        }
    }
}

fn subdirs(dir: &Path) -> Vec<PathBuf> {
    let Ok(entries) = fs::read_dir(dir) else {
        return Vec::new();
    };

    let mut dirs: Vec<PathBuf> = entries
        .flatten()
        .filter(|e| {
            let name = e.file_name();
            let name = name.to_string_lossy();
            e.path().is_dir() && name != "node_modules" && !name.starts_with('.')
        })
        .map(|e| e.path())
        .collect();
    dirs.sort();
    dirs
}

fn expand_glob(dir: &Path, segments: &[&str], out: &mut Vec<PathBuf>) {
    let Some((first, rest)) = segments.split_first() else {
        out.push(dir.to_path_buf());
        return;
    };

    match *first {
        "." | "" => expand_glob(dir, rest, out),
        "**" => {
            expand_glob(dir, rest, out);
            for sub in subdirs(dir) {
                expand_glob(&sub, segments, out);
            }
        }
        segment => {
            for sub in subdirs(dir) {
                let name = sub.file_name().unwrap().to_string_lossy().into_owned();
                if glob_segment(segment, &name) {
                    expand_glob(&sub, rest, out);
                }
            }
        }
    }
}

fn workspace_patterns(root: &Path) -> Vec<String> {
    let mut patterns = Vec::new();

    if let Ok(yaml) = fs::read_to_string(root.join("pnpm-workspace.yaml")) {
        let mut in_packages = false;
        for line in yaml.lines() {
            let line = line.split(" #").next().unwrap_or("").trim_end();
            if line.trim().is_empty() {
                continue;
            }
            if !line.starts_with([' ', '-']) {
                in_packages = line.trim_start().starts_with("packages:");
                continue;
            }
            if let (true, Some(item)) = (in_packages, line.trim_start().strip_prefix('-')) {
                patterns.push(item.trim().trim_matches(['"', '\'']).to_string());
            }
        }
    }

    let manifest = root.join("package.json");
    if manifest.is_file() {
        let json = read_package(&manifest);
        let list = match json.get("workspaces") {
            Some(Value::Array(list)) => Some(list),
            Some(Value::Object(obj)) => obj.get("packages").and_then(Value::as_array),
            _ => None,
        };
        patterns.extend(
            list.into_iter()
                .flatten()
                .filter_map(|v| v.as_str().map(String::from)),
        );
    }

    patterns
}

fn workspace_packages(root: &Path) -> Vec<PathBuf> {
    let patterns = workspace_patterns(root);
    let expand = |pattern: &str| {
        let segments: Vec<&str> = pattern.trim_end_matches('/').split('/').collect();
        let mut out = Vec::new();
        expand_glob(root, &segments, &mut out);
        out
    };

    let mut included: Vec<PathBuf> = patterns
        .iter()
        .filter(|p| !p.starts_with('!'))
        .flat_map(|p| expand(p))
        .collect();
    let excluded: Vec<PathBuf> = patterns
        .iter()
        .filter_map(|p| p.strip_prefix('!'))
        .flat_map(expand)
        .collect();

    included
        .retain(|dir| dir != root && dir.join("package.json").is_file() && !excluded.contains(dir));
    included.sort();
    included.dedup();
    included
}

fn build_command(pm: &str, script: &Script, extra: &[String]) -> Command {
    let mut cmd = if cfg!(windows) {
        let mut cmd = Command::new("cmd");
        cmd.args(["/C", pm]);
        cmd
    } else {
        Command::new(pm)
    };

    cmd.args(["run", &script.name])
        .args(extra)
        .current_dir(&script.dir);
    cmd
}

#[cfg(unix)]
fn run(mut cmd: Command, pm: &str) -> ! {
    use std::os::unix::process::CommandExt;

    let err = cmd.exec();
    eprintln!("failed to run {pm}: {err}");
    exit(1);
}

#[cfg(not(unix))]
fn run(mut cmd: Command, pm: &str) -> ! {
    match cmd.status() {
        Ok(status) => exit(status.code().unwrap_or(1)),
        Err(err) => {
            eprintln!("failed to run {pm}: {err}");
            exit(1);
        }
    }
}

fn main() {
    let opts = parse_args();

    let cwd = env::current_dir().unwrap_or_else(|e| {
        eprintln!("cannot determine current directory: {e}");
        exit(1);
    });
    let Some(local_path) = find_package_json(&cwd) else {
        eprintln!("no package.json found");
        exit(1);
    };
    let local_dir = local_path.parent().unwrap();

    let root_dir = find_workspace_root(local_dir);
    let root_path = root_dir
        .as_ref()
        .map(|dir| dir.join("package.json"))
        .filter(|path| path.is_file());

    if opts.show_path {
        println!("{}", local_path.display());
        if let Some(root) = &root_path {
            println!("{}", root.display());
        }
        return;
    }

    let local_json = read_package(&local_path);
    let local_title = local_json
        .get("name")
        .and_then(Value::as_str)
        .map(String::from)
        .unwrap_or_else(|| {
            local_dir
                .file_name()
                .map_or("local".into(), |n| n.to_string_lossy().into_owned())
        });
    let mut scripts = scripts_of(&local_json, local_dir, &local_title);
    if let Some(root) = &root_path {
        scripts.extend(scripts_of(
            &read_package(root),
            root.parent().unwrap(),
            "workspace",
        ));
    }
    if let Some(root) = &root_dir {
        for dir in workspace_packages(root)
            .iter()
            .filter(|dir| dir.as_path() != local_dir)
        {
            let json = read_package(&dir.join("package.json"));
            let title = json
                .get("name")
                .and_then(Value::as_str)
                .map(String::from)
                .unwrap_or_else(|| dir.strip_prefix(root).unwrap_or(dir).display().to_string());
            scripts.extend(scripts_of(&json, dir, &title));
        }
    }

    if scripts.is_empty() {
        eprintln!("no scripts in package.json");
        exit(1);
    }

    let sectioned = root_dir.is_some();
    let width = scripts.iter().map(|s| s.name.len()).max().unwrap_or(0);
    let tag_width = sectioned.then(|| scripts.iter().map(|s| s.section.len()).max().unwrap_or(0));

    if opts.list {
        print_list(&scripts, width, sectioned);
        return;
    }

    let exact = opts
        .script
        .as_ref()
        .and_then(|wanted| scripts.iter().position(|s| &s.name == wanted));

    let index = match exact {
        Some(index) => index,
        None => {
            let options: Vec<String> = scripts.iter().map(|s| label(s, width, tag_width)).collect();
            let mut select = Select::new("run script", options).with_page_size(15);
            if let Some(filter) = &opts.script {
                select = select.with_starting_filter_input(filter);
            }

            let Ok(choice) = select.raw_prompt() else {
                return;
            };
            choice.index
        }
    };

    let script = &scripts[index];
    let pm = package_manager(&script.dir);

    if opts.dry_run {
        println!("{pm} run {} {}", script.name, opts.extra.join(" "));
        eprintln!("  (in {})", script.dir.display());
        return;
    }

    run(build_command(pm, script, &opts.extra), pm);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn glob_segment_matches_wildcards() {
        assert!(glob_segment("*", "anything"));
        assert!(glob_segment("app-*", "app-web"));
        assert!(glob_segment("*-ui", "web-ui"));
        assert!(glob_segment("a*b*c", "aXbYc"));
        assert!(!glob_segment("app-*", "web-app"));
        assert!(!glob_segment("exact", "exact2"));
    }

    fn fixture(name: &str) -> PathBuf {
        let root = env::temp_dir().join(format!("pks-test-{name}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(&root).unwrap();
        root
    }

    fn write(path: &Path, contents: &str) {
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(path, contents).unwrap();
    }

    #[test]
    fn finds_packages_from_npm_workspaces_with_exclusion() {
        let root = fixture("npm");
        write(
            &root.join("package.json"),
            r#"{"workspaces":["apps/*","libs/**","!libs/skip"]}"#,
        );
        write(&root.join("apps/web/package.json"), "{}");
        write(&root.join("libs/ui/package.json"), "{}");
        write(&root.join("libs/ui/deep/package.json"), "{}");
        write(&root.join("libs/skip/package.json"), "{}");

        let found = workspace_packages(&root);

        assert_eq!(
            found,
            vec![
                root.join("apps/web"),
                root.join("libs/ui"),
                root.join("libs/ui/deep"),
            ]
        );
        assert_eq!(
            find_workspace_root(&root.join("apps/web")),
            Some(root.clone())
        );
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn finds_packages_from_pnpm_workspace_yaml() {
        let root = fixture("pnpm");
        write(&root.join("package.json"), "{}");
        write(
            &root.join("pnpm-workspace.yaml"),
            "packages:\n  - 'packages/*' # comment\n  - \"!packages/old\"\nonlyBuiltDependencies:\n  - esbuild\n",
        );
        write(&root.join("packages/a/package.json"), "{}");
        write(&root.join("packages/old/package.json"), "{}");

        assert_eq!(workspace_packages(&root), vec![root.join("packages/a")]);
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn plain_package_has_no_workspace_root() {
        let root = fixture("plain");
        write(&root.join("package.json"), r#"{"scripts":{"a":"b"}}"#);

        assert_eq!(find_workspace_root(&root), None);
        fs::remove_dir_all(root).unwrap();
    }
}
