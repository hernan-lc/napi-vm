//! The completion engine: a pure function of `(source, offset, context)`.
//!
//! It combines four sources, in priority order:
//!   1. the program's own scope (declarations, params, imports),
//!   2. host-exposed functions and registered modules (from the context),
//!   3. the built-in global catalog,
//!   4. language keywords.
//!
//! Member completion (`recv.pref|`) resolves the receiver statically: known
//! globals use the catalog, `import * as ns` resolves to a module's exports,
//! and variables with literal initializers reveal their keys/element type.
//! Because it works on the error-tolerant parse, it behaves while code is still
//! being typed and does not yet run.

use crate::lexer::Lexer;
use crate::parser::{Parser, Statement};

use super::catalog::{self, ProtoKind};
use super::scope::{self, InitShape, Scope};
use super::{AnalysisContext, Completion, CompletionKind};

/// Compute completion candidates at a byte offset in the source.
pub fn complete(source: &str, offset: usize, ctx: &AnalysisContext) -> Vec<Completion> {
    // Snap the offset to a char boundary to avoid slicing mid-codepoint.
    let offset = snap_boundary(source, offset.min(source.len()));
    let before = &source[..offset];

    let stmts = parse_lenient(source);
    let scope = scope::collect(&stmts);

    match analyze_trigger(before) {
        Trigger::Member { receiver, prefix } => complete_member(&receiver, &prefix, &scope, ctx),
        Trigger::Ident { prefix } => complete_ident(&prefix, before, &scope, ctx),
    }
}

/// Parse without ever failing: the parser skips what it cannot understand, so
/// this always yields the best-effort AST for the currently-typed source.
fn parse_lenient(source: &str) -> Vec<Statement> {
    let toks = Lexer::new(source).tokenize_with_spans();
    let mut parser = Parser::new_with_spans(toks);
    parser.parse()
}

/// Move `offset` back to the nearest UTF-8 char boundary.
fn snap_boundary(s: &str, mut offset: usize) -> usize {
    while offset > 0 && !s.is_char_boundary(offset) {
        offset -= 1;
    }
    offset
}

// ---- trigger detection ---------------------------------------------------

enum Trigger {
    Member { receiver: String, prefix: String },
    Ident { prefix: String },
}

/// Classify the text before the caret as a member or identifier completion.
fn analyze_trigger(before: &str) -> Trigger {
    if let Some((recv, prefix)) = match_member(before) {
        Trigger::Member {
            receiver: recv,
            prefix,
        }
    } else {
        let prefix = before
            .rsplit_once(|c: char| !(c.is_ascii_alphanumeric() || c == '_' || c == '$'))
            .map(|(_, rest)| rest.to_string())
            .unwrap_or_default();
        Trigger::Ident { prefix }
    }
}

/// Match `receiver.prefix` at the end of `before`, covering dotted chains,
/// string-literal receivers, simple array-literal receivers, and
/// `@playground/<module>.` namespace receivers.
fn match_member(before: &str) -> Option<(String, String)> {
    // @playground/<module>.<prefix> namespace receiver first,
    // before generic dotted chains.
    if let Some((recv, prefix)) = playground_member(before) {
        return Some((recv, prefix));
    }
    // Dotted identifier chain: Math.fl / user.addr.va
    if let Some((recv, prefix)) = rsplit_member(before, |c| {
        c.is_ascii_alphanumeric() || c == '_' || c == '$' || c == '.'
    }) {
        if recv.contains('.') || is_ident_start(recv.chars().next()) {
            return Some((recv, prefix));
        }
    }
    // String literal receiver: "abc".to / 'x'.
    if let Some((recv, prefix)) = literal_receiver(before, '"') {
        return Some((recv, prefix));
    }
    if let Some((recv, prefix)) = literal_receiver(before, '\'') {
        return Some((recv, prefix));
    }
    // Array literal receiver: [1, 2].ma
    if let Some(prefix) = trailing_ident(before) {
        let rest = &before[..before.len() - prefix.len()];
        if rest.ends_with("].") {
            if let Some(open) = rest[..rest.len() - 1].rfind('[') {
                let recv = rest[open..rest.len() - 1].to_string(); // includes the `]`
                return Some((recv, prefix));
            }
        }
    }
    None
}

