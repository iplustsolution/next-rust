//! The motion stylesheet: what `[animation]` in `next-rust.toml` turns into.
//!
//! Written once per process from the configuration and placed in every
//! document's `<head>` right after the metadata, as its first stylesheet:
//! before Tailwind, the UI components and the app's own styles. It has two
//! parts:
//!
//! * **Pages fade into each other.** Full loads use cross-document view
//!   transitions (`@view-transition { navigation: auto }`), and the client
//!   runtime swaps pages inside `document.startViewTransition`, reading the
//!   length of the fade from `--nr-page-fade` on `<html>` (`0ms` means none,
//!   which is also what reduced-motion users get). Browsers without view
//!   transitions get a plain fade out and in (`data-nr-fade` on `<html>`).
//! * **Hover and state changes ease.** One rule in the lowest cascade layer
//!   gives every element a transition on the properties hover, focus and
//!   class changes usually touch. Being in a layer declared before
//!   Tailwind's, anything the app says wins: a `transition` in its own CSS,
//!   a `transition-*` utility, or `data-nr-motion="off"` on a subtree.
//!
//! Because the sheet carries a `data-nr-css` id like any stylesheet, client
//! navigations never download it twice.

use next_rust_core::config::{AnimationConfig, LoaderConfig};

/// The motion stylesheet of an app: `(id, css)`, or `None` when everything
/// is off.
pub(crate) fn stylesheet(config: &AnimationConfig) -> Option<(String, String)> {
    let (pages, hover) = (config.pages(), config.hover());
    if !pages && !hover {
        return None;
    }
    let mut css = String::with_capacity(2048);
    // The layer comes first in the document, so it is the lowest one.
    css.push_str("@layer nr-motion,theme,base,components,utilities;");
    css.push_str("@layer nr-motion{:root{");
    css.push_str(&format!("--nr-page-fade:{}ms;", if pages { config.page_duration } else { 0 }));
    css.push_str(&format!("--nr-motion:{}ms;", if hover { config.duration } else { 0 }));
    css.push_str(&format!("--nr-motion-ease:{}", css_value(&config.easing)));
    css.push('}');
    if hover {
        css.push_str("*,::before,::after{transition-property:");
        css.push_str(TRANSITIONED);
        css.push_str(";transition-duration:var(--nr-motion);transition-timing-function:var(--nr-motion-ease)}");
        css.push_str("[data-nr-motion=off],[data-nr-motion=off] *{transition-property:none}");
    }
    css.push_str("@media (prefers-reduced-motion:reduce){:root{--nr-page-fade:0ms;--nr-motion:0ms}}}");
    if pages {
        css.push_str("@media (prefers-reduced-motion:no-preference){@view-transition{navigation:auto}}");
        css.push_str(
            "::view-transition-old(root){animation:var(--nr-page-fade) cubic-bezier(.4,0,1,1) both nr-fade-out}",
        );
        css.push_str(
            "::view-transition-new(root){animation:var(--nr-page-fade) cubic-bezier(0,0,.2,1) both nr-fade-in}",
        );
        css.push_str("html[data-nr-fade=out] body{opacity:0;transition:opacity calc(var(--nr-page-fade)/2) ease-out}");
        css.push_str("html[data-nr-fade=in] body{animation:nr-fade-in calc(var(--nr-page-fade)/2) ease-in}");
        css.push_str("@keyframes nr-fade-in{from{opacity:0}}@keyframes nr-fade-out{to{opacity:0}}");
    }
    let id = format!("nr-motion-{}", next_rust_assets::content_hash(css.as_bytes()));
    Some((id, css))
}

