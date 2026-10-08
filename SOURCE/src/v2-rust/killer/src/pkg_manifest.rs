//! `killer.toml` (project manifest) and `killer.lock` (resolved dependency set).
//!
//! ```toml
//! [package]
//! name = "app"
//! version = "0.1.0"
//! description = "..."
//! entry = "src/main.killer"
//!
//! [registry]                      # optional default registry (file:///dir or http://host:port)
//! url = "file:///srv/killer-registry"
//!
//! [dependencies]
//! libA = "^1.2"                                   # registry, version requirement
//! libB = { path = "../libB" }                     # workspace / local path
//! libC = { git = "https://host/c.git", rev = "abc123" }   # also tag = / branch =
//! libD = { version = "~2.1", registry = "file:///other" }
//!
//! [dev-dependencies]
//! tester = "*"
//! ```

use crate::package_manager::{Version, VersionReq};
use crate::pkg_toml::{self, Table, Value};
use std::path::{Path, PathBuf};

pub const MANIFEST: &str = "killer.toml";
pub const LOCKFILE: &str = "killer.lock";

#[derive(Clone, Debug, PartialEq)]
pub enum DepSource {
    /// A registry; `None` means the project's default registry.
    Registry(Option<String>),
    /// A directory (relative to the manifest that declares it).
    Path(String),
    Git { url: String, reference: Option<String> },
}

#[derive(Clone, Debug, PartialEq)]
pub struct DepSpec {
    pub name: String,
    pub req: VersionReq,
    pub source: DepSource,
}

impl DepSpec {
    /// The right-hand side of `name = ...` in killer.toml.
    pub fn to_toml_value(&self) -> String {
        let simple = matches!(self.source, DepSource::Registry(None));
        if simple {
            return pkg_toml::quote(self.req.as_str());
        }
        let mut parts = Vec::new();
        match &self.source {
            DepSource::Path(p) => parts.push(format!("path = {}", pkg_toml::quote(p))),
            DepSource::Git { url, reference } => {
                parts.push(format!("git = {}", pkg_toml::quote(url)));
                if let Some(r) = reference {
                    parts.push(format!("rev = {}", pkg_toml::quote(r)));
                }
            }
            DepSource::Registry(Some(u)) => parts.push(format!("registry = {}", pkg_toml::quote(u))),
            DepSource::Registry(None) => {}
        }
        if self.req.as_str() != "*" {
            parts.push(format!("version = {}", pkg_toml::quote(self.req.as_str())));
        }
        format!("{{ {} }}", parts.join(", "))
    }
}

#[derive(Clone, Debug)]
pub struct Manifest {
    pub name: String,
    pub version: Version,
    pub description: String,
    pub entry: Option<String>,
    pub registry: Option<String>,
    pub dependencies: Vec<DepSpec>,
    pub dev_dependencies: Vec<DepSpec>,
}

pub fn valid_name(name: &str) -> bool {
    let mut cs = name.chars();
    matches!(cs.next(), Some(c) if c.is_ascii_alphabetic())
        && name.len() <= 64
        && name.chars().all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-')
}

impl Manifest {
    pub fn parse(text: &str) -> Result<Manifest, String> {
        let root = pkg_toml::parse(text)?;
        let pkg = root.get_table("package").ok_or("killer.toml: missing [package] section")?;
        let name = pkg.get_str("name").ok_or("killer.toml: [package] needs a name")?.to_string();
        if !valid_name(&name) {
            return Err(format!("killer.toml: invalid package name '{}' (letters, digits, '_' and '-', starting with a letter)", name));
        }
        let version = Version::parse(pkg.get_str("version").unwrap_or("0.1.0")).map_err(|e| format!("killer.toml: {}", e))?;
        Ok(Manifest {
            name,
            version,
            description: pkg.get_str("description").unwrap_or("").to_string(),
            entry: pkg.get_str("entry").map(|s| s.to_string()),
            registry: root.get_table("registry").and_then(|r| r.get_str("url")).map(|s| s.to_string()),
            dependencies: parse_deps(root.get_table("dependencies"))?,
            dev_dependencies: parse_deps(root.get_table("dev-dependencies"))?,
        })
    }

