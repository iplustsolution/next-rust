//! Tables.

use next_rust_view::{Element, Node, View};

use crate::button::{common_methods, with_parts};
use crate::{Extra, Radius, apply, attr, root, styled};

/// A data table. `columns` become the header; the children are the rows
/// (`tr![td![..], ..]`), placed in the body. Wide tables scroll sideways
/// inside their box instead of breaking the page.
///
/// ```
/// # use next_rust_ui::*; use next_rust_view::*;
/// let t = Table![columns = ["Name", "Score"], striped = true,
///     tr![td!["Ada"], td!["98"]],
///     tr![td!["Grace"], td!["95"]],
/// ];
/// let html = render_static(t);
/// assert!(html.contains(r#"<table class="nr-table nr-table-striped">"#), "{html}");
/// assert!(html.contains(r#"<th scope="col">Name</th>"#) && html.contains("<tbody><tr><td>Ada</td>"));
/// ```
#[derive(Default)]
pub struct Table {
    columns: Vec<String>,
    caption: Option<String>,
    striped: bool,
    compact: bool,
    bordered: bool,
    hoverable: bool,
    sticky_header: bool,
    radius: Option<Radius>,
    empty: Option<Node>,
    unstyled: bool,
    extra: Extra,
}

impl Table {
    pub fn new() -> Self {
        Self::default()
    }
    common_methods!();
    with_parts!();

    /// Header cells, in order.
    pub fn columns<I, S>(mut self, v: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        self.columns = v.into_iter().map(Into::into).collect();
        self
    }
    /// A visible `<caption>` (also the table's accessible name).
    pub fn caption(mut self, v: impl Into<String>) -> Self {
        self.caption = Some(v.into());
        self
    }
    pub fn striped(mut self, v: bool) -> Self {
        self.striped = v;
        self
    }
    pub fn compact(mut self, v: bool) -> Self {
        self.compact = v;
        self
    }
    /// Lines between cells.
    pub fn bordered(mut self, v: bool) -> Self {
        self.bordered = v;
        self
    }
    /// Highlight the row under the pointer.
    pub fn hoverable(mut self, v: bool) -> Self {
        self.hoverable = v;
        self
    }
    /// The header stays visible while the body scrolls.
    pub fn sticky_header(mut self, v: bool) -> Self {
        self.sticky_header = v;
        self
    }
    pub fn radius(mut self, v: Radius) -> Self {
        self.radius = Some(v);
        self
    }
    /// Shown across the table when there are no rows.
    pub fn empty(mut self, v: impl View) -> Self {
        self.empty = Some(v.into_node());
        self
    }
}

impl View for Table {
    fn into_node(self) -> Node {
        let (class, attrs, rows) = self.extra.split();
        let classes = [
            "nr-table-wrap",
            self.radius.map(Radius::class).unwrap_or(""),
            if self.sticky_header { "nr-table-sticky" } else { "" },
        ];
        let mut el = root("div", &classes, self.unstyled, class);
        apply(&mut el, attrs);
        let table_classes = [
            "nr-table",
            if self.striped { "nr-table-striped" } else { "" },
            if self.compact { "nr-table-compact" } else { "" },
            if self.bordered { "nr-table-bordered" } else { "" },
            if self.hoverable { "nr-table-hoverable" } else { "" },
        ];
        let mut table = Element::new("table")
            .with(attr("class", table_classes.iter().filter(|c| !c.is_empty()).copied().collect::<Vec<_>>().join(" ")));
        if let Some(caption) = self.caption {
            table
                .children
                .push(Element::new("caption").with(attr("class", "nr-table-caption")).with(caption).into_node());
        }
        let width = self.columns.len();
        if width > 0 {
            let mut tr = Element::new("tr");
            for column in self.columns {
                tr.children.push(Element::new("th").with(attr("scope", "col")).with(column).into_node());
            }
            table.children.push(Element::new("thead").with(tr).into_node());
        }
        let mut body = Element::new("tbody");
        let has_rows = rows.iter().any(|r| !matches!(r, Node::Empty | Node::Style(_)));
        if has_rows {
            body.children = rows;
        } else if let Some(empty) = self.empty {
            let mut cell = Element::new("td").with(attr("class", "nr-table-empty")).with(empty);
            if width > 1 {
                cell.set_attr(attr("colspan", width.to_string()));
            }
            body.children.push(Element::new("tr").with(cell).into_node());
        }
        table.children.push(body.into_node());
        el.children.push(table.into_node());
        styled(el)
    }
}
