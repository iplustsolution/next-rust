fn main() {
    next_rust_build::Generator::new()
        .tailwind(|tw| {
            tw.enabled = true;
            // Design tokens. `light-dark()` picks the light or dark value from
            // the system setting, so one class (`bg-bg`, `text-muted`) covers
            // both themes.
            let colors = [
                ("bg", "#ffffff", "#0b0b0d"),
                ("soft", "#fafafa", "#111114"),
                ("raised", "#ffffff", "#16161a"),
                ("fg", "#111113", "#ededef"),
                ("fg-soft", "#3f3f46", "#c4c4cc"),
                ("muted", "#6b6b76", "#8b8b96"),
                ("line", "#ebebef", "#232329"),
                ("line-strong", "#dcdce2", "#2f2f37"),
                ("accent", "#e0561a", "#f46a25"),
                ("accent-fg", "#c2410c", "#ff8a4c"),
                ("accent-soft", "rgb(224 86 26 / 0.09)", "rgb(244 106 37 / 0.12)"),
                ("on-accent", "#ffffff", "#140800"),
                ("code", "#fafafa", "#101013"),
                ("header", "rgb(255 255 255 / 0.8)", "rgb(11 11 13 / 0.78)"),
                // Syntax highlighting (see src/highlight.rs).
                ("syn-keyword", "#c2410c", "#ff8f5a"),
                ("syn-function", "#1d4ed8", "#8ab4ff"),
                ("syn-type", "#a16207", "#f5c877"),
                ("syn-macro", "#0f766e", "#6fd7c3"),
                ("syn-string", "#15803d", "#a5d6a7"),
                ("syn-number", "#7e22ce", "#d6a6ff"),
                ("syn-comment", "#8c8c96", "#6b6b76"),
                ("syn-attribute", "#6d28d9", "#b39ddb"),
            ];
            for (name, light, dark) in colors {
                tw.theme.insert(format!("color-{name}"), format!("light-dark({light}, {dark})"));
            }
            tw.theme.insert(
                "font-sans".into(),
                r#"ui-sans-serif, system-ui, -apple-system, "Segoe UI", Roboto, "Helvetica Neue", Arial, sans-serif"#.into(),
            );
            tw.theme.insert(
                "font-mono".into(),
                r#"ui-monospace, "SF Mono", "JetBrains Mono", Menlo, Consolas, "Liberation Mono", monospace"#.into(),
            );
            // The document itself: color scheme, background, anchor offset below
            // the sticky header, and a focus ring for keyboard users.
            tw.css = "@layer base {\n  html { @apply scheme-light-dark bg-bg scroll-pt-22 motion-safe:scroll-smooth; }\n  :focus-visible { @apply rounded-md outline-2 outline-offset-2 outline-accent; }\n  html[data-nr-navigating] body { @apply cursor-progress; }\n}\n".into();
        })
        .run();
}
