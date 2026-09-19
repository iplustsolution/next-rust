//! JavaScript minification for release builds (`[build] minify_js`).
//!
//! The scripts in `client/` and `assets/` are compressed and their local
//! names mangled with the [oxc](https://oxc.rs) minifier before they are
//! compiled into the binary, so browsers download only what runs and the
//! source stays on the build machine. Nothing changes for development
//! builds, which serve the files from disk as written.
//!
//! Minification never breaks a build: a file the minifier cannot handle is
//! embedded as written, with a build warning naming it.

/// Minify `source`. `module` says whether it is an ES module (a `client/`
/// file loaded with `import()`), which lets top-level names be mangled too;
/// a classic script keeps its top-level names, since other scripts may use
/// them.
#[cfg(feature = "minify-js")]
pub fn minify(source: &str, module: bool) -> Result<String, String> {
    use oxc_allocator::Allocator;
    use oxc_codegen::{Codegen, CodegenOptions, CommentOptions};
    use oxc_minifier::{CompressOptions, MangleOptions, Minifier, MinifierOptions};
    use oxc_parser::Parser;
    use oxc_span::SourceType;

    // The minifier is third-party code: a panic in it must not stop the build.
    let result = std::panic::catch_unwind(|| {
        let allocator = Allocator::default();
        let source_type = if module { SourceType::mjs() } else { SourceType::cjs().with_module(false) };
        let parsed = Parser::new(&allocator, source, source_type).parse();
        if let Some(error) = parsed.errors.first() {
            return Err(error.to_string());
        }
        let mut program = parsed.program;
        let options = MinifierOptions {
            mangle: Some(MangleOptions { top_level: module, ..MangleOptions::default() }),
            compress: Some(CompressOptions::default()),
        };
        let minified = Minifier::new(options).minify(&allocator, &mut program);
        let comments = CommentOptions { normal: false, jsdoc: false, annotation: true, ..CommentOptions::default() };
        let code = Codegen::new()
            .with_options(CodegenOptions { minify: true, comments, ..CodegenOptions::default() })
            .with_scoping(minified.scoping)
            .build(&program)
            .code;
        Ok(code)
    });
    match result {
        Ok(done) => done,
        Err(_) => Err("the minifier failed unexpectedly".into()),
    }
}

#[cfg(not(feature = "minify-js"))]
pub fn minify(_source: &str, _module: bool) -> Result<String, String> {
    Err("next-rust-build was compiled without the `minify-js` feature".into())
}

/// Minify a script whose kind is not known (`assets/`): as a classic script
/// first, then as a module when it uses `import`/`export`.
pub fn minify_any(source: &str) -> Result<String, String> {
    minify(source, false).or_else(|_| minify(source, true))
}

#[cfg(all(test, feature = "minify-js"))]
mod tests {
    use super::*;

    #[test]
    fn modules_keep_their_exports_and_lose_their_comments() {
        let src = "// A comment.\nexport function hydrate(island, props) {\n  const total = props.a + props.b;\n  island.textContent = `sum ${total}`;\n}\n";
        let out = minify(src, true).unwrap();
        assert!(out.contains("hydrate"), "{out}");
        assert!(!out.contains("comment") && !out.contains('\n'), "{out}");
        assert!(out.len() < src.len());
    }

    #[test]
    fn classic_scripts_keep_top_level_names() {
        let src = "var helper = function (a) { return a + 1; };\nfunction visible(x) { return helper(x) * 2; }\n";
        let out = minify(src, false).unwrap();
        assert!(out.contains("helper") && out.contains("visible"), "{out}");
        assert!(minify_any("import x from './x.js'; export const y = x;").is_ok());
    }

    #[test]
    fn broken_scripts_are_reported_not_panicked() {
        assert!(minify("function (", true).is_err());
        assert!(minify_any("let = ;").is_err());
    }
}
