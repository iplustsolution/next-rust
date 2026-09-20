//! Small charts rendered on the server as HTML and SVG: no script, no
//! canvas. Hover (or the keyboard-free reading order of a screen reader)
//! gives the exact values; colors follow the theme in light and dark.

use next_rust_view::{Element, Node, View};

use crate::button::{common_methods, with_parts};
use crate::{Color, Extra, apply, attr, part, props, root, styled, trim_float};

/// At most this many series get a color of their own; the rest fold into "Other".
const MAX_SERIES: usize = 8;

/// `1234567.5` → `"1,234,567.5"` (two decimals at most).
fn full(v: f64) -> String {
    let rounded = (v * 100.0).round() / 100.0;
    let text = trim_float(rounded.abs());
    let (int, frac) = text.split_once('.').map_or((text.as_str(), None), |(i, f)| (i, Some(f)));
    let mut out = String::new();
    for (i, c) in int.chars().enumerate() {
        if i > 0 && (int.len() - i) % 3 == 0 {
            out.push(',');
        }
        out.push(c);
    }
    if let Some(frac) = frac {
        out.push('.');
        out.push_str(frac);
    }
    if rounded < 0.0 { format!("-{out}") } else { out }
}

/// `1200` → `"1.2K"`, `3400000` → `"3.4M"`: for axis ticks.
fn compact(v: f64) -> String {
    let a = v.abs();
    let (scaled, suffix) = if a >= 1e9 {
        (v / 1e9, "B")
    } else if a >= 1e6 {
        (v / 1e6, "M")
    } else if a >= 1e4 {
        (v / 1e3, "K")
    } else {
        (v, "")
    };
    format!("{}{suffix}", trim_float((scaled * 10.0).round() / 10.0))
}

/// The smallest "round" number at or above `v`: 1, 2, 2.5, 5 or 10 times a power of ten.
fn nice_ceil(v: f64) -> f64 {
    if v <= 0.0 || !v.is_finite() {
        return 1.0;
    }
    let pow = 10f64.powf(v.log10().floor());
    for m in [1.0, 2.0, 2.5, 5.0, 10.0] {
        if m * pow >= v * 0.999_999 {
            return m * pow;
        }
    }
    10.0 * pow
}

fn percent(v: f64) -> String {
    trim_float((v * 100.0).round() / 100.0)
}

/// Horizontal grid lines with their values: 0, half and the top of the scale.
fn grid(top: f64, unit: &str) -> Node {
    let mut el = part("div", "nr-chart-grid").with(attr("aria-hidden", "true"));
    for (at, v) in [(0, 0.0), (50, top / 2.0), (100, top)] {
        el.children.push(
            Element::new("span")
                .with(attr("style", format!("bottom:{at}%")))
                .with(Element::new("i").with(format!("{}{unit}", compact(v))))
                .into_node(),
        );
    }
    el.into_node()
}

fn series_class(i: usize) -> String {
    format!("nr-series-{}", i % MAX_SERIES + 1)
}

fn height_style(height: Option<u16>) -> Option<next_rust_view::Attr> {
    height.map(|h| attr("style", format!("--nr-chart-h:{h}px")))
}

/// A tiny line showing a trend, for a stat or a table cell. It stretches to
/// its `width` and `height`; `role="img"` with the first and last values as
/// its name.
///
/// ```
/// # use next_rust_ui::*; use next_rust_view::*;
/// let s = Sparkline![values = [3.0, 5.0, 4.0, 8.0], color = Color::Success, label = "Signups"];
/// let html = render_static(s);
/// assert!(html.contains(r#"role="img" aria-label="Signups: 3 to 8""#) && html.contains("<path"), "{html}");
/// ```
#[derive(Default)]
pub struct Sparkline {
    values: Vec<f64>,
    label: Option<String>,
    width: Option<u16>,
    height: Option<u16>,
    area: Option<bool>,
    color: Color,
    unstyled: bool,
    extra: Extra,
}

impl Sparkline {
    pub fn new() -> Self {
        Self::default()
    }
    common_methods!();
    with_parts!();
    props! {
        /// Accessible name ("Trend" by default).
        label: text,
        /// Pixels (120 by default).
        width: opt<u16>,
        /// Pixels (32 by default).
        height: opt<u16>,
        /// A soft fill under the line (on by default).
        area: opt<bool>,
        color: val<Color>,
    }
    pub fn values(mut self, v: impl IntoIterator<Item = f64>) -> Self {
        self.values = v.into_iter().filter(|v| v.is_finite()).collect();
        self
    }
}

