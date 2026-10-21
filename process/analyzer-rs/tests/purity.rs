//! `syn`-based purity test for the deterministic core (CI hardening). The Makefile grep is only a
//! fast pre-filter; this parses every `src/strategy/**/*.rs` with `syn` and fails if any `f32`/`f64`
//! **token** appears. Because it works on tokens, it catches evasions the old `grep -vE '//'`
//! pre-filter missed (a real `f64` on a line that also has a trailing comment), while an `f64`
//! mentioned in a comment or a string literal is **not** a token and is **not** flagged.
//!
//! This test is not gated behind the `harness` feature: it runs under `--no-default-features` too.

use std::path::{Path, PathBuf};

/// Scan a Rust source string for `f32`/`f64` tokens, returning one description per hit.
///
/// `syn::parse_file` rejects invalid Rust (and, like any Rust lexer, drops comments); the source is
/// then tokenized with `proc-macro2` so identifiers and float-literal suffixes are inspected at the
/// token level. A string literal is a single `Literal` token and is skipped.
fn scan_source(src: &str) -> Result<Vec<String>, String> {
    syn::parse_file(src).map_err(|e| format!("failed to parse Rust source: {e}"))?;
    let tokens: proc_macro2::TokenStream = src
        .parse()
        .map_err(|e| format!("failed to tokenize: {e}"))?;
    let mut hits = Vec::new();
    scan_tokens(tokens, &mut hits);
    Ok(hits)
}

fn scan_tokens(tokens: proc_macro2::TokenStream, hits: &mut Vec<String>) {
    use proc_macro2::TokenTree;
    for tt in tokens {
        match tt {
            TokenTree::Ident(id) => {
                let s = id.to_string();
                if s == "f32" || s == "f64" {
                    hits.push(format!("{s} at line {}", id.span().start().line));
                }
            }
            TokenTree::Literal(lit) => {
                let s = lit.to_string();
                // String / char / byte / raw literals are not float types.
                let stringy = s.starts_with('"')
                    || s.starts_with('\'')
                    || s.starts_with("b\"")
                    || s.starts_with("br")
                    || s.starts_with("r\"")
                    || s.starts_with("r#");
                if !stringy && (s.ends_with("f32") || s.ends_with("f64")) {
                    hits.push(format!(
                        "float literal `{s}` at line {}",
                        lit.span().start().line
                    ));
                }
            }
            TokenTree::Group(g) => scan_tokens(g.stream(), hits),
            TokenTree::Punct(_) => {}
        }
    }
}

fn rs_files(dir: &Path, out: &mut Vec<PathBuf>) {
    let entries = std::fs::read_dir(dir).unwrap_or_else(|e| panic!("read_dir {dir:?}: {e}"));
    for entry in entries {
        let p = entry.expect("dir entry").path();
        if p.is_dir() {
            rs_files(&p, out);
        } else if p.extension().and_then(|e| e.to_str()) == Some("rs") {
            out.push(p);
        }
    }
}

#[test]
fn strategy_core_contains_no_float_tokens() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("src/strategy");
    let mut files = Vec::new();
    rs_files(&root, &mut files);
    assert!(
        !files.is_empty(),
        "no strategy source files found under {root:?}"
    );

    let mut violations = Vec::new();
    for file in &files {
        let src = std::fs::read_to_string(file).unwrap_or_else(|e| panic!("read {file:?}: {e}"));
        let hits = scan_source(&src).unwrap_or_else(|e| panic!("{}: {e}", file.display()));
        for hit in hits {
            violations.push(format!("{}: {hit}", file.display()));
        }
    }
    assert!(
        violations.is_empty(),
        "float tokens in strategy core:\n{}",
        violations.join("\n")
    );
}

#[test]
fn scanner_flags_real_float_types_but_not_comments_or_strings() {
    // A comment and a string literal mentioning f64/f32 must NOT be flagged.
    let clean = r#"
// this comment mentions f64 and f32
fn f() {
    let s = "f64";
    let t: u32 = 1;
    let _ = (s, t);
}
"#;
    assert!(
        scan_source(clean).unwrap().is_empty(),
        "comments/strings must not be flagged"
    );

    // A real f64 type token IS flagged, even on a line with a trailing comment.
    let dirty = r#"
fn f() { let x: f64 = 1.0; } // trailing f64 comment
"#;
    assert!(
        !scan_source(dirty).unwrap().is_empty(),
        "a real f64 type must be flagged"
    );

    // A suffixed float literal is flagged too.
    let literal = r#"
fn f() -> u8 { let _x = 1.0f64; 0 }
"#;
    assert!(
        !scan_source(literal).unwrap().is_empty(),
        "an f64 literal suffix must be flagged"
    );
}
