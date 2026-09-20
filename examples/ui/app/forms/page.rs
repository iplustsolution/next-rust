use next_rust::icons;
use next_rust::prelude::*;
use next_rust::ui::*;

fn demo(title: &str, body: impl View) -> impl View {
    Card![shadow = Shadow::Sm, CardHeader![h3![title]], CardBody![body]]
}

/// Form controls beyond the basics. Everything here submits as a plain form.
pub fn Page(Query(q): Query<std::collections::HashMap<String, String>>) -> impl View {
    let query = q.get("q").cloned().unwrap_or_default();
    Container![
        width = Width::Lg,
        Stack![
            gap = 6,
            div![h1!["Forms"], p!["Native controls underneath: every one of these works with JavaScript turned off."]],
            demo(
                "Search and combobox",
                Grid![
                    cols = 2,
                    SearchInput![action = "/forms", value = query.clone(), shortcut = "⌘K", variant = FieldVariant::Bordered],
                    Combobox![
                        label = "Country",
                        name = "country",
                        options = ["Argentina", "Brazil", "Canada", "Denmark", "Germany", "India", "Japan", "Kenya", "Norway"]
                    ],
                    TagsInput![
                        label = "Topics",
                        name = "topics",
                        values = ["rust", "web"],
                        options = ["ssr", "css", "wasm"],
                        description = "Enter or comma adds a tag."
                    ],
                    NumberInput![label = "Seats", name = "seats", value = "3", min = 1.0, max = 20.0],
                    TimePicker![label = "Starts at", name = "start", value = "09:30", step = 900],
                    DateRangePicker![label = "Stay", name = "stay", start = "2026-09-21", end = "2026-09-25"],
                ]
            ),
            demo(
                "Slider, rating and segmented control",
                Grid![
                    cols = 2,
                    Slider![label = "Volume", name = "volume", value = 40.0, unit = "%"],
                    Slider![label = "Budget", name = "budget", value = 2500.0, min = 500.0, max = 5000.0, step = 100.0, color = Color::Success, size = Size::Lg],
                    Stack![gap = 3, Rating![name = "stars", label = "Your rating", value = 4.0], Rating![value = 3.5, readonly = true, size = Size::Sm]],
                    Stack![
                        gap = 3,
                        SegmentedControl![name = "view", value = "week", label = "View", [("day", "Day"), ("week", "Week"), ("month", "Month")]],
                        SegmentedControl![
                            name = "layout",
                            value = "grid",
                            label = "Layout",
                            size = Size::Sm,
                            color = Color::Secondary,
                            Segment![value = "list", start_content = icons::List().size(14), "List"],
                            Segment![value = "grid", start_content = icons::LayoutGrid().size(14), "Grid"],
                        ],
                    ],
                ]
            ),
            demo(
                "One-time code, color and upload",
                Grid![
                    cols = 2,
                    Stack![
                        gap = 4,
                        PinInput![label = "Verification code", name = "code", length = 6, description = "We sent it to your phone."],
                        ColorPicker![label = "Brand color", name = "brand", value = "#7828c8"],
                    ],
                    FileUpload![name = "photo", label = "Photo", accept = "image/*", multiple = true, hint = "PNG or JPG, up to 5 MB"],
                ]
            ),
            demo(
                "Form layout",
                form![
                    method("get"),
                    FormSection![
                        title = "Profile",
                        description = "Shown on your public page.",
                        Input![label = "Name", name = "name", variant = FieldVariant::Bordered],
                        FormField![
                            label = "Newsletter",
                            name = "newsletter",
                            description = "Any control fits in a FormField.",
                            Checkbox![name = "newsletter", "Send me product updates"]
                        ],
                    ],
                    FormSection![
                        title = "Notifications",
                        description = "Where we reach you.",
                        Switch![name = "email", checked = true, "Email"],
                        Switch![name = "push", "Push"],
                    ],
                    FormActions![
                        bordered = true,
                        Button![variant = Variant::Light, color = Color::Default, "Cancel"],
                        Button![submit = true, "Save changes"]
                    ],
                ]
            ),
        ]
    ]
}
