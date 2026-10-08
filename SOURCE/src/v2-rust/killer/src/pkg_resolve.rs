//! Dependency resolution: pick one version of every package so that all requirements hold.
//!
//! Depth-first with backtracking, newest candidate first (locked versions first when a
//! lockfile is being respected). Failures name the packages and requirements involved.

use crate::package_manager::Version;
use crate::pkg_manifest::{DepSource, DepSpec, Manifest};
use std::collections::{BTreeMap, HashMap};
use std::path::{Path, PathBuf};

/// Where a resolved package comes from.
#[derive(Clone, Debug, PartialEq)]
pub enum Source {
    Registry(String),
    /// Absolute (lexically normalized) directory.
    Path(PathBuf),
    Git { url: String, commit: String },
}

#[derive(Clone, Debug)]
pub struct Candidate {
    pub version: Version,
    pub source: Source,
}

/// Where packages and their manifests are looked up.
pub trait PackageIndex {
    /// Every candidate for `spec` (any order). `from_dir` is the directory of the requirer, used
    /// to resolve relative `path` dependencies.
    fn candidates(&mut self, name: &str, spec: &DepSpec, from_dir: &Path) -> Result<Vec<Candidate>, String>;
    fn manifest(&mut self, name: &str, cand: &Candidate) -> Result<Manifest, String>;
    /// Directory of a candidate whose own relative path dependencies must be resolved (path/git).
    fn package_dir(&mut self, name: &str, cand: &Candidate) -> Option<PathBuf>;
}

#[derive(Clone, Debug)]
pub struct Resolved {
    pub name: String,
    pub version: Version,
    pub source: Source,
    /// `"name version"` of direct dependencies.
    pub deps: Vec<String>,
}

#[derive(Clone)]
struct Req {
    by: String,
    spec: DepSpec,
    from_dir: PathBuf,
}

#[derive(Clone, Default)]
struct State {
    reqs: BTreeMap<String, Vec<Req>>,
    chosen: BTreeMap<String, Candidate>,
    deps: BTreeMap<String, Vec<String>>,
}

struct Solver<'a> {
    index: &'a mut dyn PackageIndex,
    prefer: &'a HashMap<String, Version>,
    cache: HashMap<String, Vec<Candidate>>,
}

/// Resolve the dependencies of `root` (which lives in `root_dir`). `prefer` holds versions from an
/// existing lockfile that should be kept while they still satisfy every requirement.
pub fn resolve(
    root: &Manifest,
    root_dir: &Path,
    include_dev: bool,
    index: &mut dyn PackageIndex,
    prefer: &HashMap<String, Version>,
) -> Result<Vec<Resolved>, String> {
    let mut st = State::default();
    let who = root.name.clone();
    let mut direct: Vec<&DepSpec> = root.dependencies.iter().collect();
    if include_dev {
        direct.extend(root.dev_dependencies.iter());
    }
    for d in direct {
        add_req(&mut st, Req { by: who.clone(), spec: d.clone(), from_dir: root_dir.to_path_buf() });
    }
    let mut solver = Solver { index, prefer, cache: HashMap::new() };
    let done = solver.solve(st)?;
    Ok(done
        .chosen
        .into_iter()
        .map(|(name, c)| Resolved { deps: done.deps.get(&name).cloned().unwrap_or_default(), name, version: c.version, source: c.source })
        .collect())
}

fn add_req(st: &mut State, r: Req) {
    st.reqs.entry(r.spec.name.clone()).or_default().push(r);
}

fn describe(r: &Req) -> String {
    let what = match &r.spec.source {
        DepSource::Path(p) => format!("path \"{}\"", p),
        DepSource::Git { url, .. } => format!("git {}", url),
        DepSource::Registry(_) => r.spec.req.to_string(),
    };
    format!("{} requires {} {}", r.by, r.spec.name, what)
}

