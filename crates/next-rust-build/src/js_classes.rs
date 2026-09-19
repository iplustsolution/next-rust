//! Class names inside scripts, for release builds with short class names.
//!
//! The renderer writes short names into the HTML it produces, but scripts in
//! `client/` and `assets/` add classes of their own. Their string literals
//! are examined so those classes can be shortened too:
//!
//! * A literal (or the static part of a template) made only of known
//!   classes is a class list: it is rewritten in the script.
//! * A literal that is exactly one class is rewritten where it can only be a
//!   class (`classList.add(..)`, `className = ..`, `class:` in an object,
//!   `setAttribute("class", ..)`), or when it can't be a CSS keyword
//!   (`mt-3`, `bg-[var(--x)]`). Anywhere else, `"block"` may be a `display`
//!   value, so the class keeps its name everywhere.
//! * `class="…"` inside HTML in a literal, and `.class` in a selector, are
//!   rewritten.
//! * A literal mixing known classes with other words keeps those classes'
//!   names: the script may use them as hooks.
//!
//! Regular expressions, `dataset` values and strings built character by
//! character are not seen; list such classes in `[tailwind] keep_classes`.

use std::collections::BTreeSet;

/// Where classes occur in a script.
#[derive(Debug, Default)]
pub struct Occurrences {
    /// Byte ranges of class tokens that can be rewritten, with the class.
    pub renamable: Vec<(usize, usize, String)>,
    /// Classes that must keep their names.
    pub fixed: BTreeSet<String>,
}

impl Occurrences {
    /// `source` with the renamable classes replaced by `rename` (classes it
    /// returns `None` for are left alone).
    pub fn apply(&self, source: &str, rename: &dyn Fn(&str) -> Option<String>) -> String {
        let mut edits: Vec<&(usize, usize, String)> = self.renamable.iter().collect();
        edits.sort_by_key(|e| e.0);
        let mut out = String::with_capacity(source.len());
        let mut last = 0;
        for (start, end, class) in edits {
            if *start < last {
                continue;
            }
            if let Some(short) = rename(class) {
                out.push_str(&source[last..*start]);
                out.push_str(&short);
                last = *end;
            }
        }
        out.push_str(&source[last..]);
        out
    }
}

/// Find the classes of `source`, a script. `is_class` says whether a token
/// is a class of the stylesheet.
#[cfg(feature = "minify-js")]
pub fn analyze(source: &str, is_class: &dyn Fn(&str) -> bool) -> Result<Occurrences, String> {
    use oxc_allocator::Allocator;
    use oxc_ast_visit::Visit;
    use oxc_parser::Parser;
    use oxc_span::SourceType;

    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        let allocator = Allocator::default();
        let parsed = [SourceType::mjs(), SourceType::cjs().with_module(false)]
            .into_iter()
            .map(|t| Parser::new(&allocator, source, t).parse())
            .find(|p| p.errors.is_empty())
            .ok_or_else(|| "the script does not parse".to_owned())?;
        let mut finder = Finder { source, is_class, contexts: Default::default(), out: Occurrences::default() };
        finder.visit_program(&parsed.program);
        Ok(finder.out)
    }));
    result.unwrap_or_else(|_| Err("the script analysis failed unexpectedly".into()))
}

#[cfg(not(feature = "minify-js"))]
pub fn analyze(_source: &str, _is_class: &dyn Fn(&str) -> bool) -> Result<Occurrences, String> {
    Err("next-rust-build was compiled without the `minify-js` feature".into())
}

#[cfg(feature = "minify-js")]
struct Finder<'s, 'c> {
    source: &'s str,
    is_class: &'c dyn Fn(&str) -> bool,
    /// Starts of literals that can only hold classes.
    contexts: std::collections::HashSet<u32>,
    out: Occurrences,
}

