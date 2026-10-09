//! Fixed official compiler API, closed source host, static declaration scope.
use super::runner::{self, Budget, Failure, IsolatedProject};
use crate::{
    domain::model::*,
    integration::projection::{bytes, digest},
};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{
    collections::{BTreeMap, BTreeSet},
    ffi::OsString,
    fs,
    io::Read,
    path::{Path, PathBuf},
    time::{Duration, Instant},
};
const HELPER: &str = include_str!("typescript/capture.cjs");
const PIN: &str = include_str!("../../fixtures/languages/typescript/tool-profile.json");
#[derive(Debug)]
pub enum TypeScriptFailure {
    Input(String),
    Tool(Failure),
    Protocol(String),
    UnsupportedTool,
}
impl From<String> for TypeScriptFailure {
    fn from(s: String) -> Self {
        Self::Input(s)
    }
}
impl From<&str> for TypeScriptFailure {
    fn from(s: &str) -> Self {
        Self::Input(s.into())
    }
}
impl std::fmt::Display for TypeScriptFailure {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{self:?}")
    }
}
impl std::error::Error for TypeScriptFailure {}
fn portable(p: &str) -> bool {
    !p.is_empty()
        && p.len() <= 256
        && p.split('/').all(|s| {
            !s.is_empty()
                && s != "."
                && s != ".."
                && s.bytes()
                    .all(|b| b.is_ascii_alphanumeric() || b"_$.-".contains(&b))
        })
}
#[derive(Serialize)]
pub struct TypeScriptProfile {
    modules: BTreeMap<String, String>,
    paths: BTreeMap<String, Vec<String>>,
    digest: String,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Config {
    modules: BTreeMap<String, String>,
    paths: BTreeMap<String, Vec<String>>,
}
impl TypeScriptProfile {
    pub fn from_json(raw: &[u8]) -> Result<Self, String> {
        if raw.len() > 32768 {
            return Err("configuration byte budget".into());
        }
        let c: Config = serde_json::from_slice(raw).map_err(|e| e.to_string())?;
        Self::freeze(c.modules, c.paths)
    }
    pub fn freeze(
        modules: BTreeMap<String, String>,
        paths: BTreeMap<String, Vec<String>>,
    ) -> Result<Self, String> {
        if modules.is_empty()
            || modules.len() > 64
            || paths.len() > 64
            || modules.iter().any(|(f, m)| {
                !portable(f) || !f.ends_with(".ts") || m.trim().is_empty() || m.len() > 256
            })
        {
            return Err("invalid required source scope".into());
        }
        for (name, targets) in &paths {
            let alias = name.strip_prefix('@').unwrap_or(name);
            if !portable(&alias.replace('*', "wildcard"))
                || name.matches('*').count() > 1
                || targets.is_empty()
                || targets.len() > 8
                || targets
                    .iter()
                    .any(|p| !portable(&p.replace('*', "wildcard")) || p.matches('*').count() > 1)
            {
                return Err("invalid protected paths mapping".into());
            }
        }
        let hash = digest(&bytes(&(
            "typescript5.9.3-static/v1",
            &modules,
            &paths,
            "ES2022/ESNext/Bundler/strict/noEmit/projectReferences=Reject",
        ))?);
        Ok(Self {
            modules,
            paths,
            digest: hash,
        })
    }
    pub fn digest(&self) -> &str {
        &self.digest
    }
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Installation {
    version: String,
    node_version: String,
    node_sha256: String,
    files: BTreeMap<String, String>,
}
pub struct TypeScriptToolchain {
    node: PathBuf,
    private: tempfile::TempDir,
    identity: String,
}
fn budget() -> Budget {
    Budget {
        output_bytes: 1024 * 1024,
        file_bytes: 512 * 1024,
        total_bytes: 4 * 1024 * 1024,
        files: 256,
        depth: 32,
        ..Budget::default()
    }
}
fn env(home: &Path) -> BTreeMap<OsString, OsString> {
    [
        ("HOME".into(), home.into()),
        ("LANG".into(), "C.UTF-8".into()),
        ("LC_ALL".into(), "C.UTF-8".into()),
    ]
    .into()
}
fn tool_hash(p: &Path) -> Result<String, TypeScriptFailure> {
    let start = Instant::now();
    let mut f = fs::File::open(p).map_err(|e| e.to_string())?;
    if !f.metadata().map_err(|e| e.to_string())?.is_file() {
        return Err("tool must be regular".into());
    }
    let mut n = 0;
    let mut hash = Sha256::new();
    loop {
        if start.elapsed() > Duration::from_secs(20) {
            return Err(TypeScriptFailure::Tool(Failure::Timeout));
        }
        let mut b = [0; 65536];
        let read = f.read(&mut b).map_err(|e| e.to_string())?;
        if read == 0 {
            break;
        }
        n += read;
        if n > 192 * 1024 * 1024 {
            return Err("tool byte budget".into());
        }
        hash.update(&b[..read]);
    }
    Ok(format!("{:x}", hash.finalize()))
}
impl TypeScriptToolchain {
    pub fn freeze(node: &Path, compiler: &Path) -> Result<Self, TypeScriptFailure> {
        let pin: Installation = serde_json::from_str(PIN).map_err(|e| e.to_string())?;
        let node = node.canonicalize().map_err(|e| e.to_string())?;
        if tool_hash(&node)? != pin.node_sha256 {
            return Err(TypeScriptFailure::UnsupportedTool);
        }
        let private = tempfile::tempdir().map_err(|e| e.to_string())?;
        fs::create_dir(private.path().join("lib")).map_err(|e| e.to_string())?;
        let mut libs = Vec::new();
        let mut total = 0;
        for (file, hash) in &pin.files {
            let mut f = fs::File::open(compiler.join(file)).map_err(|e| e.to_string())?;
            let mut raw = Vec::new();
            f.by_ref()
                .take(16 * 1024 * 1024 + 1)
                .read_to_end(&mut raw)
                .map_err(|e| e.to_string())?;
            total += raw.len();
            if raw.len() > 16 * 1024 * 1024
                || total > 32 * 1024 * 1024
                || digest(&raw) != format!("sha256:{hash}")
            {
                return Err(TypeScriptFailure::UnsupportedTool);
            }
            fs::write(private.path().join(file), raw).map_err(|e| e.to_string())?;
            if file.ends_with(".d.ts") {
                libs.push(
                    file.strip_prefix("lib/")
                        .ok_or("invalid library pin")?
                        .to_string(),
                );
            }
        }
        fs::write(private.path().join("libs.json"), bytes(&libs)?).map_err(|e| e.to_string())?;
        fs::write(private.path().join("capture.cjs"), HELPER).map_err(|e| e.to_string())?;
        let version = runner::run(
            &node,
            &["--version".into()],
            private.path(),
            &env(private.path()),
            &budget(),
        )
        .map_err(TypeScriptFailure::Tool)?;
        if version != format!("{}\n", pin.node_version).as_bytes() || pin.version != "5.9.3" {
            return Err(TypeScriptFailure::UnsupportedTool);
        }
        Ok(Self {
            node,
            private,
            identity: digest(&bytes(&(PIN, HELPER))?),
        })
    }
    pub fn provider_id(&self, profile: &TypeScriptProfile) -> String {
        format!(
            "archguard.typescript/v1:{}",
            digest(&bytes(&(&self.identity, profile.digest())).expect("strings serialize"))
        )
    }
    pub fn analyze(
        &self,
        root: &Path,
        profile: &TypeScriptProfile,
    ) -> Result<TypeScriptAnalysis, TypeScriptFailure> {
        let isolated = IsolatedProject::copy_artifacts(root, &[root.to_path_buf()], &budget())
            .map_err(TypeScriptFailure::Tool)?;
        let mut stack = vec![isolated.root().to_path_buf()];
        while let Some(dir) = stack.pop() {
            for e in fs::read_dir(dir).map_err(|e| e.to_string())? {
                let e = e.map_err(|e| e.to_string())?;
                if e.file_type().map_err(|e| e.to_string())?.is_dir() {
                    stack.push(e.path());
                } else if e.file_name().to_string_lossy().starts_with("tsconfig")
                    && e.file_name().to_string_lossy().ends_with(".json")
                {
                    return Err("candidate tsconfig/project references unsupported; protected controller options required".into());
                }
            }
        }
        let mut files = BTreeMap::new();
        for name in profile.modules.keys() {
            match fs::read_to_string(isolated.root().join(name)) {
                Ok(s) => {
                    files.insert(name.clone(), s);
                }
                Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
                Err(e) => return Err(e.to_string().into()),
            }
        }
        let source_digest = digest(&bytes(&(
            "archguard.typescript-input/v1",
            self.provider_id(profile),
            &files,
            &profile.modules,
        ))?);
        let input = bytes(
            &serde_json::json!({"files":files,"modules":profile.modules,"paths":profile.paths}),
        )?;
        if input.len() > 8 * 1024 * 1024 {
            return Err("encoded source budget".into());
        }
        let request = tempfile::tempdir().map_err(|e| e.to_string())?;
        let input_path = request.path().join("input.json");
        fs::write(&input_path, input).map_err(|e| e.to_string())?;
        let capture = runner::run(
            &self.node,
            &[
                "--max-old-space-size=256".into(),
                self.private.path().join("capture.cjs").into(),
                self.private.path().into(),
                input_path.into(),
            ],
            isolated.root(),
            &env(request.path()),
            &budget(),
        )
        .map_err(TypeScriptFailure::Tool)?;
        decode(capture, profile, &self.provider_id(profile), source_digest)
    }
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Capture {
    version: String,
    complete: bool,
    files: Vec<FileRecord>,
    types: Vec<TypeRecord>,
    methods: Vec<MethodRecord>,
    edges: Vec<EdgeRecord>,
    unknowns: Vec<UnknownRecord>,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct FileRecord {
    file: String,
}
#[derive(Deserialize)]
struct Position {
    file: String,
    line: u32,
    column: u32,
    #[serde(rename = "endLine")]
    end_line: u32,
    #[serde(rename = "endColumn")]
    end_column: u32,
}
impl Position {
    fn span(&self) -> Result<SourceSpan, TypeScriptFailure> {
        SourceSpan::new(
            &self.file,
            self.line,
            self.column,
            self.end_line,
            self.end_column,
        )
        .map_err(Into::into)
    }
}
#[derive(Deserialize)]
struct TypeRecord {
    #[serde(flatten)]
    pos: Position,
    name: String,
}
#[derive(Deserialize)]
struct MethodRecord {
    #[serde(flatten)]
    pos: Position,
    owner: String,
    name: String,
    signature: String,
}
#[derive(Deserialize)]
struct EdgeRecord {
    #[serde(flatten)]
    pos: Position,
    from: String,
    to: String,
}
#[derive(Deserialize)]
struct UnknownRecord {
    #[serde(flatten)]
    pos: Position,
    reason: String,
}
pub struct TypeScriptAnalysis {
    observation: LanguageObservation,
    capture: Vec<u8>,
    source_digest: String,
}
impl TypeScriptAnalysis {
    pub fn observation(&self) -> &LanguageObservation {
        &self.observation
    }
    pub fn capture(&self) -> &[u8] {
        &self.capture
    }
    pub fn source_digest(&self) -> &str {
        &self.source_digest
    }
}
fn decode(
    capture: Vec<u8>,
    profile: &TypeScriptProfile,
    provider: &str,
    source_digest: String,
) -> Result<TypeScriptAnalysis, TypeScriptFailure> {
    if capture.len() > 1024 * 1024 {
        return Err("capture byte budget".into());
    }
    let c: Capture =
        serde_json::from_slice(&capture).map_err(|e| TypeScriptFailure::Protocol(e.to_string()))?;
    if c.version != "archguard.typescript-capture/v1"
        || c.files.len() + c.types.len() + c.methods.len() + c.edges.len() + c.unknowns.len() > 4096
    {
        return Err("capture version/record budget".into());
    }
    let mut observation = LanguageObservation::new(Language::TypeScript, provider)?;
    let mut modules = BTreeMap::new();
    let available = c
        .files
        .iter()
        .map(|f| f.file.clone())
        .collect::<BTreeSet<_>>();
    if available.len() != c.files.len()
        || available.iter().any(|f| !profile.modules.contains_key(f))
    {
        return Err("foreign/duplicate file".into());
    }
    // Required missing modules are explicitly registered as scope placeholders, with
    // an Unknown missing-source record. This never asserts that a type was parsed.
    for (file, module) in &profile.modules {
        let id = SymbolId::new(
            Language::TypeScript,
            module,
            "",
            SymbolKind::Module,
            file,
            "",
        )?;
        observation.add_symbol(Symbol {
            id: id.clone(),
            source: SourceSpan::new(file, 1, 1, 1, 2)?,
        })?;
        modules.insert(file.clone(), id);
    }
    let mut ids = BTreeSet::new();
    for t in c.types {
        let m = profile.modules.get(&t.pos.file).ok_or("foreign type")?;
        if !available.contains(&t.pos.file) || t.name.len() > 4096 {
            return Err("type identity".into());
        }
        let id = SymbolId::new(
            Language::TypeScript,
            m,
            &t.pos.file,
            SymbolKind::Type,
            &t.name,
            "",
        )?;
        if ids.insert(id.clone()) {
            observation.add_symbol(Symbol {
                id,
                source: t.pos.span()?,
            })?;
        }
    }
    for t in c.methods {
        let m = profile.modules.get(&t.pos.file).ok_or("foreign method")?;
        if !available.contains(&t.pos.file)
            || t.signature.len() > 4096
            || t.owner.len() > 4096
            || t.name.len() > 4096
        {
            return Err("method identity budget".into());
        }
        let id = SymbolId::new(
            Language::TypeScript,
            m,
            &format!("{}:{}", t.pos.file, t.owner),
            SymbolKind::Method,
            &t.name,
            &t.signature,
        )?;
        if ids.insert(id.clone()) {
            observation.add_symbol(Symbol {
                id,
                source: t.pos.span()?,
            })?;
        }
    }
    for e in c.edges {
        if e.pos.file != e.from || !available.contains(&e.from) || !available.contains(&e.to) {
            return Err("edge source/target unavailable".into());
        }
        observation.confirm_relation(
            Relation::DependsOn,
            modules.get(&e.from).ok_or("foreign from")?,
            modules.get(&e.to).ok_or("foreign to")?,
            e.pos.span()?,
        )?;
    }
    for u in c.unknowns {
        observation.unknown_relation(
            Relation::DependsOn,
            modules.get(&u.pos.file).ok_or("foreign unknown")?,
            u.pos.span()?,
            &u.reason,
        )?;
    }
    if c.complete {
        if available.len() != profile.modules.len() {
            return Err("incomplete required inventory".into());
        }
        observation.mark_scope_complete(Relation::DependsOn);
    }
    Ok(TypeScriptAnalysis {
        observation,
        capture,
        source_digest,
    })
}