/// Points of `values` in a 100 × 100 box (y grows downward), between `lo` and `hi`.
fn points(values: &[f64], lo: f64, hi: f64) -> Vec<(f64, f64)> {
    let span = if hi > lo { hi - lo } else { 1.0 };
    let last = values.len().saturating_sub(1).max(1) as f64;
    values.iter().enumerate().map(|(i, v)| (i as f64 / last * 100.0, 100.0 - (v - lo) / span * 100.0)).collect()
}

fn line_path(points: &[(f64, f64)]) -> String {
    let mut d = String::new();
    for (i, (x, y)) in points.iter().enumerate() {
        d.push_str(&format!("{}{} {}", if i == 0 { "M" } else { "L" }, percent(*x), percent(*y)));
    }
    d
}

fn svg_path(class: String, d: String) -> Node {
    Element::new("path")
        .with(attr("class", class))
        .with(attr("d", d))
        .with(attr("vector-effect", "non-scaling-stroke"))
        .into_node()
}

impl View for Sparkline {
    fn into_node(self) -> Node {
        let (class, attrs, _) = self.extra.split();
        let mut el = root("span", &["nr-sparkline", self.color.class()], self.unstyled, class);
        let (w, h) = (self.width.unwrap_or(120), self.height.unwrap_or(32));
        el.set_attr(attr("style", format!("width:{w}px;height:{h}px")));
        el.set_attr(attr("role", "img"));
        let name = self.label.unwrap_or_else(|| "Trend".to_owned());
        let label = match (self.values.first(), self.values.last()) {
            (Some(a), Some(b)) => format!("{name}: {} to {}", full(*a), full(*b)),
            _ => name,
        };
        el.set_attr(attr("aria-label", label));
        apply(&mut el, attrs);
        if self.values.len() > 1 {
            let lo = self.values.iter().copied().fold(f64::INFINITY, f64::min);
            let hi = self.values.iter().copied().fold(f64::NEG_INFINITY, f64::max);
            let pts = points(&self.values, lo, hi);
            let mut svg = Element::new("svg")
                .with(attr("viewBox", "0 0 100 100"))
                .with(attr("preserveAspectRatio", "none"))
                .with(attr("aria-hidden", "true"));
            let line = line_path(&pts);
            if self.area.unwrap_or(true) {
                svg.children.push(svg_path("nr-sparkline-area".to_owned(), format!("{line}L100 100L0 100Z")));
            }
            svg.children.push(svg_path("nr-sparkline-line".to_owned(), line));
            el.children.push(svg.into_node());
            let (_, y) = pts[pts.len() - 1];
            el.children
                .push(part("i", "nr-sparkline-dot").with(attr("style", format!("top:{}%", percent(y)))).into_node());
        }
        styled(el)
    }
}

/// Bars comparing a few values. HTML and CSS only; each bar shows its value
/// on hover, and the chart reads as a list to a screen reader. Values below
/// zero draw as zero.
///
/// ```
/// # use next_rust_ui::*; use next_rust_view::*;
/// let c = BarChart![title = "Signups", data = [("Mon", 12.0), ("Tue", 30.0)], unit = "%"];
/// let html = render_static(c);
/// assert!(html.contains("Tue: 30%") && html.contains("height:60%") && html.contains("height:24%"), "{html}");
/// ```
#[derive(Default)]
pub struct BarChart {
    data: Vec<(String, f64)>,
    title: Option<String>,
    unit: Option<String>,
    height: Option<u16>,
    show_values: bool,
    max: Option<f64>,
    color: Color,
    unstyled: bool,
    extra: Extra,
}

impl BarChart {
    pub fn new() -> Self {
        Self::default()
    }
    common_methods!();
    with_parts!();
    props! {
        /// A caption above the chart (also names it).
        title: text,
        /// Text after each value ("%", " ms").
        unit: text,
        /// Height of the plot in pixels (192 by default).
        height: opt<u16>,
        /// Print each value above its bar instead of showing it on hover.
        show_values: val<bool>,
        /// Top of the scale; a round number above the largest value by default.
        max: opt<f64>,
        color: val<Color>,
    }
    /// `(label, value)` pairs, in order.
    pub fn data<S: Into<String>>(mut self, v: impl IntoIterator<Item = (S, f64)>) -> Self {
        self.data = v.into_iter().map(|(l, v)| (l.into(), if v.is_finite() { v } else { 0.0 })).collect();
        self
    }
}