    pub fn load(dir: &Path) -> Result<Manifest, String> {
        let p = dir.join(MANIFEST);
        let text = std::fs::read_to_string(&p).map_err(|e| format!("cannot read {}: {}", p.display(), e))?;
        Manifest::parse(&text).map_err(|e| format!("{}: {}", p.display(), e))
    }

    /// Entry file relative to the package root: `entry`, else the first of the conventional names.
    pub fn entry_file(&self, dir: &Path) -> Option<PathBuf> {
        if let Some(e) = &self.entry {
            let p = dir.join(e);
            return p.is_file().then_some(p);
        }
        ["src/lib.killer", "src/main.killer", "lib.killer", "main.killer"]
            .iter()
            .map(|c| dir.join(c))
            .chain(std::iter::once(dir.join(format!("{}.killer", self.name))))
            .find(|p| p.is_file())
    }
}

fn parse_deps(t: Option<&Table>) -> Result<Vec<DepSpec>, String> {
    let Some(t) = t else { return Ok(Vec::new()) };
    let mut out = Vec::new();
    for (name, v) in &t.entries {
        if !valid_name(name) {
            return Err(format!("killer.toml: invalid dependency name '{}'", name));
        }
        let ctx = |e: String| format!("killer.toml: dependency '{}': {}", name, e);
        let spec = match v {
            Value::Str(s) => DepSpec { name: name.clone(), req: VersionReq::parse(s).map_err(ctx)?, source: DepSource::Registry(None) },
            Value::Table(d) => {
                let req = VersionReq::parse(d.get_str("version").unwrap_or("*")).map_err(ctx)?;
                let source = if let Some(p) = d.get_str("path") {
                    DepSource::Path(p.to_string())
                } else if let Some(g) = d.get_str("git") {
                    let reference = d.get_str("rev").or_else(|| d.get_str("tag")).or_else(|| d.get_str("branch")).map(|s| s.to_string());
                    DepSource::Git { url: g.to_string(), reference }
                } else {
                    DepSource::Registry(d.get_str("registry").map(|s| s.to_string()))
                };
                DepSpec { name: name.clone(), req, source }
            }
            _ => return Err(ctx("expected a version string or an inline table".to_string())),
        };
        out.push(spec);
    }
    Ok(out)
}

/// Find the project root: the nearest ancestor of `start` (inclusive) with a `killer.toml`.
pub fn find_root(start: &Path) -> Option<PathBuf> {
    let mut cur = Some(start);
    while let Some(d) = cur {
        if d.join(MANIFEST).is_file() {
            return Some(d.to_path_buf());
        }
        cur = d.parent();
    }
    None
}

#[derive(Clone, Debug, PartialEq)]
pub struct LockedPackage {
    pub name: String,
    pub version: String,
    /// `registry+<url>`, `path+<dir relative to the project>` or `git+<url>#<commit>`.
    pub source: String,
    pub sha256: String,
    /// `"name version"` of each direct dependency.
    pub dependencies: Vec<String>,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct Lockfile {
    pub packages: Vec<LockedPackage>,
}

impl Lockfile {
    pub fn parse(text: &str) -> Result<Lockfile, String> {
        let root = pkg_toml::parse(text)?;
        let mut packages = Vec::new();
        for item in root.get_array("package").map(|a| a.as_slice()).unwrap_or(&[]) {
            let Value::Table(t) = item else { continue };
            let field = |k: &str| t.get_str(k).map(|s| s.to_string()).ok_or_else(|| format!("{}: package entry is missing '{}'", LOCKFILE, k));
            packages.push(LockedPackage {
                name: field("name")?,
                version: field("version")?,
                source: field("source")?,
                sha256: field("sha256")?,
                dependencies: t
                    .get_array("dependencies")
                    .map(|a| a.iter().filter_map(|v| if let Value::Str(s) = v { Some(s.clone()) } else { None }).collect())
                    .unwrap_or_default(),
            });
        }
        Ok(Lockfile { packages })
    }

