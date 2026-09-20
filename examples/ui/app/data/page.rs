use next_rust::icons;
use next_rust::prelude::*;
use next_rust::ui::*;

fn demo(title: &str, body: impl View) -> impl View {
    Card![shadow = Shadow::Sm, CardHeader![h3![title]], CardBody![body]]
}

pub fn Page(Query(q): Query<std::collections::HashMap<String, String>>) -> impl View {
    let page: u32 = q.get("page").and_then(|p| p.parse().ok()).unwrap_or(3);
    let desc = q.get("dir").is_some_and(|d| d == "desc");
    let mut rows = vec![("Ada Lovelace", 98, "1"), ("Grace Hopper", 95, "2"), ("Alan Turing", 91, "3")];
    if desc {
        rows.reverse();
    }
    Container![
        width = Width::Lg,
        Stack![
            gap = 6,
            div![h1!["Data display"], p!["Charts, tables, lists and code, all rendered on the server."]],
            demo(
                "Charts",
                Grid![
                    cols = 2,
                    BarChart![title = "Signups this week", data = [("Mon", 12.0), ("Tue", 30.0), ("Wed", 22.0), ("Thu", 41.0), ("Fri", 35.0), ("Sat", 9.0), ("Sun", 14.0)]],
                    LineChart![
                        title = "Revenue",
                        unit = "k",
                        labels = ["Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug"],
                        series = [("2025", vec![10.0, 14.0, 12.0, 18.0, 17.0, 21.0, 24.0, 23.0]), ("2026", vec![12.0, 18.0, 25.0, 24.0, 31.0, 38.0, 36.0, 45.0])]
                    ],
                    Donut![title = "Traffic sources", center_label = "visits", data = [("Direct", 4200.0), ("Search", 3100.0), ("Social", 1400.0), ("Email", 900.0)]],
                    Stack![
                        gap = 4,
                        Stack![row = true, gap = 4, align = Align::Center, Stat![label = "Active users", value = "1,204", delta = "12%", trend = Trend::Up], Sparkline![values = [3.0, 5.0, 4.0, 8.0, 7.0, 11.0, 10.0, 14.0], color = Color::Success, label = "Active users"]],
                        Meter![label = "Storage", value = 92.0, low = 60.0, high = 85.0, high_is_bad = true, value_label = "92 GB of 100 GB"],
                        Meter![label = "Password strength", value = 3.0, max = 4.0, segments = 4, low = 2.0, high = 2.5, value_label = "Good"],
                    ],
                ]
            ),
            demo(
                "Sortable, selectable table with pagination",
                Stack![
                    gap = 4,
                    Table![
                        hoverable = true,
                        header = fragment![
                            TableSelectAll![],
                            SortHeader![
                                href = if desc { "/data?dir=asc" } else { "/data?dir=desc" },
                                direction = if desc { SortDirection::Desc } else { SortDirection::Asc },
                                "Student"
                            ],
                            th![attr("scope", "col"), "Score"],
                            th![attr("scope", "col"), "Trend"],
                        ],
                        each(rows, |(name, score, id)| tr![
                            TableSelect![name = "ids", value = id, label = format!("Select {name}")],
                            td![Stack![row = true, gap = 2, align = Align::Center, Avatar![name = name, size = Size::Sm, status = AvatarStatus::Online], name]],
                            td![score.to_string()],
                            td![Sparkline![values = [70.0, 82.0, 78.0, f64::from(score)], width = 72, height = 20, area = false]],
                        ]),
                    ],
                    Pagination![page = page, total = 12, href = "/data?page={page}"],
                ]
            ),
            demo(
                "List, description list, timeline",
                Grid![
                    cols = 2,
                    List![
                        divided = true,
                        bordered = true,
                        ListItem![title = "Ada Lovelace", description = "Admin", href = "/data", start_content = Avatar![name = "Ada Lovelace", size = Size::Sm, status = AvatarStatus::Online], end_content = icons::ChevronRight().size(16)],
                        ListItem![title = "Grace Hopper", description = "Editor", href = "/data", start_content = Avatar![name = "Grace Hopper", size = Size::Sm, status = AvatarStatus::Away], end_content = icons::ChevronRight().size(16)],
                        ListItem![title = "Alan Turing", description = "Viewer", start_content = Avatar![name = "Alan Turing", size = Size::Sm, status = AvatarStatus::Offline], end_content = Chip![size = Size::Sm, "Invited"]],
                    ],
                    Timeline![
                        TimelineItem![title = "Order placed", time = "09:12", color = Color::Success, icon = icons::Check().size(12), "Payment received."],
                        TimelineItem![title = "Packed", time = "11:40", color = Color::Primary, "Left the warehouse."],
                        TimelineItem![title = "Out for delivery", time = "Tomorrow", "Arrives between 9 and 12."],
                    ],
                    DescriptionList![horizontal = true, DescriptionItem![term = "Plan", "Pro"], DescriptionItem![term = "Seats", "12"], DescriptionItem![term = "Renews", "1 Oct 2026"]],
                    ChipGroup![label = "Topics", Chip![variant = Variant::Flat, "rust"], Chip![variant = Variant::Flat, color = Color::Secondary, "web"], Chip![variant = Variant::Flat, color = Color::Success, "ssr"], Chip![variant = Variant::Flat, color = Color::Warning, "css"]],
                ]
            ),
            demo(
                "Code, callout, figure, video",
                Stack![
                    gap = 4,
                    p!["Run ", Code!["cargo run -p example-ui"], " to see this page."],
                    CodeBlock![language = "rust", title = "app/page.rs", "pub fn Page() -> impl View {\n    Hero![title = \"Hello\", actions = Button![href = \"/docs\", \"Read the docs\"]]\n}"],
                    Callout![kind = CalloutKind::Tip, "Hover the code block: the copy button appears."],
                    Callout![kind = CalloutKind::Warning, "Charts clamp negative values to zero."],
                    Grid![
                        cols = 2,
                        Figure![caption = "A figure with a caption and a fixed ratio.", ratio = "16 / 9", bordered = true,
                            div![style("display:grid;place-items:center;background:linear-gradient(135deg,#006fee,#7828c8);color:#fff;font-weight:700"), "16 : 9"]],
                        VideoEmbed![youtube = "aqz-KE-bpKQ", title = "Big Buck Bunny"],
                    ],
                ]
            ),
            demo(
                "Result",
                Grid![
                    cols = 2,
                    ResultBlock![status = ResultStatus::Success, title = "Payment received", description = "A receipt is on its way to your inbox.", Button![href = "/data", "Back to orders"]],
                    ResultBlock![code = "404", status = ResultStatus::Info, title = "Page not found", description = "The link may be old.", Button![variant = Variant::Bordered, href = "/", "Go home"]],
                ]
            ),
        ]
    ]
}