#[cfg(feature = "minify-js")]
impl Finder<'_, '_> {
    /// Remember every literal reachable from `e` through `+`, `||`, `??`,
    /// `?:` and parentheses as a class context.
    fn mark(&mut self, e: &oxc_ast::ast::Expression<'_>) {
        use oxc_ast::ast::Expression;
        match e {
            Expression::StringLiteral(s) => {
                self.contexts.insert(s.span.start);
            }
            Expression::TemplateLiteral(t) => {
                self.contexts.insert(t.span.start);
            }
            Expression::ParenthesizedExpression(p) => self.mark(&p.expression),
            Expression::BinaryExpression(b) => {
                self.mark(&b.left);
                self.mark(&b.right);
            }
            Expression::LogicalExpression(l) => {
                self.mark(&l.left);
                self.mark(&l.right);
            }
            Expression::ConditionalExpression(c) => {
                self.mark(&c.consequent);
                self.mark(&c.alternate);
            }
            _ => {}
        }
    }

    /// A literal made of `pieces` (one for a string, the static parts of a
    /// template), each at its byte offset.
    fn literal(&mut self, pieces: &[(usize, &str)], in_context: bool) {
        if pieces.iter().any(|(_, raw)| raw.contains('\\')) {
            return;
        }
        // HTML in a string: `class="…"` attributes.
        let mut found_html = false;
        for (base, raw) in pieces {
            let mut from = 0;
            while let Some(at) = raw[from..].find("class=").map(|at| from + at) {
                let start = at + "class=".len();
                from = start;
                let Some(q @ ('"' | '\'')) = raw[start..].chars().next() else { continue };
                let Some(len) = raw[start + 1..].find(q) else { break };
                self.literal(&[(base + start + 1, &raw[start + 1..start + 1 + len])], true);
                found_html = true;
                from = start + 1 + len;
            }
        }
        if found_html {
            return;
        }
        // Complete tokens: a template part next to `${…}` may hold half of one.
        let count = pieces.len();
        let mut tokens: Vec<(usize, &str)> = Vec::new();
        for (i, (base, raw)) in pieces.iter().enumerate() {
            let first = i == 0 || raw.starts_with(|c: char| c.is_ascii_whitespace());
            let last = i + 1 == count || raw.ends_with(|c: char| c.is_ascii_whitespace());
            let mut found: Vec<(usize, &str)> = Vec::new();
            let mut at = 0;
            for token in raw.split_ascii_whitespace() {
                let pos = raw[at..].find(token).map_or(at, |p| at + p);
                found.push((base + pos, token));
                at = pos + token.len();
            }
            let n = found.len();
            tokens.extend(
                found.into_iter().enumerate().filter(|(j, _)| (first || *j > 0) && (last || j + 1 < n)).map(|(_, t)| t),
            );
        }
        if tokens.is_empty() {
            return;
        }
        let known: Vec<(usize, &str)> = tokens.iter().copied().filter(|(_, t)| (self.is_class)(t)).collect();
        if known.is_empty() {
            for (base, raw) in pieces {
                self.selector(*base, raw);
            }
            return;
        }
        let list = known.len() == tokens.len();
        // A lone token is a class list when it can't be a CSS keyword
        // (`mt-3`, `bg-[var(--x)]`, `sm:flex`), unlike `block` or `flex`.
        let unmistakable = |t: &str| t.contains(|c: char| c.is_ascii_digit() || matches!(c, '[' | ':' | '/' | '!'));
        if in_context || (list && (tokens.len() >= 2 || unmistakable(tokens[0].1))) {
            for (pos, class) in known {
                self.out.renamable.push((pos, pos + class.len(), class.to_owned()));
            }
        } else {
            self.out.fixed.extend(known.into_iter().map(|(_, c)| c.to_owned()));
        }
    }

    /// `.class` tokens of a selector.
    fn selector(&mut self, base: usize, raw: &str) {
        let bytes = raw.as_bytes();
        let part = |b: u8| b.is_ascii_alphanumeric() || b == b'-' || b == b'_';
        for (at, _) in raw.match_indices('.') {
            if at > 0 && matches!(bytes[at - 1], b'.' | b'\\') {
                continue;
            }
            let end = at + 1 + bytes[at + 1..].iter().take_while(|b| part(**b)).count();
            let name = &raw[at + 1..end];
            if name.is_empty() || !(self.is_class)(name) {
                continue;
            }
            if at > 0 && part(bytes[at - 1]) {
                // `div.card` is a selector, `home.js` a file name: keep the name.
                self.out.fixed.insert(name.to_owned());
            } else {
                self.out.renamable.push((base + at + 1, base + end, name.to_owned()));
            }
        }
    }
}