fn is_ident_start(c: Option<char>) -> bool {
    matches!(c, Some(c) if c.is_ascii_alphabetic() || c == '_' || c == '$')
}

/// Split a trailing `receiver.prefix` where receiver chars satisfy `recv_ok`.
fn rsplit_member(before: &str, recv_ok: impl Fn(char) -> bool) -> Option<(String, String)> {
    let prefix = trailing_ident(before)?;
    let rest = &before[..before.len() - prefix.len()];
    let rest = rest.strip_suffix('.')?;
    let start = rest
        .char_indices()
        .rev()
        .take_while(|&(_, c)| recv_ok(c))
        .last()
        .map(|(i, _)| i)
        .unwrap_or(rest.len());
    let recv = rest[start..].to_string();
    if recv.is_empty() {
        None
    } else {
        Some((recv, prefix))
    }
}

/// The trailing identifier (possibly empty) at the end of `before`.
fn trailing_ident(before: &str) -> Option<String> {
    let rest = before
        .rsplit_once(|c: char| !(c.is_ascii_alphanumeric() || c == '_' || c == '$'))
        .map(|(_, r)| r)
        .unwrap_or(before);
    Some(rest.to_string())
}

/// Match a quoted-string receiver ending just before `.<prefix>`.
fn literal_receiver(before: &str, quote: char) -> Option<(String, String)> {
    let prefix = trailing_ident(before)?;
    let rest = &before[..before.len() - prefix.len()];
    let rest = rest.strip_suffix('.')?;
    if !rest.ends_with(quote) {
        return None;
    }
    // Find the matching opening quote.
    let body = &rest[..rest.len() - 1];
    let open = body.rfind(quote)?;
    let recv = rest[open..].to_string(); // e.g. `"abc"`
    Some((recv, prefix))
}

/// Match a trailing `@playground/<module>.<prefix>` pattern.
fn playground_member(before: &str) -> Option<(String, String)> {
    let dot_pos = before.rfind('.')?;
    let recv = &before[..dot_pos];
    let prefix = &before[dot_pos + 1..];
    if recv.starts_with("@playground/") && !recv["@playground/".len()..].is_empty() {
        Some((recv.to_string(), prefix.to_string()))
    } else {
        None
    }
}

// ---- identifier completion -----------------------------------------------

fn complete_ident(prefix: &str, before: &str, scope: &Scope, ctx: &AnalysisContext) -> Vec<Completion> {
    let mut out: Vec<Completion> = Vec::new();
    let mut seen = std::collections::HashSet::new();
    let mut add = |label: &str, kind: CompletionKind, detail: Option<&str>| {
        if label.starts_with(prefix) && seen.insert(label.to_string()) {
            out.push(Completion {
                label: label.to_string(),
                kind,
                detail: detail.map(|d| d.to_string()),
            });
        }
    };

    // 1. User declarations (highest priority).
    let mut user: Vec<_> = scope.decls.iter().collect();
    user.sort_by(|a, b| a.name.cmp(&b.name));
    for d in user {
        let detail = match d.kind {
            CompletionKind::Function => Some("function"),
            CompletionKind::Class => Some("class"),
            CompletionKind::Module => Some("namespace import"),
            _ => None,
        };
        add(&d.name, d.kind, detail);
    }

    // 2. Host-exposed functions.
    let mut exposed = ctx.exposed_functions.clone();
    exposed.sort();
    for name in &exposed {
        add(name, CompletionKind::ExposedFn, Some("exposed function"));
    }

    // 3. Modules: registered in the context, plus any imported in the source.
    let mut modules: Vec<String> = ctx.modules.iter().map(|m| m.name.clone()).collect();
    for m in &scope.modules {
        if !modules.contains(m) {
            modules.push(m.clone());
        }
    }
    modules.sort();
    for name in &modules {
        add(name, CompletionKind::Module, Some("module"));
    }

    // 4. Built-in globals.
    for g in catalog::GLOBALS {
        add(g, CompletionKind::Global, None);
    }

    // 5. Keywords (lowest priority).
    for k in catalog::KEYWORDS {
        add(k, CompletionKind::Keyword, None);
    }

    // 6. @playground/ namespace: offer @playground/<module> when the
    //    user is typing in a @playground/ import specifier.
    if let Some(module_prefix) = before.rsplit_once("@playground/").map(|(_, rest)| rest) {
        let mut playgrounds: Vec<Completion> = Vec::new();
        for ctx_mod in &ctx.modules {
            if ctx_mod.name.starts_with(module_prefix) {
                playgrounds.push(Completion {
                    label: format!("@playground/{}", ctx_mod.name),
                    kind: CompletionKind::Module,
                    detail: Some("@playground module".to_string()),
                });
            }
        }
        playgrounds.sort_by(|a, b| a.label.cmp(&b.label));
        for c in playgrounds {
            if seen.insert(c.label.clone()) {
                out.push(c);
            }
        }
    }

    out
}