impl View for BarChart {
    fn into_node(self) -> Node {
        let (class, attrs, _) = self.extra.split();
        let mut el = root("figure", &["nr-chart", "nr-barchart", self.color.class()], self.unstyled, class);
        el.set_attr(height_style(self.height).unwrap_or_else(next_rust_view::Attr::none));
        apply(&mut el, attrs);
        if let Some(title) = self.title {
            el.children.push(part("figcaption", "nr-chart-title").with(title).into_node());
        }
        let unit = self.unit.unwrap_or_default();
        let largest = self.data.iter().map(|(_, v)| *v).fold(0.0, f64::max);
        let top = self.max.filter(|m| *m > 0.0).unwrap_or_else(|| nice_ceil(largest));
        let mut plot = part("div", "nr-chart-plot").with(grid(top, &unit));
        let mut bars = part("ul", if self.show_values { "nr-bars nr-bars-values" } else { "nr-bars" });
        for (label, value) in self.data {
            let h = (value.max(0.0) / top * 100.0).min(100.0);
            let bar = part("span", "nr-bar")
                .with(attr("style", format!("height:{}%", percent(h))))
                .with(part("span", "nr-bar-tip").with(format!("{label}: {}{unit}", full(value))));
            bars.children.push(
                part("li", "nr-bar-slot")
                    .with(bar)
                    .with(part("span", "nr-bar-label").with(attr("aria-hidden", "true")).with(label))
                    .into_node(),
            );
        }
        plot.children.push(bars.into_node());
        el.children.push(plot.into_node());
        styled(el)
    }
}

/// One or more lines over the same points in time. The lines are SVG;
/// moving over the chart shows a marker and the values at that point, with
/// CSS alone. With two or more series a legend names them.
///
/// ```
/// # use next_rust_ui::*; use next_rust_view::*;
/// let c = LineChart![title = "Revenue", labels = ["Jan", "Feb", "Mar"],
///     series = [("2025", vec![10.0, 14.0, 12.0]), ("2026", vec![12.0, 18.0, 25.0])]];
/// let html = render_static(c);
/// assert!(html.contains("nr-chart-legend") && html.contains("nr-series-2") && html.contains("2026: 25"), "{html}");
/// ```
#[derive(Default)]
pub struct LineChart {
    labels: Vec<String>,
    series: Vec<(String, Vec<f64>)>,
    title: Option<String>,
    unit: Option<String>,
    height: Option<u16>,
    area: Option<bool>,
    max: Option<f64>,
    color: Color,
    unstyled: bool,
    extra: Extra,
}

impl LineChart {
    pub fn new() -> Self {
        Self::default()
    }
    common_methods!();
    with_parts!();
    props! {
        title: text,
        /// Text after each value.
        unit: text,
        /// Height of the plot in pixels (192 by default).
        height: opt<u16>,
        /// A soft fill under a single line (on by default; never with several series).
        area: opt<bool>,
        /// Top of the scale.
        max: opt<f64>,
        /// Color of a single series; several series use the chart palette (`--nr-chart-1` … `--nr-chart-8`).
        color: val<Color>,
    }
    /// The points along the x axis, one per value.
    pub fn labels<S: Into<String>>(mut self, v: impl IntoIterator<Item = S>) -> Self {
        self.labels = v.into_iter().map(Into::into).collect();
        self
    }
    /// `(name, values)` per line.
    pub fn series<S: Into<String>, V: IntoIterator<Item = f64>>(mut self, v: impl IntoIterator<Item = (S, V)>) -> Self {
        self.series =
            v.into_iter().map(|(n, vals)| (n.into(), vals.into_iter().filter(|v| v.is_finite()).collect())).collect();
        self
    }
    /// A single unnamed line.
    pub fn values(mut self, v: impl IntoIterator<Item = f64>) -> Self {
        self.series = vec![(String::new(), v.into_iter().filter(|v| v.is_finite()).collect())];
        self
    }
}

fn legend(names: impl Iterator<Item = (String, Option<String>)>) -> Node {
    let mut list = part("ul", "nr-chart-legend");
    for (i, (name, value)) in names.enumerate() {
        let mut li = Element::new("li")
            .with(Element::new("i").with(attr("class", format!("nr-chart-swatch {}", series_class(i)))))
            .with(Element::new("span").with(name));
        if let Some(value) = value {
            li.children.push(Element::new("strong").with(value).into_node());
        }
        list.children.push(li.into_node());
    }
    list.into_node()
}