#[cfg(feature = "minify-js")]
impl<'a> oxc_ast_visit::Visit<'a> for Finder<'_, '_> {
    fn visit_call_expression(&mut self, it: &oxc_ast::ast::CallExpression<'a>) {
        use oxc_ast::ast::Expression;
        if let Some(member) = it.callee.as_member_expression() {
            let method = member.static_property_name().unwrap_or("");
            let on_class_list =
                member.object().as_member_expression().and_then(|o| o.static_property_name()) == Some("classList");
            if on_class_list && matches!(method, "add" | "remove" | "toggle" | "contains" | "replace") {
                for arg in &it.arguments {
                    if let Some(e) = arg.as_expression() {
                        self.mark(e);
                    }
                }
            } else if method == "setAttribute"
                && let [first, second] = it.arguments.as_slice()
                && matches!(first.as_expression(), Some(Expression::StringLiteral(s)) if s.value == "class")
                && let Some(e) = second.as_expression()
            {
                self.mark(e);
            }
        }
        oxc_ast_visit::walk::walk_call_expression(self, it);
    }

    fn visit_assignment_expression(&mut self, it: &oxc_ast::ast::AssignmentExpression<'a>) {
        use oxc_ast::ast::AssignmentTarget;
        if let AssignmentTarget::StaticMemberExpression(m) = &it.left
            && m.property.name == "className"
        {
            self.mark(&it.right);
        }
        oxc_ast_visit::walk::walk_assignment_expression(self, it);
    }

    fn visit_object_property(&mut self, it: &oxc_ast::ast::ObjectProperty<'a>) {
        if matches!(it.key.static_name().as_deref(), Some("class" | "className")) {
            self.mark(&it.value);
        }
        oxc_ast_visit::walk::walk_object_property(self, it);
    }

    fn visit_string_literal(&mut self, it: &oxc_ast::ast::StringLiteral<'a>) {
        let (start, end) = (it.span.start as usize, it.span.end as usize);
        if end > start + 1 {
            let in_context = self.contexts.contains(&it.span.start);
            self.literal(&[(start + 1, &self.source[start + 1..end - 1])], in_context);
        }
    }

    fn visit_template_literal(&mut self, it: &oxc_ast::ast::TemplateLiteral<'a>) {
        let in_context = self.contexts.contains(&it.span.start);
        let pieces: Vec<(usize, &str)> = it
            .quasis
            .iter()
            .map(|q| (q.span.start as usize, &self.source[q.span.start as usize..q.span.end as usize]))
            .collect();
        self.literal(&pieces, in_context);
        oxc_ast_visit::walk::walk_template_literal(self, it);
    }
}

#[cfg(all(test, feature = "minify-js"))]
mod tests {
    use super::*;

    fn classes(c: &str) -> bool {
        ["flex", "hidden", "block", "gap-2", "text-xs", "badge", "sm:flex", "mt-0.5", "card"].contains(&c)
    }

    fn rename(c: &str) -> Option<String> {
        Some(format!("_{}", c.len()))
    }

    #[test]
    fn class_lists_are_rewritten_and_lone_words_kept() {
        let src = r#"el("div", "flex gap-2 "); x.style.display = "block"; y.classList.add("hidden"); z.className = cond ? "flex" : "hidden sm:flex";
const t = `text-xs ${size} mt-0.5 gap-${n}`; note(" code block "); q(".card .badge, div.badge:hover"); h(`<a class="flex badge">${t}</a>`);"#;
        let occ = analyze(src, &classes).unwrap();
        let out = occ.apply(src, &rename);
        assert_eq!(
            out,
            r#"el("div", "_4 _5 "); x.style.display = "block"; y.classList.add("_6"); z.className = cond ? "_4" : "_6 _7";
const t = `_7 ${size} _6 gap-${n}`; note(" code block "); q("._4 ._5, div.badge:hover"); h(`<a class="_4 _5">${t}</a>`);"#
        );
        assert_eq!(occ.fixed, BTreeSet::from(["badge".to_owned(), "block".to_owned()]));
    }

    #[test]
    fn broken_scripts_are_reported() {
        assert!(analyze("function (", &classes).is_err());
    }
}
