//! `killer_pkg` subcommands as a library: `pub fn run(args) -> exit code`.
//!
//! The `killer_pkg` binary calls `run(&std::env::args().skip(1))`; the `killer` driver can map
//! `killer new|init|add|remove|install|update|list|publish|check ...` to the same call
//! (`killer_native::pkg_cli::run(&args)` with the subcommand as `args[0]`).

use crate::package_manager::{Version, VersionReq};
use crate::pkg_files;
use crate::pkg_manifest::{valid_name, DepSource, DepSpec, LockedPackage, Lockfile, Manifest, LOCKFILE, MANIFEST};
use crate::pkg_registry::{self, Registry};
use crate::pkg_resolve::{self, Candidate, PackageIndex, Source};
use crate::pkg_toml;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::process::Command;

pub const USAGE: &str = "killer_pkg - the Killer package manager

USAGE:
  killer_pkg <command> [options]

COMMANDS:
  new <name> [--lib]        create a project in ./<name> (killer.toml, src/, tests/, .gitignore)
  init [--name N] [--lib]   create killer.toml (and src/) in the current directory
  add <name>[@req]          add a dependency: --path DIR | --git URL [--rev R] | --dep-registry URL | --dev
  remove <name>             remove a dependency (also from dev-dependencies)
  install                   resolve, download, verify sha256, write killer.lock, fill .killer/packages
  update [name]             re-resolve to the newest allowed versions (all packages, or one)
  list                      show direct and locked dependencies
  publish                   publish this package: --registry URL --token T [--dry-run]
  check                     verify killer.lock against the installed package contents
  help                      show this text

