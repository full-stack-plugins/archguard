//! Fixed JDK21 classfile observations; candidate classes are read, never loaded.
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
const HELPER: &str = include_str!("java/BytecodeCapture.java");
const PIN: &str = include_str!("../../fixtures/languages/java/jdk-profile.json");
const CAPTURE: &str = "captures/java-bytecode.tsv";
#[derive(Debug)]
pub enum JavaFailure {
    Input(String),
    Tool(Failure),
    Protocol(String),
    UnsupportedTool,
}
impl From<String> for JavaFailure {
    fn from(s: String) -> Self {
        Self::Input(s)
    }
}
impl From<&str> for JavaFailure {
    fn from(s: &str) -> Self {
        Self::Input(s.into())
    }
}
impl std::fmt::Display for JavaFailure {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{self:?}")
    }
}
impl std::error::Error for JavaFailure {}
#[derive(Serialize)]
pub struct JavaProfile {
    types: BTreeMap<String, String>,
    external: BTreeSet<String>,
    digest: String,
}
fn internal_name(name: &str) -> bool {
    !name.is_empty()
        && name.len() <= 256
        && name.split('/').all(|part| {
            !part.is_empty()
                && part
                    .bytes()
                    .all(|b| b.is_ascii_alphanumeric() || b == b'_' || b == b'$')
        })
}
impl JavaProfile {
    pub fn freeze(
        types: BTreeMap<String, String>,
        external: BTreeSet<String>,
    ) -> Result<Self, String> {
        if types.is_empty()
            || types.len() > 64
            || external.len() > 256
            || types.iter().any(|(t, m)| {
                !internal_name(t) || m.trim().is_empty() || m.len() > 256 || external.contains(t)
            })
            || external.iter().any(|t| !internal_name(t))
        {
            return Err("invalid Java required type/module/external scope".into());
        }
        let hash = digest(&bytes(&("java21-classfile/v1", &types, &external))?);
        Ok(Self {
            types,
            external,
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
    implementor: String,
    files: BTreeMap<String, String>,
}
pub struct JavaToolchain {
    home: PathBuf,
    helper: tempfile::TempDir,
    identity: String,
    build_log: Vec<u8>,
}
fn tool_budget() -> Budget {
    Budget {
        output_bytes: 1024 * 1024,
        file_bytes: 512 * 1024,
        total_bytes: 4 * 1024 * 1024,
        files: 256,
        depth: 32,
        ..Budget::default()
    }
}
fn environment(home: &Path) -> BTreeMap<OsString, OsString> {
    [
        ("HOME".into(), home.as_os_str().into()),
        ("LANG".into(), "C.UTF-8".into()),
        ("LC_ALL".into(), "C.UTF-8".into()),
    ]
    .into()
}
fn hash_tool(path: &Path, start: Instant) -> Result<String, JavaFailure> {
    let mut file = fs::File::open(path).map_err(|e| JavaFailure::Input(e.to_string()))?;
    if !file.metadata().map_err(|e| e.to_string())?.is_file()
        || file.metadata().map_err(|e| e.to_string())?.len() > 192 * 1024 * 1024
    {
        return Err("tool file size".into());
    }
    let mut hash = Sha256::new();
    let mut total = 0usize;
    loop {
        if start.elapsed() > Duration::from_secs(20) {
            return Err(JavaFailure::Tool(Failure::Timeout));
        }
        let mut buf = [0; 65536];
        let n = file.read(&mut buf).map_err(|e| e.to_string())?;
        if n == 0 {
            break;
        }
        total += n;
        if total > 192 * 1024 * 1024 {
            return Err("tool file size".into());
        }
        hash.update(&buf[..n]);
    }
    Ok(format!("{:x}", hash.finalize()))
}
impl JavaToolchain {
    pub fn freeze(home: &Path) -> Result<Self, JavaFailure> {
        let installation: Installation = serde_json::from_str(PIN).map_err(|e| e.to_string())?;
        let home = home.canonicalize().map_err(|e| e.to_string())?;
        let start = Instant::now();
        for (path, expected) in &installation.files {
            if hash_tool(&home.join(path), start)? != *expected {
                return Err(JavaFailure::UnsupportedTool);
            }
        }
        let helper = tempfile::Builder::new()
            .prefix("archguard-java-helper-")
            .tempdir()
            .map_err(|e| e.to_string())?;
        let env = environment(helper.path());
        let version = runner::run(
            &home.join("bin/javac"),
            &["-version".into()],
            helper.path(),
            &env,
            &tool_budget(),
        )
        .map_err(JavaFailure::Tool)?;
        if version != format!("javac {}\n", installation.version).as_bytes() {
            return Err(JavaFailure::UnsupportedTool);
        }
        let source = helper.path().join("BytecodeCapture.java");
        fs::write(&source, HELPER).map_err(|e| e.to_string())?;
        let args: Vec<OsString> = vec![
            "-J-Xmx128m".into(),
            "-J-XX:MaxMetaspaceSize=64m".into(),
            "-J-XX:ActiveProcessorCount=2".into(),
            "-J-XX:+UseSerialGC".into(),
            "-proc:none".into(),
            "--add-modules".into(),
            "jdk.jdeps".into(),
            "--add-exports".into(),
            "jdk.jdeps/com.sun.tools.classfile=ALL-UNNAMED".into(),
            "-d".into(),
            helper.path().into(),
            source.into(),
        ];
        let output = runner::run(
            &home.join("bin/javac"),
            &args,
            helper.path(),
            &env,
            &tool_budget(),
        )
        .map_err(JavaFailure::Tool)?;
        let compiled =
            fs::read(helper.path().join("BytecodeCapture.class")).map_err(|e| e.to_string())?;
        let compiled_digest = digest(&compiled);
        let identity = digest(&bytes(&(
            PIN,
            HELPER,
            &compiled_digest,
            "java21-capture/v1",
        ))?);
        let build_log = bytes(
            &serde_json::json!({"javac_version_stdout":String::from_utf8_lossy(&version),"implementor":installation.implementor,"args":args.iter().map(|a|a.to_string_lossy()).collect::<Vec<_>>(),"stdout":String::from_utf8_lossy(&output),"exit":0,"helper_digest":digest(HELPER.as_bytes()),"compiled_helper_digest":compiled_digest,"installation":serde_json::from_str::<serde_json::Value>(PIN).map_err(|e|e.to_string())?}),
        )?;
        Ok(Self {
            home,
            helper,
            identity,
            build_log,
        })
    }
    pub fn build_log(&self) -> &[u8] {
        &self.build_log
    }
    pub fn provider_id(&self, profile: &JavaProfile) -> String {
        format!(
            "archguard.java21/v1alpha1:{}",
            digest(format!("{}:{}", self.identity, profile.digest).as_bytes())
        )
    }
    pub fn analyze(&self, root: &Path, profile: &JavaProfile) -> Result<JavaAnalysis, JavaFailure> {
        let isolated = IsolatedProject::copy_artifacts(root, &[root.to_owned()], &tool_budget())
            .map_err(JavaFailure::Tool)?;
        let mut input_hash = Sha256::new();
        input_hash.update(b"archguard.java.bytecode-input/v1\0");
        input_hash.update(self.provider_id(profile));
        let mut classfile_digests = BTreeMap::new();
        let mut gaps = Vec::new();
        let mut available = BTreeSet::new();
        let mut args: Vec<OsString> = vec![
            "-Xmx128m".into(),
            "-XX:MaxMetaspaceSize=64m".into(),
            "-XX:ActiveProcessorCount=2".into(),
            "-XX:+UseSerialGC".into(),
            "--add-modules".into(),
            "jdk.jdeps".into(),
            "--add-exports".into(),
            "jdk.jdeps/com.sun.tools.classfile=ALL-UNNAMED".into(),
            "-cp".into(),
            self.helper.path().into(),
            "BytecodeCapture".into(),
        ];
        for name in profile.types.keys() {
            input_hash.update((name.len() as u64).to_le_bytes());
            input_hash.update(name);
            let path = isolated.root().join(format!("{name}.class"));
            let raw = match fs::read(&path) {
                Ok(raw) => raw,
                Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
                    input_hash.update([0]);
                    gaps.push(JavaGap {
                        class_name: name.clone(),
                        reason: "required bytecode missing".into(),
                    });
                    continue;
                }
                Err(e) => return Err(e.to_string().into()),
            };
            input_hash.update([1]);
            input_hash.update((raw.len() as u64).to_le_bytes());
            input_hash.update(&raw);
            classfile_digests.insert(name.clone(), digest(&raw));
            if raw.len() < 8 || raw[..4] != [0xca, 0xfe, 0xba, 0xbe] {
                return Err("invalid classfile header".into());
            }
            if raw[4..8] != [0, 0, 0, 65] {
                gaps.push(JavaGap {
                    class_name: name.clone(),
                    reason: "unsupported classfile version; requires 65.0".into(),
                });
                continue;
            }
            available.insert(name.clone());
            args.push(name.into());
            args.push(path.into());
        }
        let capture = runner::run(
            &self.home.join("bin/java"),
            &args,
            isolated.root(),
            &environment(self.helper.path()),
            &tool_budget(),
        )
        .map_err(JavaFailure::Tool)?;
        decode(
            &capture,
            profile,
            &available,
            &self.provider_id(profile),
            gaps,
            format!("sha256:{:x}", input_hash.finalize()),
            classfile_digests,
        )
    }
}
#[derive(Debug, Serialize)]
pub struct JavaGap {
    pub class_name: String,
    pub reason: String,
}
pub struct JavaAnalysis {
    observation: LanguageObservation,
    capture: Vec<u8>,
    gaps: Vec<JavaGap>,
    source_digest: String,
    external_references: BTreeSet<String>,
    classfile_digests: BTreeMap<String, String>,
}
impl JavaAnalysis {
    pub fn observation(&self) -> &LanguageObservation {
        &self.observation
    }
    pub fn capture(&self) -> &[u8] {
        &self.capture
    }
    pub fn gaps(&self) -> &[JavaGap] {
        &self.gaps
    }
    pub fn source_digest(&self) -> &str {
        &self.source_digest
    }
    pub fn classfile_digests(&self) -> &BTreeMap<String, String> {
        &self.classfile_digests
    }
    pub fn external_references(&self) -> &BTreeSet<String> {
        &self.external_references
    }
}
fn unhex(s: &str) -> Result<String, JavaFailure> {
    if s.len() > 4096 || !s.len().is_multiple_of(2) || !s.bytes().all(|b| b.is_ascii_hexdigit()) {
        return Err(JavaFailure::Protocol("record text budget".into()));
    }
    let bytes = (0..s.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(&s[i..i + 2], 16))
        .collect::<Result<Vec<_>, _>>()
        .map_err(|_| JavaFailure::Protocol("record encoding".into()))?;
    String::from_utf8(bytes).map_err(|_| JavaFailure::Protocol("record UTF-8".into()))
}
fn span(line: usize) -> SourceSpan {
    SourceSpan::new(CAPTURE, line as u32, 1, line as u32, 2)
        .expect("bounded native capture position")
}
fn type_id(profile: &JavaProfile, name: &str) -> Result<SymbolId, JavaFailure> {
    SymbolId::new(
        Language::Java,
        profile.types.get(name).ok_or("foreign native class")?,
        "",
        SymbolKind::Type,
        name,
        "",
    )
    .map_err(Into::into)
}
fn method_id(
    profile: &JavaProfile,
    owner: &str,
    name: &str,
    desc: &str,
) -> Result<SymbolId, JavaFailure> {
    SymbolId::new(
        Language::Java,
        profile.types.get(owner).ok_or("foreign native method")?,
        owner,
        SymbolKind::Method,
        name,
        desc,
    )
    .map_err(Into::into)
}
fn decode(
    raw: &[u8],
    profile: &JavaProfile,
    available: &BTreeSet<String>,
    provider: &str,
    gaps: Vec<JavaGap>,
    source_digest: String,
    classfile_digests: BTreeMap<String, String>,
) -> Result<JavaAnalysis, JavaFailure> {
    let text = std::str::from_utf8(raw).map_err(|_| "native capture UTF-8")?;
    if raw.len() > 1024 * 1024
        || !text.starts_with("ARCHGUARD-JAVA\t1\n")
        || !text.ends_with("END\n")
    {
        return Err(JavaFailure::Protocol("native capture framing".into()));
    }
    let mut observation = LanguageObservation::new(Language::Java, provider)?;
    let mut seen = BTreeSet::new();
    let mut modules = BTreeSet::new();
    let mut edges = Vec::new();
    let mut unknowns = Vec::new();
    let mut external_references = BTreeSet::new();
    for (n, line) in text.lines().enumerate() {
        if n > 4096 {
            return Err("native record budget".into());
        }
        let cols: Vec<_> = line.split('\t').collect();
        match cols.as_slice() {
            ["ARCHGUARD-JAVA", "1"] if n == 0 => (),
            ["END"] if n + 1 == text.lines().count() => (),
            ["C", name] => {
                let name = unhex(name)?;
                if !available.contains(&name) || !seen.insert(name.clone()) {
                    return Err("unexpected or duplicate native type".into());
                }
                let id = type_id(profile, &name)?;
                let module = id.module().to_owned();
                observation.add_symbol(Symbol {
                    id,
                    source: span(n + 1),
                })?;
                if modules.insert(module.clone()) {
                    observation.add_symbol(Symbol {
                        id: SymbolId::new(
                            Language::Java,
                            &module,
                            "",
                            SymbolKind::Module,
                            &module,
                            "",
                        )?,
                        source: span(n + 1),
                    })?;
                }
            }
            ["M", owner, name, desc] => {
                let owner = unhex(owner)?;
                if !seen.contains(&owner) {
                    return Err("native method before type".into());
                }
                let id = method_id(profile, &owner, &unhex(name)?, &unhex(desc)?)?;
                observation.add_symbol(Symbol {
                    id,
                    source: span(n + 1),
                })?;
            }
            ["D", owner, target] => {
                let owner = unhex(owner)?;
                let target = unhex(target)?;
                if !seen.contains(&owner) || !internal_name(&target) {
                    return Err("invalid native dependency".into());
                }
                edges.push((owner, target, n + 1));
            }
            ["U", owner, name, desc, relation, reason, pc] => {
                let owner = unhex(owner)?;
                if !seen.contains(&owner) || pc.parse::<i32>().is_err() {
                    return Err("invalid native gap".into());
                }
                let relation = match *relation {
                    "D" => Relation::DependsOn,
                    "C" => Relation::Calls,
                    _ => return Err("invalid relation".into()),
                };
                unknowns.push((
                    owner,
                    unhex(name)?,
                    unhex(desc)?,
                    relation,
                    unhex(reason)?,
                    n + 1,
                ));
            }
            _ => return Err(JavaFailure::Protocol("unrecognized native record".into())),
        }
    }
    if &seen != available {
        return Err("missing native types".into());
    }
    for (owner, target, line) in edges {
        let from = type_id(profile, &owner)?;
        if seen.contains(&target) {
            observation.confirm_relation(
                Relation::DependsOn,
                &from,
                &type_id(profile, &target)?,
                span(line),
            )?;
        } else if profile.types.contains_key(&target) {
            observation.unknown_relation(
                Relation::DependsOn,
                &from,
                span(line),
                "required target bytecode unavailable",
            )?;
        } else if target.starts_with("java/") || profile.external.contains(&target) {
            external_references.insert(target);
        } else {
            observation.unknown_relation(
                Relation::DependsOn,
                &from,
                span(line),
                &format!("undeclared external type: {target}"),
            )?;
        }
    }
    for (owner, name, desc, relation, reason, line) in unknowns {
        let from = if name.is_empty() {
            type_id(profile, &owner)?
        } else {
            method_id(profile, &owner, &name, &desc)?
        };
        observation.unknown_relation(relation, &from, span(line), &reason)?;
    }
    if gaps.is_empty() {
        observation.mark_scope_complete(Relation::DependsOn);
    }
    observation.bounded_input_bytes()?;
    Ok(JavaAnalysis {
        observation,
        capture: raw.to_vec(),
        gaps,
        source_digest,
        external_references,
        classfile_digests,
    })
}

#[cfg(test)]
mod protocol_tests {
    use super::*;
    #[test]
    fn native_text_decoder_rejects_nonascii_without_panicking() {
        assert!(std::panic::catch_unwind(|| unhex("0é0")).unwrap().is_err());
    }
}
