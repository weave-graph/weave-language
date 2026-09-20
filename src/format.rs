//! Syntax-only formatting: token spelling is never regenerated from decoded AST values.
use crate::{Diagnostic, parse, syntax};
const LIMIT: usize = 1_048_576;
#[derive(Clone, Copy)]
struct Piece<'a> {
    raw: &'a str,
    comment: bool,
    inline: bool,
}
fn budget(source: &str) -> Diagnostic {
    Diagnostic::new(
        "E_BUDGET",
        "Formatting exceeds source/layout budget",
        0,
        source.len(),
    )
}
fn pieces(source: &str) -> Result<Vec<Piece<'_>>, Diagnostic> {
    let spans = syntax::token_spans(source)?;
    let mut pieces = Vec::new();
    let mut end = 0;
    for (start, stop) in spans.into_iter().chain([(source.len(), source.len())]) {
        let gap = &source[end..start];
        let mut offset = 0;
        let mut same_line = end != 0;
        while let Some(relative) = gap[offset..].find("//") {
            let begin = offset + relative;
            let finish = gap[begin..].find('\n').map_or(gap.len(), |n| begin + n);
            pieces.push(Piece {
                raw: &gap[begin..finish],
                comment: true,
                inline: same_line && !gap[offset..begin].contains('\n'),
            });
            offset = finish;
            same_line = false;
            if pieces.len() > 100_000 {
                return Err(budget(source));
            }
        }
        if start != stop {
            pieces.push(Piece {
                raw: &source[start..stop],
                comment: false,
                inline: false,
            });
            if pieces.len() > 100_000 {
                return Err(budget(source));
            }
        }
        end = stop;
    }
    Ok(pieces)
}
struct Layout<'a> {
    out: String,
    source: &'a str,
    depth: usize,
    pending: bool,
}
impl Layout<'_> {
    fn push(&mut self, text: &str) -> Result<(), Diagnostic> {
        if self.out.len().saturating_add(text.len()) > LIMIT {
            return Err(budget(self.source));
        }
        self.out.push_str(text);
        Ok(())
    }
    fn newline(&mut self) -> Result<(), Diagnostic> {
        if !self.out.is_empty() && !self.out.ends_with('\n') {
            self.push("\n")?;
        }
        self.pending = false;
        Ok(())
    }
    fn indent(&mut self) -> Result<(), Diagnostic> {
        if self.out.is_empty() || self.out.ends_with('\n') {
            for _ in 0..self.depth {
                self.push("  ")?;
            }
        }
        Ok(())
    }
    fn space(&mut self) -> Result<(), Diagnostic> {
        if !self.out.is_empty() && !self.out.ends_with([' ', '\n']) {
            self.push(" ")?;
        }
        Ok(())
    }
}
/// Format one syntactically valid source unit without linking or executing it.
/// Every token and `//` comment retains its exact spelling, including escapes.
/// Module content pins intentionally change when their source bytes change.
/// Input/output are bounded to 1 MiB and at most 100,000 tokens plus comments.
/// Diagnostics from malformed input retain the original source byte spans.
pub fn format_source(source: &str) -> Result<String, Diagnostic> {
    parse(source)?;
    let pieces = pieces(source)?;
    let mut layout = Layout {
        out: String::new(),
        source,
        depth: 0,
        pending: false,
    };
    let mut previous = "";
    for (i, piece) in pieces.iter().enumerate() {
        if piece.comment {
            if !piece.inline {
                layout.newline()?;
            }
            if piece.inline {
                layout.space()?;
            } else {
                layout.indent()?;
            }
            layout.push(piece.raw)?;
            layout.newline()?;
            continue;
        }
        let current = piece.raw;
        if layout.pending {
            layout.newline()?;
        }
        if current == "}" {
            layout.depth = layout.depth.saturating_sub(1);
            if previous != "{" {
                layout.newline()?;
            }
        }
        layout.indent()?;
        match current {
            "{" => {
                if !matches!(previous, "" | "[" | "(" | "{") {
                    layout.space()?;
                }
                layout.push(current)?;
                layout.depth += 1;
                layout.pending = pieces.get(i + 1).is_none_or(|p| p.raw != "}");
            }
            "}" => {
                layout.push(current)?;
                layout.pending = pieces
                    .get(i + 1)
                    .is_none_or(|p| !matches!(p.raw, ";" | "," | ")" | "]"));
            }
            ";" => {
                layout.push(current)?;
                layout.pending = true;
            }
            "," | ":" | ")" | "]" => {
                layout.push(current)?;
            }
            "(" => {
                if previous.starts_with('"') {
                    layout.space()?;
                }
                layout.push(current)?;
            }
            "[" => {
                if !matches!(previous, "" | "[" | "(") {
                    layout.space()?;
                }
                layout.push(current)?;
            }
            _ => {
                if !matches!(previous, "" | "(" | "[") {
                    layout.space()?;
                }
                layout.push(current)?;
            }
        }
        previous = current;
    }
    layout.newline()?;
    // Empty/whitespace-only input has one canonical final newline too.
    if layout.out.is_empty() {
        layout.push("\n")?;
    }
    Ok(layout.out)
}