impl View for LineChart {
    fn into_node(self) -> Node {
        let (class, attrs, _) = self.extra.split();
        let mut el = root("figure", &["nr-chart", "nr-linechart", self.color.class()], self.unstyled, class);
        el.set_attr(height_style(self.height).unwrap_or_else(next_rust_view::Attr::none));
        apply(&mut el, attrs);
        if let Some(title) = self.title {
            el.children.push(part("figcaption", "nr-chart-title").with(title).into_node());
        }
        let series: Vec<(String, Vec<f64>)> = self.series.into_iter().take(MAX_SERIES).collect();
        let many = series.len() > 1;
        if many {
            el.children.push(legend(series.iter().map(|(n, _)| (n.clone(), None))));
        }
        let unit = self.unit.unwrap_or_default();
        let n = series.iter().map(|(_, v)| v.len()).max().unwrap_or(0);
        let largest = series.iter().flat_map(|(_, v)| v.iter().copied()).fold(0.0, f64::max);
        let top = self.max.filter(|m| *m > 0.0).unwrap_or_else(|| nice_ceil(largest));
        let mut plot = part("div", "nr-chart-plot").with(grid(top, &unit));
        let mut svg = Element::new("svg")
            .with(attr("class", "nr-line-svg"))
            .with(attr("viewBox", "0 0 100 100"))
            .with(attr("preserveAspectRatio", "none"))
            .with(attr("aria-hidden", "true"));
        for (i, (_, values)) in series.iter().enumerate() {
            if values.len() < 2 {
                continue;
            }
            let class = if many { series_class(i) } else { String::new() };
            // Every series spans the full width of its own points.
            let pts = points(values, 0.0, top);
            let line = line_path(&pts);
            if !many && self.area.unwrap_or(true) {
                svg.children.push(svg_path("nr-line-area".to_owned(), format!("{line}L100 100L0 100Z")));
            }
            svg.children.push(svg_path(format!("nr-line {class}").trim_end().to_owned(), line));
        }
        plot.children.push(svg.into_node());
        if n > 1 {
            let mut cols = part("ol", "nr-line-cols");
            // A few x labels only, evenly spread: they must not collide.
            let shown = n.min(6);
            let labelled: Vec<usize> = (0..shown).map(|i| (i * (n - 1) + (shown - 1) / 2) / (shown - 1)).collect();
            for x in 0..n {
                let left = x as f64 / (n - 1) as f64 * 100.0;
                let label = self.labels.get(x).cloned().unwrap_or_else(|| (x + 1).to_string());
                let edge = if x == 0 {
                    " nr-line-col-first"
                } else if x == n - 1 {
                    " nr-line-col-last"
                } else {
                    ""
                };
                let mut col = Element::new("li")
                    .with(attr("class", format!("nr-line-col{edge}")))
                    .with(attr("style", format!("left:{}%;width:{}%", percent(left), percent(100.0 / (n - 1) as f64))));
                let mut tip = part("span", "nr-line-tip").with(Element::new("strong").with(label.clone()));
                for (i, (name, values)) in series.iter().enumerate() {
                    let Some(v) = values.get(x) else { continue };
                    let class = if many { series_class(i) } else { String::new() };
                    col.children.push(
                        Element::new("i")
                            .with(attr("class", format!("nr-line-dot {class}").trim_end().to_owned()))
                            .with(attr("style", format!("bottom:{}%", percent((v / top * 100.0).clamp(0.0, 100.0)))))
                            .into_node(),
                    );
                    let text = if name.is_empty() {
                        format!("{}{unit}", full(*v))
                    } else {
                        format!("{name}: {}{unit}", full(*v))
                    };
                    tip.children.push(
                        Element::new("span")
                            .with(
                                Element::new("i")
                                    .with(attr("class", format!("nr-chart-swatch {class}").trim_end().to_owned())),
                            )
                            .with(text)
                            .into_node(),
                    );
                }
                col.children.push(tip.into_node());
                if labelled.contains(&x) {
                    col.children
                        .push(part("span", "nr-line-label").with(attr("aria-hidden", "true")).with(label).into_node());
                }
                cols.children.push(col.into_node());
            }
            plot.children.push(cols.into_node());
        }
        el.children.push(plot.into_node());
        styled(el)
    }
}

/// Parts of a whole as a ring, with the total in the middle and a legend
/// giving every share. SVG, no script. More than eight slices fold into
/// "Other".
///
/// ```
/// # use next_rust_ui::*; use next_rust_view::*;
/// let d = Donut![title = "Traffic", center_label = "visits", data = [("Direct", 60.0), ("Search", 30.0), ("Social", 10.0)]];
/// let html = render_static(d);
/// assert!(html.contains("Direct: 60 (60%)") && html.contains(r#"<strong class="nr-donut-total">100</strong>"#), "{html}");
/// ```
#[derive(Default)]
pub struct Donut {
    data: Vec<(String, f64)>,
    title: Option<String>,
    center_label: Option<String>,
    unit: Option<String>,
    size: Option<u16>,
    unstyled: bool,
    extra: Extra,
}