GLOBAL OPTIONS:
  -C, --dir DIR             operate on the project in DIR instead of the current directory
  --registry URL            registry (file:///path, a directory path or http://host:port);
                            default: $KILLER_REGISTRY, then [registry] url in killer.toml
  --token T                 publish token (default: $KILLER_REGISTRY_TOKEN)

REQUIREMENTS: ^1.2  ~1.2.3  >=1.0, <2.0  =1.2.3  1.2.3 (exact)  1.2  *
";

struct Args {
    pos: Vec<String>,
    flags: HashMap<String, String>,
    bools: Vec<String>,
}

const VALUE_FLAGS: &[&str] = &["--registry", "--token", "--path", "--git", "--rev", "--name", "--dir", "-C", "--tag", "--branch", "--dep-registry"];

fn parse_args(args: &[String]) -> Result<Args, String> {
    let mut a = Args { pos: Vec::new(), flags: HashMap::new(), bools: Vec::new() };
    let mut i = 0;
    while i < args.len() {
        let s = &args[i];
        if VALUE_FLAGS.contains(&s.as_str()) {
            let v = args.get(i + 1).ok_or_else(|| format!("option {} needs a value", s))?;
            let key = if s == "-C" { "--dir" } else { s.as_str() };
            a.flags.insert(key.to_string(), v.clone());
            i += 2;
        } else if s.starts_with("--") {
            a.bools.push(s.clone());
            i += 1;
        } else {
            a.pos.push(s.clone());
            i += 1;
        }
    }
    Ok(a)
}

impl Args {
    fn flag(&self, k: &str) -> Option<&str> {
        self.flags.get(k).map(|s| s.as_str())
    }
    fn has(&self, k: &str) -> bool {
        self.bools.iter().any(|b| b == k)
    }
}

/// Run a subcommand. `args[0]` is the subcommand; returns the process exit code.
pub fn run(args: &[String]) -> i32 {
    match run_inner(args) {
        Ok(()) => 0,
        Err(e) => {
            eprintln!("error: {}", e);
            1
        }
    }
}

fn run_inner(args: &[String]) -> Result<(), String> {
    let mut a = parse_args(args)?;
    if a.pos.is_empty() {
        if a.has("--help") || a.has("--version") {
            print!("{}", USAGE);
            return Ok(());
        }
        print!("{}", USAGE);
        return Err("no command given".to_string());
    }
    let cmd = a.pos.remove(0);
    let dir = match a.flag("--dir") {
        Some(d) => PathBuf::from(d),
        None => std::env::current_dir().map_err(|e| e.to_string())?,
    };
    match cmd.as_str() {
        "help" => {
            print!("{}", USAGE);
            Ok(())
        }
        "new" => cmd_new(&a, &dir),
        "init" => cmd_init(&a, &dir),
        "add" => cmd_add(&a, &dir),
        "remove" | "rm" => cmd_remove(&a, &dir),
        "install" | "i" => cmd_install(&a, &dir, false, None).map(|_| ()),
        "update" | "up" => cmd_install(&a, &dir, true, a.pos.first().map(|s| s.as_str())).map(|_| ()),
        "list" | "ls" => cmd_list(&dir),
        "publish" => cmd_publish(&a, &dir),
        "check" => cmd_check(&dir),
        other => Err(format!("unknown command '{}'; run `killer_pkg help`", other)),
    }
}

// ---------------------------------------------------------------- scaffolding

fn ident(name: &str) -> String {
    name.replace('-', "_")
}

fn scaffold(dir: &Path, name: &str, lib: bool) -> Result<(), String> {
    let entry = if lib { "src/lib.killer" } else { "src/main.killer" };
    let mk = |p: &str, body: &str| -> Result<(), String> {
        let f = dir.join(p);
        if let Some(parent) = f.parent() {
            std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
        }
        if !f.exists() {
            std::fs::write(&f, body).map_err(|e| format!("cannot write {}: {}", f.display(), e))?;
        }
        Ok(())
    };
    std::fs::create_dir_all(dir).map_err(|e| format!("cannot create {}: {}", dir.display(), e))?;
    mk(MANIFEST, &format!("[package]\nname = {}\nversion = \"0.1.0\"\ndescription = \"\"\nentry = \"{}\"\n\n[dependencies]\n", pkg_toml::quote(name), entry))?;
    if lib {
        mk(entry, &format!("fn {id}_hello() {{\n  return \"hello from {name}\"\n}}\n", id = ident(name), name = name))?;
    } else {
        mk(entry, &format!("println(\"Hello from {}\")\n", name))?;
    }
    mk("tests/basic.killer", "test \"arithmetic works\" {\n  assert_eq(1 + 1, 2)\n}\n")?;
    let gi = dir.join(".gitignore");
    let existing = std::fs::read_to_string(&gi).unwrap_or_default();
    if !existing.lines().any(|l| l.trim() == ".killer/") {
        let mut s = existing;
        if !s.is_empty() && !s.ends_with('\n') {
            s.push('\n');
        }
        s.push_str(".killer/\ntarget/\n");
        std::fs::write(&gi, s).map_err(|e| e.to_string())?;
    }
    Ok(())
}

fn cmd_new(a: &Args, cwd: &Path) -> Result<(), String> {
    let name = a.pos.first().ok_or("usage: killer_pkg new <name> [--lib]")?;
    if !valid_name(name) {
        return Err(format!("invalid package name '{}' (letters, digits, '_' and '-', starting with a letter)", name));
    }
    let dir = cwd.join(name);
    if dir.exists() {
        return Err(format!("{} already exists", dir.display()));
    }
    scaffold(&dir, name, a.has("--lib"))?;
    println!("Created {} `{}` in {}", if a.has("--lib") { "library" } else { "application" }, name, dir.display());
    Ok(())
}

fn cmd_init(a: &Args, dir: &Path) -> Result<(), String> {
    if dir.join(MANIFEST).exists() {
        return Err(format!("{} already exists", dir.join(MANIFEST).display()));
    }
    let raw = match a.flag("--name") {
        Some(n) => n.to_string(),
        None => dir
            .canonicalize()
            .ok()
            .and_then(|p| p.file_name().map(|f| f.to_string_lossy().to_string()))
            .unwrap_or_else(|| "app".to_string()),
    };
    let mut name: String = raw.chars().map(|c| if c.is_ascii_alphanumeric() || c == '_' || c == '-' { c } else { '_' }).collect();
    if !name.chars().next().map_or(false, |c| c.is_ascii_alphabetic()) {
        name = format!("p{}", name);
    }
    scaffold(dir, &name, a.has("--lib"))?;
    println!("Initialized package `{}` in {}", name, dir.display());
    Ok(())
}

// ---------------------------------------------------------------- registries

fn registry_url(a: &Args, manifest: Option<&Manifest>) -> Option<String> {
    a.flag("--registry")
        .map(|s| s.to_string())
        .or_else(|| std::env::var("KILLER_REGISTRY").ok().filter(|s| !s.is_empty()))
        .or_else(|| manifest.and_then(|m| m.registry.clone()))
}

fn open_registry(url: &str, project: &Path) -> Result<Registry, String> {
    match pkg_registry::open(url)? {
        Registry::Dir(p) if p.is_relative() => Ok(Registry::Dir(pkg_files::normalize(&project.join(p)))),
        r => Ok(r),
    }
}

// ---------------------------------------------------------------- add / remove

fn cmd_add(a: &Args, dir: &Path) -> Result<(), String> {
    let arg = a.pos.first().ok_or("usage: killer_pkg add <name>[@requirement] [--path DIR | --git URL | --dep-registry URL] [--dev]")?;
    let (name, req_text) = match arg.split_once('@') {
        Some((n, r)) => (n.to_string(), Some(r.to_string())),
        None => (arg.clone(), None),
    };
    if !valid_name(&name) {
        return Err(format!("invalid package name '{}'", name));
    }
    let manifest = Manifest::load(dir)?;
    let section = if a.has("--dev") { "dev-dependencies" } else { "dependencies" };
    let source = if let Some(p) = a.flag("--path") {
        DepSource::Path(p.replace('\\', "/"))
    } else if let Some(g) = a.flag("--git") {
        DepSource::Git { url: g.to_string(), reference: a.flag("--rev").or(a.flag("--tag")).or(a.flag("--branch")).map(|s| s.to_string()) }
    } else {
        DepSource::Registry(a.flag("--dep-registry").map(|s| s.to_string()))
    };
    let req = match (&req_text, &source) {
        (Some(r), _) => VersionReq::parse(r)?,
        (None, DepSource::Registry(own)) => {
            // newest published version, as a caret requirement
            let url = own.clone().or_else(|| registry_url(a, Some(&manifest))).ok_or("no registry configured: pass --registry, set KILLER_REGISTRY or add [registry] url to killer.toml")?;
            let reg = open_registry(&url, dir)?;
            let latest = reg.versions(&name)?.into_iter().max().ok_or_else(|| format!("package `{}` was not found in registry {}", name, reg.url()))?;
            VersionReq::parse(&format!("^{}", latest))?
        }
        (None, _) => VersionReq::any(),
    };
    let spec = DepSpec { name: name.clone(), req, source };
    let text = std::fs::read_to_string(dir.join(MANIFEST)).map_err(|e| e.to_string())?;
    let new_text = pkg_toml::set_in_section(&text, section, &name, &spec.to_toml_value());
    Manifest::parse(&new_text).map_err(|e| format!("refusing to write an invalid manifest: {}", e))?;
    std::fs::write(dir.join(MANIFEST), new_text).map_err(|e| e.to_string())?;
    println!("Added {} {} to [{}]. Run `killer_pkg install` to fetch it.", name, spec.to_toml_value(), section);
    Ok(())
}

fn cmd_remove(a: &Args, dir: &Path) -> Result<(), String> {
    let name = a.pos.first().ok_or("usage: killer_pkg remove <name>")?;
    let mut text = std::fs::read_to_string(dir.join(MANIFEST)).map_err(|e| format!("cannot read {}: {}", dir.join(MANIFEST).display(), e))?;
    let mut any = false;
    for section in ["dependencies", "dev-dependencies"] {
        let (t, removed) = pkg_toml::remove_from_section(&text, section, name);
        text = t;
        any |= removed;
    }
    if !any {
        return Err(format!("`{}` is not a dependency of this package", name));
    }
    std::fs::write(dir.join(MANIFEST), text).map_err(|e| e.to_string())?;
    println!("Removed {}. Run `killer_pkg install` to update killer.lock.", name);
    Ok(())
}

// ---------------------------------------------------------------- the real index

struct Index {
    project: PathBuf,
    default_registry: Option<String>,
    locked_git: HashMap<String, String>,
    registries: HashMap<String, Registry>,
    git_dirs: HashMap<String, PathBuf>,
}

impl Index {
    fn registry(&mut self, url: Option<&String>, name: &str) -> Result<Registry, String> {
        let url = url.cloned().or_else(|| self.default_registry.clone()).ok_or_else(|| {
            format!("no registry configured for `{}`: pass --registry, set KILLER_REGISTRY or add [registry] url to killer.toml", name)
        })?;
        if let Some(r) = self.registries.get(&url) {
            return Ok(r.clone());
        }
        let r = open_registry(&url, &self.project)?;
        self.registries.insert(url, r.clone());
        Ok(r)
    }

    fn checkout(&mut self, name: &str, url: &str, reference: Option<&str>) -> Result<(PathBuf, String), String> {
        let dest = self.project.join(".killer").join("git").join(name);
        let pinned = self.locked_git.get(name).cloned();
        let target = pinned.as_deref().or(reference);
        let _ = std::fs::remove_dir_all(&dest);
        std::fs::create_dir_all(dest.parent().unwrap()).map_err(|e| e.to_string())?;
        run_git(None, &["clone", "--quiet", url, &dest.to_string_lossy()])?;
        if let Some(t) = target {
            run_git(Some(&dest), &["checkout", "--quiet", t])?;
        }
        let commit = run_git(Some(&dest), &["rev-parse", "HEAD"])?.trim().to_string();
        self.git_dirs.insert(name.to_string(), dest.clone());
        Ok((dest, commit))
    }
}

fn run_git(dir: Option<&Path>, args: &[&str]) -> Result<String, String> {
    let mut c = Command::new("git");
    if let Some(d) = dir {
        c.current_dir(d);
    }
    let out = c.args(args).output().map_err(|e| format!("cannot run git: {}", e))?;
    if !out.status.success() {
        return Err(format!("git {} failed: {}", args.join(" "), String::from_utf8_lossy(&out.stderr).trim()));
    }
    Ok(String::from_utf8_lossy(&out.stdout).to_string())
}

impl PackageIndex for Index {
    fn candidates(&mut self, name: &str, spec: &DepSpec, from_dir: &Path) -> Result<Vec<Candidate>, String> {
        match &spec.source {
            DepSource::Registry(url) => {
                let reg = self.registry(url.as_ref(), name)?;
                let versions = reg.versions(name)?;
                if versions.is_empty() {
                    return Err(format!("package `{}` was not found in registry {}", name, reg.url()));
                }
                Ok(versions.into_iter().map(|version| Candidate { version, source: Source::Registry(reg.url()) }).collect())
            }
            DepSource::Path(p) => {
                let dir = pkg_files::normalize(&from_dir.join(p));
                let m = Manifest::load(&dir).map_err(|e| format!("path dependency `{}` ({}): {}", name, p, e))?;
                if m.name != name {
                    return Err(format!("{} contains package `{}` but it is required as `{}`", dir.display(), m.name, name));
                }
                Ok(vec![Candidate { version: m.version, source: Source::Path(dir) }])
            }
            DepSource::Git { url, reference } => {
                let (dir, commit) = self.checkout(name, url, reference.as_deref())?;
                let m = Manifest::load(&dir).map_err(|e| format!("git dependency `{}` ({}): {}", name, url, e))?;
                Ok(vec![Candidate { version: m.version, source: Source::Git { url: url.clone(), commit } }])
            }
        }
    }

    fn manifest(&mut self, name: &str, cand: &Candidate) -> Result<Manifest, String> {
        match &cand.source {
            Source::Registry(url) => {
                let reg = self.registry(Some(url), name)?;
                Manifest::parse(&reg.manifest(name, &cand.version)?)
            }
            Source::Path(p) => Manifest::load(p),
            Source::Git { .. } => Manifest::load(self.git_dirs.get(name).ok_or("git checkout missing")?),
        }
    }

    fn package_dir(&mut self, name: &str, cand: &Candidate) -> Option<PathBuf> {
        match &cand.source {
            Source::Path(p) => Some(p.clone()),
            Source::Git { .. } => self.git_dirs.get(name).cloned(),
            Source::Registry(_) => None,
        }
    }
}

// ---------------------------------------------------------------- install / update

pub fn packages_dir(project: &Path) -> PathBuf {
    project.join(".killer").join("packages")
}

fn cmd_install(a: &Args, dir: &Path, update: bool, only: Option<&str>) -> Result<Lockfile, String> {
    let manifest = Manifest::load(dir)?;
    let old = Lockfile::load(dir)?;
    let mut prefer: HashMap<String, Version> = HashMap::new();
    let mut locked_git = HashMap::new();
    if let Some(l) = &old {
        for p in &l.packages {
            if update && only.map_or(true, |o| o == p.name) {
                continue; // let this one float
            }
            if let Ok(v) = Version::parse(&p.version) {
                prefer.insert(p.name.clone(), v);
            }
            if let Some(r) = p.source.strip_prefix("git+") {
                if let Some((_, c)) = r.rsplit_once('#') {
                    locked_git.insert(p.name.clone(), c.to_string());
                }
            }
        }
    }
    if let Some(o) = only {
        if !manifest.dependencies.iter().chain(&manifest.dev_dependencies).any(|d| d.name == o) && old.as_ref().and_then(|l| l.get(o)).is_none() {
            return Err(format!("`{}` is not a dependency of this package", o));
        }
    }
    let mut index = Index { project: dir.to_path_buf(), default_registry: registry_url(a, Some(&manifest)), locked_git, registries: HashMap::new(), git_dirs: HashMap::new() };
    let resolved = pkg_resolve::resolve(&manifest, dir, true, &mut index, &prefer)?;

    let pkgs_dir = packages_dir(dir);
    std::fs::create_dir_all(&pkgs_dir).map_err(|e| e.to_string())?;
    let mut locked = Vec::new();
    let mut keep = Vec::new();
    for r in &resolved {
        let source_text = pkg_resolve::source_to_lock(&r.source, dir);
        let prior = old.as_ref().and_then(|l| l.get(&r.name)).filter(|p| p.version == r.version.to_string() && p.source == source_text);
        let sha = match &r.source {
            Source::Registry(url) => {
                let reg = open_registry(url, dir)?;
                let rel = reg.fetch(&r.name, &r.version)?;
                let files = pkg_files::unpack(&rel.pack)?;
                let actual = pkg_files::content_hash(&files);
                if actual != rel.sha256 {
                    return Err(format!("checksum mismatch for {} {}: the registry says {} but the downloaded archive hashes to {}", r.name, r.version, rel.sha256, actual));
                }
                if let Some(p) = prior {
                    if p.sha256 != actual {
                        return Err(format!("checksum mismatch for {} {}: killer.lock records {} but the registry now serves {} (was the release tampered with?)", r.name, r.version, p.sha256, actual));
                    }
                }
                let dest = pkgs_dir.join(&r.name);
                if pkg_files::hash_dir(&dest).ok().as_deref() != Some(actual.as_str()) {
                    pkg_files::write_files(&dest, &files)?;
                }
                keep.push(r.name.clone());
                actual
            }
            Source::Git { .. } => {
                let src = index.git_dirs.get(&r.name).cloned().ok_or("git checkout missing")?;
                let files = pkg_files::read_files(&src)?;
                let actual = pkg_files::content_hash(&files);
                if let Some(p) = prior {
                    if p.sha256 != actual {
                        return Err(format!("checksum mismatch for {} {}: killer.lock records {} but the checkout hashes to {}", r.name, r.version, p.sha256, actual));
                    }
                }
                pkg_files::write_files(&pkgs_dir.join(&r.name), &files)?;
                keep.push(r.name.clone());
                actual
            }
            Source::Path(p) => pkg_files::hash_dir(p)?,
        };
        locked.push(LockedPackage { name: r.name.clone(), version: r.version.to_string(), source: source_text, sha256: sha, dependencies: r.deps.clone() });
    }
    // drop installed packages that are no longer needed
    if let Ok(rd) = std::fs::read_dir(&pkgs_dir) {
        for e in rd.flatten() {
            let n = e.file_name().to_string_lossy().to_string();
            if !keep.contains(&n) && e.path().is_dir() {
                let _ = std::fs::remove_dir_all(e.path());
            }
        }
    }
    let lock = Lockfile { packages: locked };
    lock.save(dir)?;
    println!("Installed {} package(s); wrote {}", lock.packages.len(), LOCKFILE);
    for p in &lock.packages {
        println!("  {} {}  sha256 {}", p.name, p.version, &p.sha256[..12.min(p.sha256.len())]);
    }
    Ok(lock)
}

// ---------------------------------------------------------------- list / publish / check

fn cmd_list(dir: &Path) -> Result<(), String> {
    let m = Manifest::load(dir)?;
    let lock = Lockfile::load(dir)?;
    println!("{} {}", m.name, m.version);
    for (label, deps) in [("dependencies", &m.dependencies), ("dev-dependencies", &m.dev_dependencies)] {
        if deps.is_empty() {
            continue;
        }
        println!("[{}]", label);
        for d in deps {
            let locked = lock.as_ref().and_then(|l| l.get(&d.name));
            match locked {
                Some(p) => println!("  {} {}  -> {} ({})", d.name, d.to_toml_value(), p.version, p.source),
                None => println!("  {} {}  (not installed)", d.name, d.to_toml_value()),
            }
        }
    }
    if let Some(l) = &lock {
        let direct: Vec<&str> = m.dependencies.iter().chain(&m.dev_dependencies).map(|d| d.name.as_str()).collect();
        let trans: Vec<&LockedPackage> = l.packages.iter().filter(|p| !direct.contains(&p.name.as_str())).collect();
        if !trans.is_empty() {
            println!("[transitive]");
            for p in trans {
                println!("  {} {} ({})", p.name, p.version, p.source);
            }
        }
    }
    Ok(())
}

fn cmd_publish(a: &Args, dir: &Path) -> Result<(), String> {
    let m = Manifest::load(dir)?;
    if m.dependencies.iter().any(|d| matches!(d.source, DepSource::Path(_))) {
        return Err("a published package cannot have path dependencies; replace them with registry versions first".to_string());
    }
    if m.entry_file(dir).is_none() {
        return Err(format!("no entry file found for `{}` (set `entry` in [package])", m.name));
    }
    let files = pkg_files::read_files(dir)?;
    let hash = pkg_files::content_hash(&files);
    let pack = pkg_files::pack(&files);
    if a.has("--dry-run") {
        println!("Would publish {} {} ({} files, {} bytes) sha256 {}", m.name, m.version, files.len(), pack.len(), hash);
        for (p, _) in &files {
            println!("  {}", p);
        }
        return Ok(());
    }
    let url = registry_url(a, Some(&m)).ok_or("no registry configured: pass --registry, set KILLER_REGISTRY or add [registry] url to killer.toml")?;
    let reg = open_registry(&url, dir)?;
    let token = a.flag("--token").map(|s| s.to_string()).or_else(|| std::env::var("KILLER_REGISTRY_TOKEN").ok());
    reg.publish(&m.name, &m.version, &pack, token.as_deref())?;
    println!("Published {} {} to {} (sha256 {})", m.name, m.version, reg.url(), hash);
    Ok(())
}

fn cmd_check(dir: &Path) -> Result<(), String> {
    let m = Manifest::load(dir)?;
    let lock = Lockfile::load(dir)?.ok_or_else(|| format!("no {} found; run `killer_pkg install` first", LOCKFILE))?;
    let mut problems: Vec<String> = Vec::new();
    for d in m.dependencies.iter().chain(&m.dev_dependencies) {
        match lock.get(&d.name) {
            None => problems.push(format!("{} is in killer.toml but not in {}; run `killer_pkg install`", d.name, LOCKFILE)),
            Some(p) => {
                if let (DepSource::Registry(_), Ok(v)) = (&d.source, Version::parse(&p.version)) {
                    if !d.req.matches(&v) {
                        problems.push(format!("{} is locked at {} which does not satisfy {}; run `killer_pkg install`", d.name, p.version, d.req));
                    }
                }
            }
        }
    }
    for p in &lock.packages {
        let loc = pkg_resolve::source_from_lock(&p.source, dir);
        let target = match &loc {
            Some(Source::Path(path)) => path.clone(),
            Some(_) => packages_dir(dir).join(&p.name),
            None => {
                problems.push(format!("{} {}: unrecognized source '{}'", p.name, p.version, p.source));
                continue;
            }
        };
        if !target.is_dir() {
            problems.push(format!("{} {} is not installed at {}; run `killer_pkg install`", p.name, p.version, target.display()));
            continue;
        }
        match pkg_files::hash_dir(&target) {
            Ok(h) if h == p.sha256 => {}
            Ok(h) => problems.push(format!("{} {}: checksum mismatch in {}: killer.lock has {} but the files hash to {}", p.name, p.version, target.display(), p.sha256, h)),
            Err(e) => problems.push(format!("{} {}: {}", p.name, p.version, e)),
        }
    }
    if problems.is_empty() {
        println!("ok: {} package(s) match {}", lock.packages.len(), LOCKFILE);
        Ok(())
    } else {
        for p in &problems {
            eprintln!("  {}", p);
        }
        Err(format!("{} problem(s) found", problems.len()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn args_parse_flags_and_positionals() {
        let a = parse_args(&["libA@^1".to_string(), "--registry".to_string(), "x".to_string(), "--dev".to_string(), "-C".to_string(), "d".to_string()]).unwrap();
        assert_eq!(a.pos, vec!["libA@^1".to_string()]);
        assert_eq!(a.flag("--registry"), Some("x"));
        assert_eq!(a.flag("--dir"), Some("d"));
        assert!(a.has("--dev"));
        assert!(parse_args(&["--token".to_string()]).is_err());
    }
}
