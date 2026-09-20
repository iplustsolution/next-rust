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
                // Ambient background (src/ui.rs `ambient`). One warm bloom from
                // a point above the top edge, spreading outwards: glow-1 is its
                // core, glow-2..4 the falloff. Light mode stays very restrained
                // — anything stronger reads as a dirty smudge on white.
                ("glow-1", "rgb(244 106 37 / 0.11)", "rgb(255 130 64 / 0.22)"),
                ("glow-2", "rgb(240 100 32 / 0.065)", "rgb(250 104 40 / 0.13)"),
                ("glow-3", "rgb(236 94 28 / 0.032)", "rgb(240 88 28 / 0.062)"),
                ("glow-4", "rgb(232 90 26 / 0.011)", "rgb(228 74 24 / 0.023)"),
                ("glow-amber", "rgb(255 168 66 / 0.05)", "rgb(255 176 88 / 0.07)"),
                ("glow-ember", "rgb(198 68 20 / 0.024)", "rgb(168 46 14 / 0.085)"),
                ("glow-wash", "rgb(17 17 19 / 0.013)", "rgb(255 232 208 / 0.012)"),
                ("glow-edge", "rgb(224 86 26 / 0.42)", "rgb(244 106 37 / 0.5)"),
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
            // Fluid type scale: every size on the site is in rem, so the whole
            // layout grows with the root font size. It stays at the reader's
            // own size (100%) up to 1600px, then grows with the viewport to
            // 162.5% (26px at the default 16px) on a 4K screen.
            // The document itself: color scheme, background, anchor offset below
            // the sticky header, and a focus ring for keyboard users.
            tw.css = "@layer base {\n  html { font-size: clamp(100%, calc(55% + 0.45vw), 162.5%); @apply scheme-light-dark bg-bg scroll-pt-22 motion-safe:scroll-smooth; }\n  :focus-visible { @apply rounded-md outline-2 outline-offset-2 outline-accent; }\n  html[data-nr-navigating] body { @apply cursor-progress; }\n}\n".into();
            // Landing page: the animated logo in the hero (app/page.rs). The
            // classes are only ever used behind `motion-safe:`.
            tw.theme.insert("animate-gear-spin".into(), "gear-spin 48s linear infinite".into());
            tw.theme.insert("animate-orbit-spin".into(), "orbit-spin 90s linear infinite".into());
            tw.theme.insert("animate-ticks-spin".into(), "gear-spin 140s linear infinite".into());
            tw.theme.insert("animate-glow-pulse".into(), "glow-pulse 5s ease-in-out infinite".into());
            // The site-wide ambient background breathes very slowly, so it
            // reads as light rather than as a moving graphic.
            tw.theme.insert("animate-ambient-drift".into(), "ambient-drift 26s ease-in-out infinite".into());
            tw.css.push_str(
                "@keyframes gear-spin { to { transform: rotate(360deg); } }\n@keyframes orbit-spin { to { transform: rotate(-360deg); } }\n@keyframes glow-pulse { 0%, 100% { opacity: 0.55; transform: scale(0.94); } 50% { opacity: 1; transform: scale(1.04); } }\n@keyframes ambient-drift { 0%, 100% { transform: translate3d(0, 0, 0) scale(1); } 50% { transform: translate3d(0, 1.2%, 0) scale(1.07); } }\n",
            );
        })
        .run();
}