// ---- member completion ---------------------------------------------------

fn complete_member(
    receiver: &str,
    prefix: &str,
    scope: &Scope,
    ctx: &AnalysisContext,
) -> Vec<Completion> {
    let mut out: Vec<Completion> = Vec::new();
    let mut seen = std::collections::HashSet::new();
    let mut add = |label: &str, kind: CompletionKind| {
        if is_member_name(label) && label.starts_with(prefix) && seen.insert(label.to_string()) {
            out.push(Completion {
                label: label.to_string(),
                kind,
                detail: None,
            });
        }
    };

    let head = receiver.split('.').next().unwrap_or(receiver);

    // @playground/<module> → the module's exports.
    if head.starts_with("@playground/") {
        let module_name = &head["@playground/".len()..];
        if let Some(info) = ctx.modules.iter().find(|m| m.name == module_name) {
            let mut exports = info.exports.clone();
            exports.sort();
            for e in &exports {
                add(e, CompletionKind::Property);
            }
            return filter_sorted(out);
        }
    }

    // import * as ns  →  the module's exports.
    if let Some(module) = scope.module_for_namespace(head) {
        if let Some(info) = ctx.modules.iter().find(|m| m.name == module) {
            let mut exports = info.exports.clone();
            exports.sort();
            for e in &exports {
                add(e, CompletionKind::Property);
            }
            return filter_sorted(out);
        }
    }

    // Named built-in global (Math, JSON, console, …).
    if let Some(members) = catalog::builtin_members(receiver) {
        for m in members {
            add(m, member_kind(m));
        }
        return filter_sorted(out);
    }

    // Literal receivers.
    if receiver.ends_with(']') {
        for m in catalog::prototype_members(ProtoKind::Array) {
            add(m, CompletionKind::Method);
        }
        return filter_sorted(out);
    }
    if receiver.ends_with('"') || receiver.ends_with('\'') {
        for m in catalog::prototype_members(ProtoKind::String) {
            add(m, CompletionKind::Method);
        }
        return filter_sorted(out);
    }

    // Variable with a known literal shape.
    if let Some(decl) = scope.find(head) {
        match &decl.shape {
            Some(InitShape::Array) => {
                for m in catalog::prototype_members(ProtoKind::Array) {
                    add(m, CompletionKind::Method);
                }
            }
            Some(InitShape::Object(keys)) => {
                let mut keys = keys.clone();
                keys.sort();
                for k in &keys {
                    add(k, CompletionKind::Property);
                }
            }
            None => {}
        }
    }

    filter_sorted(out)
}

fn is_member_name(label: &str) -> bool {
    !label.is_empty()
        && label
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '$')
}

/// ALL_CAPS built-ins (Math.PI, Symbol.iterator aside) are constants.
fn member_kind(label: &str) -> CompletionKind {
    let is_const = label.chars().all(|c| c.is_ascii_uppercase() || c.is_ascii_digit() || c == '_')
        && label.chars().next().map(|c| c.is_ascii_uppercase()).unwrap_or(false);
    if is_const {
        CompletionKind::Property
    } else {
        CompletionKind::Method
    }
}

fn filter_sorted(mut out: Vec<Completion>) -> Vec<Completion> {
    out.sort_by(|a, b| a.label.cmp(&b.label));
    out
}