/// The loading-bar stylesheet of an app (`[loader]`): `(id, css)`, or `None`
/// when off. The bar is `html[data-nr-loading]::before`, so it lives outside
/// `<body>` and survives the body being replaced on navigation. It spans the
/// page and is slid in from the left by a transform, `--nr-load` of the way,
/// which the client runtime moves: no layout, the compositor does the work.
/// While loading the runtime redraws the target every [`LOADER_STEP`] ms and a
/// linear transition a little longer carries the bar there, so it never rests;
/// `data-nr-loading=done` fills it with the configured easing, then fades it.
/// `--nr-loader`, `--nr-load-delay` and `--nr-load-speed` on `<html>` tell the
/// runtime the bar exists and how it is timed. With `shadow` the bar glows and
/// its leading edge carries a brighter peg. Reduced motion gets a bar that
/// fades in and out in place instead of travelling.
pub(crate) fn loader(config: &LoaderConfig) -> Option<(String, String)> {
    if !config.enabled || config.height == 0 {
        return None;
    }
    let color = css_value(&config.color);
    let easing = css_value(&config.easing);
    let (edge, z, height) = (if config.at_bottom { "bottom" } else { "top" }, config.z_index, config.height);
    let speed = config.speed.max(1);
    let mut css = String::with_capacity(2048);
    css.push_str(&format!(":root{{--nr-loader:1;--nr-load-delay:{}ms;--nr-load-speed:{speed}ms}}", config.delay));
    css.push_str(&format!(
        "html[data-nr-loading]::before{{content:\"\";position:fixed;left:0;right:0;{edge}:0;z-index:{z};\
pointer-events:none;transform:translate3d(calc(var(--nr-load,0%) - 100%),0,0);\
transition:transform {}ms linear;will-change:transform,opacity;",
        LOADER_STEP * 5 / 4
    ));
    if config.shadow {
        // A taller box: the bar along its edge, a radial peg under the leading
        // end, and a glow around both.
        css.push_str(&format!(
            "height:{}px;background:linear-gradient({color},{color}) 0 {edge}/100% {height}px no-repeat,\
radial-gradient(ellipse at 100% {},{color},transparent 70%) 100% {edge}/100px 100% no-repeat;\
filter:drop-shadow(0 0 5px {color})",
            height + 6,
            if config.at_bottom { "100%" } else { "0" }
        ));
    } else {
        css.push_str(&format!("height:{height}px;background:{color}"));
    }
    css.push_str(&format!(
        "}}html[data-nr-loading=done]::before{{opacity:0;\
transition:transform {speed}ms {easing},opacity {speed}ms ease {speed}ms}}"
    ));
    if config.spinner {
        css.push_str(&format!(
            "html[data-nr-loading]::after{{content:\"\";position:fixed;{edge}:15px;right:15px;z-index:{z};\
width:18px;height:18px;box-sizing:border-box;border:2px solid transparent;\
border-top-color:{color};border-left-color:{color};border-radius:50%;\
animation:nr-spin .5s linear infinite,nr-load-in {speed}ms ease;pointer-events:none}}\
html[data-nr-loading=done]::after{{opacity:0;transition:opacity {speed}ms ease {speed}ms}}\
@keyframes nr-spin{{to{{transform:rotate(360deg)}}}}"
        ));
    }
    css.push_str(&format!(
        "@keyframes nr-load-in{{from{{opacity:0}}}}@media (prefers-reduced-motion:reduce){{\
html[data-nr-loading]::before{{transform:none;transition:none;animation:nr-load-in {speed}ms ease}}\
html[data-nr-loading=done]::before{{transition:opacity {speed}ms ease}}"
    ));
    if config.spinner {
        css.push_str(&format!(
            "html[data-nr-loading]::after{{animation-duration:1.5s,{speed}ms}}\
html[data-nr-loading=done]::after{{transition:opacity {speed}ms ease}}"
        ));
    }
    css.push('}');
    let id = format!("nr-loader-{}", next_rust_assets::content_hash(css.as_bytes()));
    Some((id, css))
}

/// How often, in milliseconds, the client runtime moves the loading bar while
/// a page loads (`STEP` in the runtime's loader).
const LOADER_STEP: u32 = 200;

/// What a hover, focus, active or class change usually alters: colors,
/// shadows, opacity, transforms, size, spacing and type. Layout-only
/// properties that scripts set constantly (`display`, `position`,
/// `overflow`) and `all` are left out on purpose.
const TRANSITIONED: &str = "color,background-color,border-color,outline-color,text-decoration-color,\
fill,stroke,opacity,box-shadow,text-shadow,filter,backdrop-filter,\
transform,translate,scale,rotate,\
width,height,min-width,max-width,min-height,max-height,flex-basis,flex-grow,\
padding,margin,gap,inset,border-radius,border-width,outline-offset,\
font-size,font-weight,font-variation-settings,letter-spacing,line-height,text-underline-offset,\
background-position,background-size,clip-path,visibility";