impl Donut {
    pub fn new() -> Self {
        Self::default()
    }
    common_methods!();
    with_parts!();
    props! {
        title: text,
        /// A word under the total in the middle ("visits").
        center_label: text,
        /// Text after the values.
        unit: text,
        /// Diameter in pixels (160 by default).
        size: opt<u16>,
    }
    /// `(label, value)` pairs; values at or below zero are left out.
    pub fn data<S: Into<String>>(mut self, v: impl IntoIterator<Item = (S, f64)>) -> Self {
        self.data = v.into_iter().map(|(l, v)| (l.into(), v)).filter(|(_, v)| v.is_finite() && *v > 0.0).collect();
        self
    }
}

impl View for Donut {
    fn into_node(self) -> Node {
        let (class, attrs, _) = self.extra.split();
        let mut el = root("figure", &["nr-chart", "nr-donut"], self.unstyled, class);
        apply(&mut el, attrs);
        if let Some(title) = self.title {
            el.children.push(part("figcaption", "nr-chart-title").with(title).into_node());
        }
        let mut data = self.data;
        if data.len() > MAX_SERIES {
            let rest: f64 = data[MAX_SERIES - 1..].iter().map(|(_, v)| v).sum();
            data.truncate(MAX_SERIES - 1);
            data.push(("Other".to_owned(), rest));
        }
        let unit = self.unit.unwrap_or_default();
        let total: f64 = data.iter().map(|(_, v)| v).sum();
        let mut svg = Element::new("svg").with(attr("viewBox", "0 0 42 42")).with(attr("role", "img")).with(attr(
            "aria-label",
            data.iter()
                .map(|(l, v)| {
                    format!(
                        "{l}: {}{unit} ({}%)",
                        full(*v),
                        percent(v / total * 100.0).split('.').next().unwrap_or("0")
                    )
                })
                .collect::<Vec<_>>()
                .join(", "),
        ));
        let circle = |class: String| {
            Element::new("circle")
                .with(attr("class", class))
                .with(attr("cx", "21"))
                .with(attr("cy", "21"))
                .with(attr("r", "15.9155"))
                .with(attr("pathLength", "100"))
        };
        svg.children.push(circle("nr-donut-track".to_owned()).into_node());
        let gap = if data.len() > 1 { 0.8 } else { 0.0 };
        let mut offset = 0.0;
        let mut shares = Vec::new();
        for (i, (label, value)) in data.iter().enumerate() {
            let share = value / total * 100.0;
            let whole = format!("{}%", share.round() as i64);
            svg.children.push(
                circle(format!("nr-donut-seg {}", series_class(i)))
                    .with(attr(
                        "stroke-dasharray",
                        format!("{} {}", percent((share - gap).max(0.1)), percent(100.0 - (share - gap).max(0.1))),
                    ))
                    .with(attr("stroke-dashoffset", percent(-offset)))
                    .with(Element::new("title").with(format!("{label}: {}{unit} ({whole})", full(*value))))
                    .into_node(),
            );
            offset += share;
            shares.push((label.clone(), Some(whole)));
        }
        let mut ring = part("div", "nr-donut-ring");
        if let Some(size) = self.size {
            ring.set_attr(attr("style", format!("--nr-donut-d:{size}px")));
        }
        ring.children.push(svg.into_node());
        let mut center = part("div", "nr-donut-center").with(attr("aria-hidden", "true"));
        center.children.push(part("strong", "nr-donut-total").with(format!("{}{unit}", compact(total))).into_node());
        if let Some(label) = self.center_label {
            center.children.push(Element::new("span").with(label).into_node());
        }
        ring.children.push(center.into_node());
        el.children.push(part("div", "nr-donut-body").with(ring).with(legend(shares.into_iter())).into_node());
        styled(el)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn numbers_format() {
        assert_eq!(full(1_234_567.5), "1,234,567.5");
        assert_eq!(full(-1200.0), "-1,200");
        assert_eq!(full(12.345), "12.35");
        assert_eq!(compact(12_500.0), "12.5K");
        assert_eq!(compact(3_400_000.0), "3.4M");
        assert_eq!(compact(950.0), "950");
    }

    #[test]
    fn scales_are_round() {
        assert_eq!(nice_ceil(30.0), 50.0);
        assert_eq!(nice_ceil(18.0), 20.0);
        assert_eq!(nice_ceil(0.0), 1.0);
        assert_eq!(nice_ceil(100.0), 100.0);
        assert_eq!(nice_ceil(0.3), 0.5);
    }
}