fn conflict(name: &str, reqs: &[Req], available: &[Candidate], extra: &str) -> String {
    let mut msg = format!("version conflict on `{}`:", name);
    for r in reqs {
        msg.push_str(&format!("\n  {}", describe(r)));
    }
    if !extra.is_empty() {
        msg.push_str(&format!("\n  {}", extra));
    }
    if available.is_empty() {
        msg.push_str("\n  no versions are available");
    } else {
        let mut vs: Vec<&Version> = available.iter().map(|c| &c.version).collect();
        vs.sort();
        msg.push_str(&format!("\n  available versions: {}", vs.iter().map(|v| v.to_string()).collect::<Vec<_>>().join(", ")));
    }
    msg
}

impl<'a> Solver<'a> {
    fn candidates_for(&mut self, name: &str, reqs: &[Req]) -> Result<Vec<Candidate>, String> {
        if let Some(c) = self.cache.get(name) {
            return Ok(c.clone());
        }
        // a path or git requirement decides where the package comes from
        let pinned: Vec<&Req> = reqs.iter().filter(|r| !matches!(r.spec.source, DepSource::Registry(_))).collect();
        if pinned.len() > 1 && pinned.iter().any(|r| r.spec.source != pinned[0].spec.source || r.from_dir != pinned[0].from_dir) {
            let owned: Vec<Req> = pinned.iter().map(|r| (*r).clone()).collect();
            return Err(conflict(name, &owned, &[], "it is required from different sources"));
        }
        let def = pinned.first().copied().unwrap_or(&reqs[0]);
        let c = self
            .index
            .candidates(name, &def.spec, &def.from_dir)
            .map_err(|e| format!("{} (required by {})", e, def.by))?;
        self.cache.insert(name.to_string(), c.clone());
        Ok(c)
    }

    fn solve(&mut self, st: State) -> Result<State, String> {
        let Some(name) = st.reqs.keys().find(|n| !st.chosen.contains_key(*n)).cloned() else { return Ok(st) };
        let reqs = st.reqs[&name].clone();
        let all = self.candidates_for(&name, &reqs)?;
        let mut cands: Vec<Candidate> = all
            .iter()
            .filter(|c| reqs.iter().all(|r| r.spec.req.matches(&c.version)))
            .cloned()
            .collect();
        if cands.is_empty() {
            return Err(conflict(&name, &reqs, &all, ""));
        }
        let preferred = self.prefer.get(&name).cloned();
        cands.sort_by(|a, b| {
            let pa = preferred.as_ref() == Some(&a.version);
            let pb = preferred.as_ref() == Some(&b.version);
            pb.cmp(&pa).then(b.version.cmp(&a.version))
        });
        let mut first_err: Option<String> = None;
        for cand in cands {
            match self.try_candidate(&st, &name, cand) {
                Ok(done) => return Ok(done),
                Err(e) => {
                    first_err.get_or_insert(e);
                }
            }
        }
        Err(first_err.unwrap_or_else(|| conflict(&name, &reqs, &all, "")))
    }

    fn try_candidate(&mut self, st: &State, name: &str, cand: Candidate) -> Result<State, String> {
        let manifest = self.index.manifest(name, &cand)?;
        let dir = self.index.package_dir(name, &cand);
        let who = format!("{} {}", name, cand.version);
        let mut next = st.clone();
        next.chosen.insert(name.to_string(), cand.clone());
        let mut dep_ids = Vec::new();
        for d in &manifest.dependencies {
            if matches!(d.source, DepSource::Path(_)) && dir.is_none() {
                return Err(format!("{} depends on `{}` by path, which a published package cannot do", who, d.name));
            }
            add_req(&mut next, Req { by: who.clone(), spec: d.clone(), from_dir: dir.clone().unwrap_or_default() });
            if let Some(chosen) = next.chosen.get(&d.name) {
                if !d.req.matches(&chosen.version) {
                    let reqs = next.reqs[&d.name].clone();
                    let avail = self.cache.get(&d.name).cloned().unwrap_or_default();
                    return Err(conflict(&d.name, &reqs, &avail, &format!("{} was already selected", chosen.version)));
                }
            }
            dep_ids.push(d.name.clone());
        }
        next.deps.insert(name.to_string(), dep_ids);
        let mut done = self.solve(next)?;
        // dependency ids become "name version" once everything is chosen
        let ids = done.deps.get(name).cloned().unwrap_or_default();
        let fixed: Vec<String> = ids.iter().map(|n| format!("{} {}", n, done.chosen[n].version)).collect();
        done.deps.insert(name.to_string(), fixed);
        Ok(done)
    }
}