/// A configured value, kept to characters that cannot close the rule or the
/// `<style>` element.
fn css_value(value: &str) -> String {
    let clean: String = value.chars().filter(|c| !matches!(c, ';' | '{' | '}' | '<' | '>' | '\n')).collect();
    let clean = clean.trim();
    if clean.is_empty() { "ease".to_owned() } else { clean.to_owned() }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_fade_pages_and_ease_hover() {
        let (id, css) = stylesheet(&AnimationConfig::default()).unwrap();
        assert!(id.starts_with("nr-motion-"));
        assert!(css.starts_with("@layer nr-motion,theme,base,components,utilities;@layer nr-motion{"));
        assert!(css.contains("--nr-page-fade:250ms;--nr-motion:200ms;--nr-motion-ease:cubic-bezier(0.2, 0, 0, 1)}"));
        assert!(css.contains("*,::before,::after{transition-property:color,"));
        assert!(css.contains("@view-transition{navigation:auto}"));
        assert!(css.contains("prefers-reduced-motion:reduce){:root{--nr-page-fade:0ms;--nr-motion:0ms}"));
    }

    #[test]
    fn each_part_can_be_turned_off() {
        let no_pages = AnimationConfig { pages: false, ..AnimationConfig::default() };
        let (_, css) = stylesheet(&no_pages).unwrap();
        assert!(css.contains("--nr-page-fade:0ms") && !css.contains("@view-transition"));
        assert!(css.contains("transition-property:color"));

        let no_hover = AnimationConfig { hover: false, ..AnimationConfig::default() };
        let (_, css) = stylesheet(&no_hover).unwrap();
        assert!(css.contains("--nr-motion:0ms") && !css.contains("transition-property:color"));
        assert!(css.contains("@view-transition"));

        assert!(stylesheet(&AnimationConfig { enabled: false, ..AnimationConfig::default() }).is_none());
        assert!(stylesheet(&AnimationConfig { duration: 0, page_duration: 0, ..AnimationConfig::default() }).is_none());
    }

    #[test]
    fn the_id_follows_the_content() {
        let a = stylesheet(&AnimationConfig::default()).unwrap().0;
        let b = stylesheet(&AnimationConfig { duration: 150, ..AnimationConfig::default() }).unwrap().0;
        assert_ne!(a, b);
        assert_eq!(a, stylesheet(&AnimationConfig::default()).unwrap().0);
    }

    #[test]
    fn the_loader_follows_its_configuration() {
        let (id, css) = loader(&LoaderConfig::default()).unwrap();
        assert!(id.starts_with("nr-loader-"));
        assert!(css.starts_with(":root{--nr-loader:1;--nr-load-delay:100ms;--nr-load-speed:200ms}"));
        assert!(
            css.contains(
                "html[data-nr-loading]::before{content:\"\";position:fixed;left:0;right:0;top:0;z-index:1600;"
            )
        );
        // Moved by the compositor, never by layout; continuous while loading.
        assert!(
            css.contains(
                "transform:translate3d(calc(var(--nr-load,0%) - 100%),0,0);transition:transform 250ms linear;"
            )
        );
        assert!(!css.contains("width:var("));
        // The bar, the peg under its leading end, and the glow.
        assert!(css.contains("height:10px;background:linear-gradient(var(--nr-primary, #006fee),var(--nr-primary, #006fee)) 0 top/100% 4px no-repeat,"));
        assert!(css.contains("radial-gradient(ellipse at 100% 0,var(--nr-primary, #006fee),transparent 70%) 100% top/100px 100% no-repeat;"));
        assert!(css.contains("filter:drop-shadow(0 0 5px var(--nr-primary, #006fee))"));
        // Done: fill with the configured easing, then fade.
        assert!(css.contains(
            "html[data-nr-loading=done]::before{opacity:0;transition:transform 200ms ease,opacity 200ms ease 200ms}"
        ));
        assert!(css.contains(
            "@media (prefers-reduced-motion:reduce){html[data-nr-loading]::before{transform:none;transition:none;"
        ));
        assert!(!css.contains("::after"), "no spinner by default");

        let custom = LoaderConfig {
            color: "#f26b2a".into(),
            height: 3,
            shadow: false,
            spinner: true,
            at_bottom: true,
            z_index: 99,
            speed: 300,
            ..LoaderConfig::default()
        };
        let (other, css) = loader(&custom).unwrap();
        assert_ne!(other, id);
        assert!(css.contains("left:0;right:0;bottom:0;z-index:99;"));
        assert!(css.contains("height:3px;background:#f26b2a}"));
        assert!(!css.contains("drop-shadow") && !css.contains("gradient"));
        assert!(css.contains("html[data-nr-loading]::after{animation-duration:1.5s,300ms}"));
        assert!(css.ends_with("html[data-nr-loading=done]::after{transition:opacity 300ms ease}}"));
        assert!(css.contains("html[data-nr-loading]::after{content:\"\";position:fixed;bottom:15px;right:15px;"));
        assert!(css.contains("@keyframes nr-spin"));
        assert!(css.contains("--nr-load-speed:300ms"));

        assert!(loader(&LoaderConfig { enabled: false, ..LoaderConfig::default() }).is_none());
        assert!(loader(&LoaderConfig { height: 0, ..LoaderConfig::default() }).is_none());
    }

    #[test]
    fn a_configured_easing_cannot_break_out_of_the_sheet() {
        let c = AnimationConfig { easing: "ease}</style><script>".into(), ..AnimationConfig::default() };
        let (_, css) = stylesheet(&c).unwrap();
        assert!(!css.contains("</style>") && !css.contains("<script"));
        assert!(css.contains("--nr-motion-ease:ease/stylescript}"));
    }
}
