//! Versioned in-memory language facts. Provider signatures must be canonical for
//! their pinned configuration; this model does not infer language semantics.
use std::collections::{BTreeMap, BTreeSet};

pub const MODEL_VERSION: &str = "archguard.language-model/v1alpha1";

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Language {
    Java,
    TypeScript,
    Rust,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum SymbolKind {
    Module,
    Type,
    Method,
}

/// Structured identity avoids delimiter collisions. Source locations are not
/// identities: moving a declaration alone does not rename the symbol.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct SymbolId {
    language: Language,
    module: String,
    owner: String,
    kind: SymbolKind,
    name: String,
    signature: String,
}

impl SymbolId {
    pub fn new(
        language: Language,
        module: &str,
        owner: &str,
        kind: SymbolKind,
        name: &str,
        signature: &str,
    ) -> Result<Self, &'static str> {
        if module.trim().is_empty() || name.trim().is_empty() {
            return Err("symbol requires module and name");
        }
        if kind == SymbolKind::Method && signature.trim().is_empty() {
            return Err("method requires provider canonical signature");
        }
        Ok(Self {
            language,
            module: module.into(),
            owner: owner.into(),
            kind,
            name: name.into(),
            signature: signature.into(),
        })
    }
}

/// One-based line/column positions; end is exclusive. Paths use portable,
/// snapshot-relative slash notation, without dot or empty components.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SourceSpan {
    path: String,
    start: (u32, u32),
    end: (u32, u32),
}

impl SourceSpan {
    pub fn new(
        path: &str,
        line: u32,
        column: u32,
        end_line: u32,
        end_column: u32,
    ) -> Result<Self, &'static str> {
        if path.contains(['\\', ':', '\0'])
            || path.split('/').any(|part| matches!(part, "" | "." | ".."))
            || [line, column, end_line, end_column].contains(&0)
            || (line, column) >= (end_line, end_column)
        {
            return Err("invalid snapshot-relative source span");
        }
        Ok(Self {
            path: path.into(),
            start: (line, column),
            end: (end_line, end_column),
        })
    }
    pub fn path(&self) -> &str {
        &self.path
    }
    pub fn start(&self) -> (u32, u32) {
        self.start
    }
    pub fn end(&self) -> (u32, u32) {
        self.end
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Symbol {
    pub id: SymbolId,
    pub source: SourceSpan,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Relation {
    DependsOn,
    Calls,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConfirmedEdge {
    pub relation: Relation,
    pub from: SymbolId,
    pub to: SymbolId,
    pub source: SourceSpan,
}

/// There is deliberately no target: an unresolved call is not a guessed edge.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UnknownRelation {
    pub relation: Relation,
    pub from: SymbolId,
    pub source: SourceSpan,
    pub reason: String,
}

#[derive(Debug, Clone)]
pub struct LanguageObservation {
    language: Language,
    provider_profile: String,
    symbols: BTreeMap<SymbolId, Symbol>,
    edges: Vec<ConfirmedEdge>,
    unknowns: Vec<UnknownRelation>,
    complete_scopes: BTreeSet<Relation>,
}

impl LanguageObservation {
    pub fn new(language: Language, provider_profile: &str) -> Result<Self, &'static str> {
        if provider_profile.trim().is_empty() {
            return Err("provider profile required");
        }
        Ok(Self {
            language,
            provider_profile: provider_profile.into(),
            symbols: BTreeMap::new(),
            edges: Vec::new(),
            unknowns: Vec::new(),
            complete_scopes: BTreeSet::new(),
        })
    }
    pub fn model_version(&self) -> &'static str {
        MODEL_VERSION
    }
    pub fn provider_profile(&self) -> &str {
        &self.provider_profile
    }
    pub fn symbols(&self) -> impl Iterator<Item = &Symbol> {
        self.symbols.values()
    }
    pub fn edges(&self) -> &[ConfirmedEdge] {
        &self.edges
    }
    pub fn unknowns(&self) -> &[UnknownRelation] {
        &self.unknowns
    }
    pub fn add_symbol(&mut self, symbol: Symbol) -> Result<(), &'static str> {
        if symbol.id.language != self.language || self.symbols.contains_key(&symbol.id) {
            return Err("foreign or duplicate symbol");
        }
        self.symbols.insert(symbol.id.clone(), symbol);
        Ok(())
    }
    pub fn confirm_relation(
        &mut self,
        relation: Relation,
        from: &SymbolId,
        to: &SymbolId,
        source: SourceSpan,
    ) -> Result<(), &'static str> {
        if !self.symbols.contains_key(from) || !self.symbols.contains_key(to) {
            return Err("unregistered edge endpoint");
        }
        self.edges.push(ConfirmedEdge {
            relation,
            from: from.clone(),
            to: to.clone(),
            source,
        });
        Ok(())
    }
    pub fn unknown_relation(
        &mut self,
        relation: Relation,
        from: &SymbolId,
        source: SourceSpan,
        reason: &str,
    ) -> Result<(), &'static str> {
        if !self.symbols.contains_key(from) {
            return Err("unregistered unknown source");
        }
        if reason.trim().is_empty() {
            return Err("unknown relation requires reason");
        }
        self.unknowns.push(UnknownRelation {
            relation,
            from: from.clone(),
            source,
            reason: reason.into(),
        });
        Ok(())
    }
    /// A provider declaration, not authentication or proof of scope inventory.
    /// Known unresolved relations cannot be cleared by this declaration.
    pub fn mark_scope_complete(&mut self, relation: Relation) {
        self.complete_scopes.insert(relation);
    }
    pub fn is_complete(&self, relation: Relation) -> bool {
        self.complete_scopes.contains(&relation)
            && !self.unknowns.iter().any(|gap| gap.relation == relation)
    }
}