    pub fn to_text(&self) -> String {
        let mut sorted = self.packages.clone();
        sorted.sort_by(|a, b| a.name.cmp(&b.name));
        let mut root = Table::default();
        root.set("version", Value::Int(1));
        let arr = sorted
            .iter()
            .map(|p| {
                let mut t = Table::default();
                t.set("name", Value::Str(p.name.clone()));
                t.set("version", Value::Str(p.version.clone()));
                t.set("source", Value::Str(p.source.clone()));
                t.set("sha256", Value::Str(p.sha256.clone()));
                if !p.dependencies.is_empty() {
                    t.set("dependencies", Value::Array(p.dependencies.iter().map(|d| Value::Str(d.clone())).collect()));
                }
                Value::Table(t)
            })
            .collect::<Vec<_>>();
        if !arr.is_empty() {
            root.set("package", Value::Array(arr));
        }
        format!("# killer.lock - generated by killer_pkg; do not edit by hand\n{}", pkg_toml::to_string(&root))
    }

    pub fn load(dir: &Path) -> Result<Option<Lockfile>, String> {
        let p = dir.join(LOCKFILE);
        if !p.is_file() {
            return Ok(None);
        }
        let text = std::fs::read_to_string(&p).map_err(|e| format!("cannot read {}: {}", p.display(), e))?;
        Lockfile::parse(&text).map(Some).map_err(|e| format!("{}: {}", p.display(), e))
    }

    pub fn save(&self, dir: &Path) -> Result<(), String> {
        std::fs::write(dir.join(LOCKFILE), self.to_text()).map_err(|e| format!("cannot write {}: {}", LOCKFILE, e))
    }

    pub fn get(&self, name: &str) -> Option<&LockedPackage> {
        self.packages.iter().find(|p| p.name == name)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn manifest_round_trip_shapes() {
        let m = Manifest::parse(
            "[package]\nname = \"app\"\nversion = \"1.2.3\"\ndescription = \"d\"\nentry = \"src/main.killer\"\n[registry]\nurl = \"file:///r\"\n[dependencies]\na = \"^1.2\"\nb = { path = \"../b\" }\nc = { git = \"https://x/c.git\", rev = \"abc\" }\nd = { version = \"~2.1\", registry = \"file:///o\" }\n[dev-dependencies]\nt = \"*\"\n",
        )
        .unwrap();
        assert_eq!(m.name, "app");
        assert_eq!(m.registry.as_deref(), Some("file:///r"));
        assert_eq!(m.dependencies.len(), 4);
        assert_eq!(m.dependencies[1].source, DepSource::Path("../b".into()));
        assert_eq!(m.dependencies[2].source, DepSource::Git { url: "https://x/c.git".into(), reference: Some("abc".into()) });
        assert_eq!(m.dependencies[3].source, DepSource::Registry(Some("file:///o".into())));
        assert_eq!(m.dev_dependencies.len(), 1);
        for d in &m.dependencies {
            let again = parse_deps(pkg_toml::parse(&format!("[dependencies]\n{} = {}\n", d.name, d.to_toml_value())).unwrap().get_table("dependencies")).unwrap();
            assert_eq!(again[0], *d);
        }
    }

    #[test]
    fn manifest_errors_are_readable() {
        assert!(Manifest::parse("[dependencies]\n").unwrap_err().contains("[package]"));
        assert!(Manifest::parse("[package]\nname = \"../evil\"\n").unwrap_err().contains("invalid package name"));
        let e = Manifest::parse("[package]\nname = \"a\"\n[dependencies]\nb = \"^x\"\n").unwrap_err();
        assert!(e.contains("dependency 'b'") && e.contains("invalid version requirement"), "{e}");
    }

    #[test]
    fn lockfile_round_trip() {
        let l = Lockfile {
            packages: vec![
                LockedPackage { name: "b".into(), version: "1.0.0".into(), source: "path+../b".into(), sha256: "ab".into(), dependencies: vec![] },
                LockedPackage { name: "a".into(), version: "1.1.0".into(), source: "registry+file:///r".into(), sha256: "cd".into(), dependencies: vec!["b 1.0.0".into()] },
            ],
        };
        let again = Lockfile::parse(&l.to_text()).unwrap();
        assert_eq!(again.packages.len(), 2);
        assert_eq!(again.get("a").unwrap().dependencies, vec!["b 1.0.0".to_string()]);
        assert_eq!(again.packages[0].name, "a"); // sorted
    }
}