/// Lock-file text for a source.
pub fn source_to_lock(s: &Source, root_dir: &Path) -> String {
    match s {
        Source::Registry(u) => format!("registry+{}", u),
        Source::Git { url, commit } => format!("git+{}#{}", url, commit),
        Source::Path(p) => {
            let rel = relative_to(p, root_dir).unwrap_or_else(|| p.clone());
            format!("path+{}", rel.to_string_lossy().replace('\\', "/"))
        }
    }
}

pub fn source_from_lock(s: &str, root_dir: &Path) -> Option<Source> {
    if let Some(u) = s.strip_prefix("registry+") {
        Some(Source::Registry(u.to_string()))
    } else if let Some(r) = s.strip_prefix("git+") {
        let (url, commit) = r.rsplit_once('#')?;
        Some(Source::Git { url: url.to_string(), commit: commit.to_string() })
    } else {
        s.strip_prefix("path+").map(|p| Source::Path(crate::pkg_files::normalize(&root_dir.join(p))))
    }
}

fn relative_to(p: &Path, base: &Path) -> Option<PathBuf> {
    let p = crate::pkg_files::normalize(p);
    let base = crate::pkg_files::normalize(base);
    let pc: Vec<_> = p.components().collect();
    let bc: Vec<_> = base.components().collect();
    let common = pc.iter().zip(bc.iter()).take_while(|(a, b)| a == b).count();
    if common == 0 {
        return None; // different drives / roots
    }
    let mut out = PathBuf::new();
    for _ in common..bc.len() {
        out.push("..");
    }
    for c in &pc[common..] {
        out.push(c.as_os_str());
    }
    if out.as_os_str().is_empty() {
        out.push(".");
    }
    Some(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::package_manager::VersionReq;

    #[derive(Default)]
    struct Mem {
        pkgs: HashMap<String, Vec<(Version, Vec<(String, String)>)>>,
    }

    impl Mem {
        fn add(&mut self, name: &str, ver: &str, deps: &[(&str, &str)]) {
            self.pkgs.entry(name.to_string()).or_default().push((
                Version::parse(ver).unwrap(),
                deps.iter().map(|(n, r)| (n.to_string(), r.to_string())).collect(),
            ));
        }
    }

    impl PackageIndex for Mem {
        fn candidates(&mut self, name: &str, _s: &DepSpec, _d: &Path) -> Result<Vec<Candidate>, String> {
            let v = self.pkgs.get(name).ok_or_else(|| format!("package `{}` was not found in the registry", name))?;
            Ok(v.iter().map(|(v, _)| Candidate { version: v.clone(), source: Source::Registry("mem".into()) }).collect())
        }
        fn manifest(&mut self, name: &str, c: &Candidate) -> Result<Manifest, String> {
            let (_, deps) = self.pkgs[name].iter().find(|(v, _)| *v == c.version).unwrap();
            Ok(Manifest {
                name: name.to_string(),
                version: c.version.clone(),
                description: String::new(),
                entry: None,
                registry: None,
                dependencies: deps
                    .iter()
                    .map(|(n, r)| DepSpec { name: n.clone(), req: VersionReq::parse(r).unwrap(), source: DepSource::Registry(None) })
                    .collect(),
                dev_dependencies: vec![],
            })
        }
        fn package_dir(&mut self, _n: &str, _c: &Candidate) -> Option<PathBuf> {
            None
        }
    }

    fn root(deps: &[(&str, &str)]) -> Manifest {
        Manifest {
            name: "app".into(),
            version: Version::new(0, 1, 0),
            description: String::new(),
            entry: None,
            registry: None,
            dependencies: deps.iter().map(|(n, r)| DepSpec { name: n.to_string(), req: VersionReq::parse(r).unwrap(), source: DepSource::Registry(None) }).collect(),
            dev_dependencies: vec![],
        }
    }

    fn go(m: &mut Mem, deps: &[(&str, &str)]) -> Result<Vec<Resolved>, String> {
        resolve(&root(deps), Path::new("."), false, m, &HashMap::new())
    }

    #[test]
    fn picks_newest_matching_and_transitive() {
        let mut m = Mem::default();
        m.add("a", "1.0.0", &[]);
        m.add("a", "1.4.0", &[("b", "^2.0")]);
        m.add("a", "2.0.0", &[]);
        m.add("b", "2.1.0", &[]);
        let r = go(&mut m, &[("a", "^1.0")]).unwrap();
        let a = r.iter().find(|p| p.name == "a").unwrap();
        assert_eq!(a.version, Version::new(1, 4, 0));
        assert_eq!(a.deps, vec!["b 2.1.0".to_string()]);
        assert_eq!(r.len(), 2);
    }

    #[test]
    fn backtracks_to_an_older_version_when_the_newest_conflicts() {
        let mut m = Mem::default();
        m.add("a", "1.0.0", &[("c", "^1.0")]);
        m.add("a", "1.1.0", &[("c", "^2.0")]);
        m.add("c", "1.0.0", &[]);
        m.add("c", "2.0.0", &[]);
        let r = go(&mut m, &[("a", "^1.0"), ("c", "^1.0")]).unwrap();
        assert_eq!(r.iter().find(|p| p.name == "a").unwrap().version, Version::new(1, 0, 0));
    }

    #[test]
    fn conflicts_name_the_packages_and_requirements() {
        let mut m = Mem::default();
        m.add("a", "1.0.0", &[("c", "^2.0")]);
        m.add("c", "1.0.0", &[]);
        m.add("c", "2.0.0", &[]);
        let e = go(&mut m, &[("a", "^1.0"), ("c", "^1.0")]).unwrap_err();
        assert!(e.contains("`c`") && e.contains("app requires c ^1.0") && e.contains("a 1.0.0 requires c ^2.0"), "{e}");
        assert!(e.contains("available versions: 1.0.0, 2.0.0"), "{e}");
    }

    #[test]
    fn unsatisfiable_and_missing_packages() {
        let mut m = Mem::default();
        m.add("a", "1.0.0", &[]);
        let e = go(&mut m, &[("a", "^3.0")]).unwrap_err();
        assert!(e.contains("app requires a ^3.0") && e.contains("1.0.0"), "{e}");
        let e = go(&mut m, &[("zzz", "*")]).unwrap_err();
        assert!(e.contains("zzz") && e.contains("required by app"), "{e}");
    }

    #[test]
    fn locked_versions_are_preferred() {
        let mut m = Mem::default();
        m.add("a", "1.0.0", &[]);
        m.add("a", "1.2.0", &[]);
        let mut prefer = HashMap::new();
        prefer.insert("a".to_string(), Version::new(1, 0, 0));
        let r = resolve(&root(&[("a", "^1")]), Path::new("."), false, &mut m, &prefer).unwrap();
        assert_eq!(r[0].version, Version::new(1, 0, 0));
    }

    #[test]
    fn lock_source_round_trip() {
        let root = Path::new("/w/app");
        let p = Source::Path(PathBuf::from("/w/libB"));
        let s = source_to_lock(&p, root);
        assert_eq!(s, "path+../libB");
        assert_eq!(source_from_lock(&s, root), Some(Source::Path(PathBuf::from("/w/libB"))));
        let g = Source::Git { url: "https://x/y.git".into(), commit: "abc".into() };
        assert_eq!(source_from_lock(&source_to_lock(&g, root), root), Some(g));
    }
}
