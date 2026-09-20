//! The component gallery (`/components`): the code of every UI component,
//! grouped by category, each with a Preview button that opens the live
//! component in a popup.

use next_rust::icons;
use next_rust::prelude::*;
use next_rust::ui::*;

/// One example: its code, with a Preview button in the header bar that opens
/// the live component in a popup.
fn demo(name: &'static str, code: &'static str, live: impl View) -> Node {
    demo_with(name, code, live, fragment![])
}

/// Like [`demo`], with `outside` rendered next to the popup instead of in it.
/// Dialogs the example opens go there: a dialog nested in a closed dialog
/// cannot be shown, and a page-wide hotkey must work with the popup closed.
fn demo_with(name: &'static str, code: &'static str, live: impl View, outside: impl View) -> Node {
    let anchor = slug(name);
    let preview = format!("preview-{anchor}");
    section![
        class("nr-gallery-demo min-w-0"),
        id(anchor.clone()),
        h3![class("mb-3 text-[1.0625rem] font-[650] text-fg"), a![href(format!("#{anchor}")), name]],
        div![
            class("min-w-0 overflow-hidden rounded-xl border border-line"),
            div![
                class("flex items-center justify-between gap-3 border-b border-line bg-soft py-1.5 pr-2 pl-4"),
                span![class("font-mono text-xs font-medium tracking-[0.04em] text-muted uppercase"), "rust"],
                // The kit's palette is its own (the demos below show it); this
                // button is site chrome, so it borrows the site accent by
                // re-pointing the colour variables the kit's styles read.
                Button![
                    size = Size::Sm,
                    variant = Variant::Flat,
                    start_content = icons::Eye().size(15),
                    on_press = Press::open_modal(preview.clone()),
                    class("[--c:var(--color-accent)] [--c-text:var(--color-accent-fg)]"),
                    aria("haspopup", "dialog"),
                    aria("label", format!("Preview {name}")),
                    "Preview"
                ],
            ],
            // The kit's own block, fed the HTML our highlighter produced — it
            // takes markup as its children, so the copy button survives and the
            // tokens get the same `syn-*` colours as the docs and landing page.
            CodeBlock![
                class("rounded-none border-0 bg-code text-fg-soft"),
                class(crate::ui::SYNTAX),
                class("[&_pre]:px-[1.125rem] [&_pre]:py-4 [&_pre]:text-[0.8125rem]/[1.7]"),
                raw_html(crate::highlight::highlight("rust", code)),
            ],
        ],
        // Escape, the close button and a click on the backdrop close it; long
        // examples scroll inside. Select, DatePicker, menus and toasts open
        // in the top layer, above the popup, so its scrolling never clips them.
        Modal![
            id = preview,
            title = name,
            size = Width::Xxl,
            scroll_inside = true,
            class("max-w-[56rem]"),
            ModalBody![class("min-h-[12rem] overflow-x-auto"), div![class("min-w-0 pt-8 pb-6"), live]],
        ],
        outside,
    ]
    .into_node()
}

fn slug(name: &str) -> String {
    name.to_lowercase().chars().map(|c| if c.is_ascii_alphanumeric() { c } else { '-' }).collect()
}

fn category(id_: &'static str, title: &'static str, intro: &'static str, demos: Vec<Node>) -> Node {
    section![
        class("mt-16 first:mt-10"),
        id(id_),
        h2![class("text-[1.625rem]/[1.25] font-[650] tracking-[-0.02em] text-fg"), title],
        p![class("mt-2 max-w-[44rem] text-fg-soft"), intro],
        div![class("mt-7 grid min-w-0 gap-10"), demos],
    ]
    .into_node()
}

const CATEGORIES: &[(&str, &str)] = &[
    ("actions", "Actions"),
    ("forms", "Forms"),
    ("overlays", "Overlays and menus"),
    ("navigation", "Navigation"),
    ("data", "Data display"),
    ("charts", "Charts"),
    ("feedback", "Feedback"),
    ("sections", "Page sections"),
    ("layout", "Layout"),
];

pub fn page() -> impl View {
    main![
        id("content"),
        class("mx-auto max-w-[72rem] px-6 pt-12 pb-24 max-[640px]:px-4"),
        p![class("text-sm font-semibold text-accent-fg"), "UI kit"],
        h1![
            class("mt-2 text-[2.5rem]/[1.1] font-[700] tracking-[-0.03em] max-[640px]:text-[2rem]"),
            "Component gallery"
        ],
        p![
            class("mt-4 max-w-[44rem] text-lg/[1.6] text-muted"),
            "Every component of ",
            code!["next_rust::ui"],
            ", with the code that makes it. Press Preview on any example to open the live, working component in a popup. Most need no JavaScript at all; the rest fall back to a native control without it. Properties are described in the ",
            a![class("text-accent-fg underline underline-offset-3"), href("/docs/components"), "components guide"],
            ".",
        ],
        nav![
            aria("label", "Categories"),
            class("mt-8"),
            ChipGroup![each(CATEGORIES.iter(), |(id_, title)| a![
                href(format!("#{id_}")),
                Chip![variant = Variant::Flat, color = Color::Default, *title]
            ])],
        ],
        actions(),
        forms(),
        overlays(),
        navigation(),
        data(),
        charts(),
        feedback(),
        sections(),
        layout(),
        Toaster![],
    ]
}

fn actions() -> Node {
    category(
        "actions",
        "Actions",
        "Buttons and what pressing them does.",
        vec![
            demo(
                "Button",
                r#"Button!["Save"]
Button![color = Color::Danger, variant = Variant::Bordered, "Delete"]
Button![variant = Variant::Flat, start_content = icons::Plus().size(16), "New"]
Button![loading = true, "Saving"]
Button![href = "/docs", variant = Variant::Light, "A link"]
Button![on_press = Press::toast("Pressed"), variant = Variant::Shadow, "Toast"]"#,
                Stack![
                    row = true,
                    wrap = true,
                    gap = 3,
                    align = Align::Center,
                    Button!["Save"],
                    Button![color = Color::Danger, variant = Variant::Bordered, "Delete"],
                    Button![variant = Variant::Flat, start_content = icons::Plus().size(16), "New"],
                    Button![loading = true, "Saving"],
                    Button![href = "/docs", variant = Variant::Light, "A link"],
                    Button![on_press = Press::toast("Pressed"), variant = Variant::Shadow, "Toast"],
                ],
            ),
            demo(
                "ButtonGroup",
                r#"ButtonGroup![
    Button![variant = Variant::Bordered, "Day"],
    Button![variant = Variant::Bordered, "Week"],
    Button![variant = Variant::Bordered, "Month"],
]"#,
                ButtonGroup![
                    Button![variant = Variant::Bordered, "Day"],
                    Button![variant = Variant::Bordered, "Week"],
                    Button![variant = Variant::Bordered, "Month"],
                ],
            ),
        ],
    )
}

fn forms() -> Node {
    category(
        "forms",
        "Forms",
        "Real form controls underneath: they submit, validate and autofill like the browser's own.",
        vec![
            demo(
                "Input, PasswordInput, Textarea",
                r#"Input![label = "Email", name = "email", kind = "email", start_content = icons::Mail().size(18)]
PasswordInput![label = "Password", name = "password"]
Textarea![label = "Message", rows = 3]"#,
                Grid![
                    cols = 2,
                    Input![label = "Email", name = "email", kind = "email", start_content = icons::Mail().size(18)],
                    PasswordInput![label = "Password", name = "password"],
                    Textarea![label = "Message", rows = 3],
                ],
            ),
            demo(
                "Select, DatePicker",
                r#"Select![label = "Plan", name = "plan", placeholder = "Choose", [("free", "Free"), ("pro", "Pro")]]
        DatePicker![label = "Day", name = "day", first_day_of_week = 1]"#,
                Grid![
                    cols = 2,
                    Select![label = "Plan", name = "plan", placeholder = "Choose", [("free", "Free"), ("pro", "Pro")]],
                    DatePicker![label = "Day", name = "day", first_day_of_week = 1],
                ],
            ),
            demo(
                "Checkbox, Switch, RadioGroup",
                r#"Checkbox![name = "terms", checked = true, "I agree"]
Switch![name = "wifi", checked = true, "Wi-Fi"]
RadioGroup![name = "plan", value = "pro", horizontal = true, Radio![value = "free", "Free"], Radio![value = "pro", "Pro"]]"#,
                Stack![
                    row = true,
                    wrap = true,
                    gap = 6,
                    align = Align::Center,
                    Checkbox![name = "terms", checked = true, "I agree"],
                    Switch![name = "wifi", checked = true, "Wi-Fi"],
                    RadioGroup![
                        name = "plan",
                        value = "pro",
                        horizontal = true,
                        Radio![value = "free", "Free"],
                        Radio![value = "pro", "Pro"]
                    ],
                ],
            ),
            demo(
                "SearchInput",
                r#"SearchInput![action = "/docs/search", shortcut = "⌘K", variant = FieldVariant::Bordered]   // a GET form, name="q""#,
                SearchInput![action = "/docs/search", shortcut = "⌘K", variant = FieldVariant::Bordered],
            ),
            demo(
                "Combobox, TagsInput",
                r#"Combobox![label = "Country", name = "country", options = ["Brazil", "Germany", "India", "Japan"]]
        TagsInput![label = "Topics", name = "topics", values = ["rust", "web"], max = 5]   // submits "rust, web""#,
                Grid![
                    cols = 2,
                    Combobox![label = "Country", name = "country", options = ["Brazil", "Germany", "India", "Japan"]],
                    TagsInput![label = "Topics", name = "topics", values = ["rust", "web"], max = 5],
                ],
            ),
            demo(
                "NumberInput, TimePicker, DateRangePicker",
                r#"NumberInput![label = "Seats", name = "seats", value = "3", min = 1.0, max = 10.0]
TimePicker![label = "Starts at", name = "start", value = "09:30", step = 900]
DateRangePicker![label = "Stay", name = "stay", start = "2026-09-21", end = "2026-09-25"]"#,
                Grid![
                    cols = 2,
                    NumberInput![label = "Seats", name = "seats", value = "3", min = 1.0, max = 10.0],
                    TimePicker![label = "Starts at", name = "start", value = "09:30", step = 900],
                    DateRangePicker![label = "Stay", name = "stay", start = "2026-09-21", end = "2026-09-25"],
                ],
            ),
            demo(
                "Slider",
                r#"Slider![label = "Volume", name = "volume", value = 40.0, unit = "%"]"#,
                Slider![label = "Volume", name = "volume", value = 40.0, unit = "%"],
            ),
            demo(
                "Rating, SegmentedControl",
                r#"Rating![name = "stars", label = "Your rating", value = 4.0]
Rating![value = 3.5, readonly = true, size = Size::Sm]
SegmentedControl![name = "view", value = "week", label = "View", [("day", "Day"), ("week", "Week"), ("month", "Month")]]"#,
                Stack![
                    row = true,
                    wrap = true,
                    gap = 8,
                    align = Align::End,
                    Rating![name = "stars", label = "Your rating", value = 4.0],
                    Rating![value = 3.5, readonly = true, size = Size::Sm],
                    SegmentedControl![
                        name = "view",
                        value = "week",
                        label = "View",
                        [("day", "Day"), ("week", "Week"), ("month", "Month")]
                    ],
                ],
            ),
            demo(
                "PinInput, ColorPicker",
                r##"PinInput![label = "Verification code", name = "code", length = 6]
ColorPicker![label = "Brand color", name = "brand", value = "#7828c8"]"##,
                Stack![
                    row = true,
                    wrap = true,
                    gap = 8,
                    align = Align::End,
                    PinInput![label = "Verification code", name = "code", length = 6],
                    ColorPicker![label = "Brand color", name = "brand", value = "#7828c8"],
                ],
            ),
            demo(
                "FileUpload",
                r#"FileUpload![name = "photo", label = "Photo", accept = "image/*", hint = "PNG or JPG, up to 5 MB"]"#,
                FileUpload![name = "photo", label = "Photo", accept = "image/*", hint = "PNG or JPG, up to 5 MB"],
            ),
            demo(
                "FormSection, FormField, FormActions",
                r#"FormSection![title = "Profile", description = "Shown on your public page.",
    Input![label = "Name", name = "name"],
    FormField![label = "Newsletter", name = "newsletter", Checkbox![name = "newsletter", "Send me updates"]],
]
FormActions![bordered = true, Button![variant = Variant::Light, color = Color::Default, "Cancel"], Button![submit = true, "Save"]]"#,
                div![
                    FormSection![
                        title = "Profile",
                        description = "Shown on your public page.",
                        Input![label = "Name", name = "name"],
                        FormField![
                            label = "Newsletter",
                            name = "newsletter",
                            Checkbox![name = "newsletter", "Send me updates"]
                        ],
                    ],
                    FormActions![
                        bordered = true,
                        Button![variant = Variant::Light, color = Color::Default, "Cancel"],
                        Button![submit = true, "Save"]
                    ],
                ],
            ),
        ],
    )
}

fn overlays() -> Node {
    category(
        "overlays",
        "Overlays and menus",
        "Native popovers and dialogs: they open, close and return focus on their own. The script adds arrow keys, filtering and toasts.",
        vec![
            demo(
                "Dropdown, Popover, ContextMenu",
                r#"Dropdown![trigger = Button![variant = Variant::Bordered, "Options"],
    DropdownSection![title = "Project",
        DropdownItem![href = "/docs", start_content = icons::Pencil().size(16), shortcut = "⌘E", "Edit"],
        DropdownItem![on_press = Press::toast("Duplicated"), description = "Make a copy", "Duplicate"]],
    DropdownSection![DropdownItem![color = Color::Danger, on_press = Press::open_modal("gallery-confirm"), "Delete"]],
]
Popover![trigger = Button![variant = Variant::Flat, "Share"], title = "Share this page", p!["Anyone with the link can view."]]
ContextMenu![target = Card![CardBody!["Right click me"]], DropdownItem![on_press = Press::toast("Renamed"), "Rename"]]"#,
                Stack![
                    row = true,
                    wrap = true,
                    gap = 3,
                    align = Align::Center,
                    Dropdown![
                        trigger = Button![variant = Variant::Bordered, "Options"],
                        DropdownSection![
                            title = "Project",
                            DropdownItem![
                                href = "/docs",
                                start_content = icons::Pencil().size(16),
                                shortcut = "⌘E",
                                "Edit"
                            ],
                            DropdownItem![
                                on_press = Press::toast("Duplicated"),
                                start_content = icons::Copy().size(16),
                                description = "Make a copy",
                                "Duplicate"
                            ],
                        ],
                        DropdownSection![DropdownItem![
                            color = Color::Danger,
                            start_content = icons::Trash2().size(16),
                            on_press = Press::open_modal("gallery-confirm"),
                            "Delete"
                        ]],
                    ],
                    Popover![
                        trigger = Button![variant = Variant::Flat, "Share"],
                        title = "Share this page",
                        p!["Anyone with the link can view."]
                    ],
                    ContextMenu![
                        target = Card![bordered = true, shadow = Shadow::None, CardBody!["Right click me"]],
                        DropdownItem![on_press = Press::toast("Renamed"), "Rename"],
                    ],
                ],
            ),
            demo_with(
                "Modal, Drawer, ConfirmDialog",
                r#"Button![on_press = Press::open_modal("gallery-modal"), "Modal"]
Modal![id = "gallery-modal", title = "A dialog", ModalBody![p!["Escape, the close button and the backdrop close it."]]]
Drawer![id = "gallery-drawer", title = "Filters", side = DrawerSide::Right, ModalBody![..], ModalFooter![..]]
ConfirmDialog![id = "gallery-confirm", title = "Delete this project?", confirm_label = "Delete",
    on_confirm = Press::toast("Deleted"), "This cannot be undone."]"#,
                Stack![
                    row = true,
                    wrap = true,
                    gap = 3,
                    Button![on_press = Press::open_modal("gallery-modal"), "Modal"],
                    Button![variant = Variant::Bordered, on_press = Press::open_modal("gallery-drawer"), "Drawer"],
                    Button![
                        color = Color::Danger,
                        variant = Variant::Flat,
                        on_press = Press::open_modal("gallery-confirm"),
                        "Confirm…"
                    ],
                ],
                fragment![
                    Modal![
                        id = "gallery-modal",
                        title = "A dialog",
                        ModalBody![p!["Escape, the close button and a click on the backdrop close it."]],
                        ModalFooter![Button![on_press = Press::close_modal(), "Done"]],
                    ],
                    Drawer![
                        id = "gallery-drawer",
                        title = "Filters",
                        ModalBody![Stack![
                            gap = 4,
                            Slider![label = "Price", value = 60.0],
                            Checkbox![checked = true, "In stock only"]
                        ]],
                        ModalFooter![Button![on_press = Press::close_modal(), "Apply"]],
                    ],
                    ConfirmDialog![
                        id = "gallery-confirm",
                        title = "Delete this project?",
                        confirm_label = "Delete",
                        on_confirm = Press::toast("Deleted"),
                        "This cannot be undone."
                    ],
                ],
            ),
            demo_with(
                "CommandPalette",
                r#"CommandPalette![id = "gallery-commands", hotkey = "j",          // ⌘J / Ctrl J here; "k" by default
    CommandGroup![title = "Pages", CommandItem![href = "/docs", keywords = "guide manual", "Documentation"]],
    CommandGroup![title = "Actions", CommandItem![on_press = Press::toast("Hello"), "Show a toast"]],
]
Button![on_press = Press::open_modal("gallery-commands"), end_content = Kbd!["⌘", "J"], "Commands"]"#,
                Button![
                    variant = Variant::Faded,
                    color = Color::Default,
                    on_press = Press::open_modal("gallery-commands"),
                    start_content = icons::Search().size(16),
                    end_content = Kbd!["⌘", "J"],
                    "Commands"
                ],
                fragment![CommandPalette![
                    id = "gallery-commands",
                    hotkey = "j",
                    CommandGroup![
                        title = "Pages",
                        CommandItem![href = "/docs", keywords = "guide manual", "Documentation"],
                        CommandItem![href = "/docs/components", keywords = "ui kit", "Components guide"],
                        CommandItem![href = "/", "Home"],
                    ],
                    CommandGroup![title = "Actions", CommandItem![on_press = Press::toast("Hello"), "Show a toast"]],
                ]],
            ),
            demo(
                "Toaster, Toast",
                r#"Toaster![flash.map(|m| Toast![color = Color::Success, m])]      // once, in the layout
Button![on_press = Press::toast("Saved your changes"), "Toast"]
// From your scripts: nextRust.toast({ title: "Saved", message: "All good", color: "success" })"#,
                Button![color = Color::Success, on_press = Press::toast("Saved your changes"), "Toast"],
            ),
            demo(
                "Tooltip",
                r#"Tooltip![content = "Copy the link", Button![icon_only = true, aria("label", "Copy"), icons::Link().size(18)]]"#,
                Tooltip![
                    content = "Copy the link",
                    Button![icon_only = true, variant = Variant::Flat, aria("label", "Copy"), icons::Link().size(18)]
                ],
            ),
        ],
    )
}

fn navigation() -> Node {
    category(
        "navigation",
        "Navigation",
        "Links the server renders: nothing here needs a script except tab panels.",
        vec![
            demo(
                "Tabs",
                r#"Tabs![selected = "b", Tab![key = "a", title = "Overview", p!["…"]], Tab![key = "b", title = "Grades", p!["…"]]]"#,
                Tabs![
                    selected = "b",
                    Tab![key = "a", title = "Overview", p!["The overview panel."]],
                    Tab![key = "b", title = "Grades", p!["The grades panel."]],
                ],
            ),
            demo(
                "Breadcrumbs, Steps",
                r#"Breadcrumbs![BreadcrumbItem![href = "/", "Home"], BreadcrumbItem![href = "/docs", "Docs"], BreadcrumbItem!["Components"]]
Steps![current = 1, Step![title = "Account"], Step![title = "Plan"], Step![title = "Done"]]"#,
                Stack![
                    gap = 5,
                    Breadcrumbs![
                        BreadcrumbItem![href = "/", "Home"],
                        BreadcrumbItem![href = "/docs", "Docs"],
                        BreadcrumbItem!["Components"]
                    ],
                    Steps![current = 1, Step![title = "Account"], Step![title = "Plan"], Step![title = "Done"]],
                ],
            ),
            demo(
                "Pagination",
                r##"Pagination![page = 5, total = 20, href = "/components?page={page}#pagination"]"##,
                Pagination![page = 5, total = 20, href = "/components?page={page}#pagination"],
            ),
            demo(
                "Accordion, Faq",
                r#"Faq![structured_data = true, [("Do these need JavaScript?", "No."), ("Dark mode?", "Follows the system.")]]"#,
                Faq![[
                    ("Do these need JavaScript?", "No: they are <details> elements."),
                    ("Dark mode?", "Follows the system, or class=\"dark\".")
                ]],
            ),
        ],
    )
}

fn data() -> Node {
    category(
        "data",
        "Data display",
        "Tables, lists and text blocks.",
        vec![
            demo(
                "Table, SortHeader, TableSelect",
                r#"Table![hoverable = true,
    header = fragment![TableSelectAll![], SortHeader![href = "?dir=desc", direction = SortDirection::Asc, "Student"], th!["Score"]],
    tr![TableSelect![name = "ids", value = "1"], td!["Ada Lovelace"], td!["98"]],
    tr![TableSelect![name = "ids", value = "2"], td!["Grace Hopper"], td!["95"]],
]"#,
                Table![
                    hoverable = true,
                    header = fragment![
                        TableSelectAll![],
                        SortHeader![
                            href = "/components?dir=desc#table--sortheader--tableselect",
                            direction = SortDirection::Asc,
                            "Student"
                        ],
                        th![attr("scope", "col"), "Score"]
                    ],
                    tr![TableSelect![name = "ids", value = "1"], td!["Ada Lovelace"], td!["98"]],
                    tr![TableSelect![name = "ids", value = "2"], td!["Grace Hopper"], td!["95"]],
                ],
            ),
            demo(
                "List, Avatar",
                r#"List![divided = true, bordered = true,
    ListItem![title = "Ada Lovelace", description = "Admin", href = "/docs",
        start_content = Avatar![name = "Ada Lovelace", size = Size::Sm, status = AvatarStatus::Online]],
    ListItem![title = "Grace Hopper", description = "Editor", end_content = Chip![size = Size::Sm, "Invited"]],
]"#,
                List![
                    divided = true,
                    bordered = true,
                    ListItem![
                        title = "Ada Lovelace",
                        description = "Admin",
                        href = "/docs",
                        start_content = Avatar![name = "Ada Lovelace", size = Size::Sm, status = AvatarStatus::Online],
                        end_content = icons::ChevronRight().size(16)
                    ],
                    ListItem![
                        title = "Grace Hopper",
                        description = "Editor",
                        start_content = Avatar![name = "Grace Hopper", size = Size::Sm, status = AvatarStatus::Away],
                        end_content = Chip![size = Size::Sm, "Invited"]
                    ],
                ],
            ),
            demo(
                "DescriptionList, ChipGroup, Badge, Kbd",
                r#"DescriptionList![cols = 3, DescriptionItem![term = "Plan", "Pro"], DescriptionItem![term = "Seats", "12"], DescriptionItem![term = "Renews", "1 Oct"]]
ChipGroup![label = "Topics", Chip![variant = Variant::Flat, "rust"], Chip![variant = Variant::Flat, color = Color::Success, dot = true, "online"]]
Badge![content = "3", Avatar![name = "Ada Lovelace"]]   Kbd!["⌘", "K"]"#,
                Stack![
                    gap = 5,
                    DescriptionList![
                        cols = 3,
                        DescriptionItem![term = "Plan", "Pro"],
                        DescriptionItem![term = "Seats", "12"],
                        DescriptionItem![term = "Renews", "1 Oct"]
                    ],
                    Stack![
                        row = true,
                        wrap = true,
                        gap = 5,
                        align = Align::Center,
                        ChipGroup![
                            label = "Topics",
                            Chip![variant = Variant::Flat, "rust"],
                            Chip![variant = Variant::Flat, color = Color::Success, dot = true, "online"]
                        ],
                        Badge![content = "3", Avatar![name = "Ada Lovelace"]],
                        Kbd!["⌘", "K"],
                    ],
                ],
            ),
            demo(
                "Timeline",
                r#"Timeline![
    TimelineItem![title = "Order placed", time = "09:12", color = Color::Success, icon = icons::Check().size(12), "Payment received."],
    TimelineItem![title = "Packed", time = "11:40", color = Color::Primary, "Left the warehouse."],
    TimelineItem![title = "Out for delivery", time = "Tomorrow"],
]"#,
                Timeline![
                    TimelineItem![
                        title = "Order placed",
                        time = "09:12",
                        color = Color::Success,
                        icon = icons::Check().size(12),
                        "Payment received."
                    ],
                    TimelineItem![title = "Packed", time = "11:40", color = Color::Primary, "Left the warehouse."],
                    TimelineItem![title = "Out for delivery", time = "Tomorrow"],
                ],
            ),
            demo(
                "Code, CodeBlock, Callout",
                r#"p!["Run ", Code!["cargo run"], "."]
CodeBlock![language = "rust", title = "src/main.rs", "fn main() {}"]
Callout![kind = CalloutKind::Tip, "Hover a code block for its copy button."]"#,
                Stack![
                    gap = 4,
                    p!["Run ", Code!["cargo run"], "."],
                    CodeBlock![
                        language = "rust",
                        title = "src/main.rs",
                        "fn main() {\n    next_rust::run(routes());\n}"
                    ],
                    Callout![kind = CalloutKind::Tip, "Hover a code block for its copy button."],
                ],
            ),
            demo(
                "Figure, VideoEmbed, AspectRatio",
                r#"Figure![src = "/logo.svg", alt = "The Next Rust logo", caption = "A figure with a caption.", ratio = "16 / 9"]
VideoEmbed![youtube = "aqz-KE-bpKQ", title = "Big Buck Bunny"]      // only the poster loads until play"#,
                Grid![
                    cols = 2,
                    Figure![
                        src = "/logo.svg",
                        alt = "The Next Rust logo",
                        caption = "A figure with a caption.",
                        ratio = "16 / 9",
                        bordered = true
                    ],
                    VideoEmbed![youtube = "aqz-KE-bpKQ", title = "Big Buck Bunny"],
                ],
            ),
            demo(
                "Card, Stat",
                r#"Card![CardHeader![h3!["Pro plan"]], CardBody![p!["Everything you need."]], CardFooter![Button![size = Size::Sm, "Upgrade"]]]
Stat![label = "Students", value = "1,204", delta = "12%", trend = Trend::Up]"#,
                Grid![
                    cols = 2,
                    Card![
                        CardHeader![h3!["Pro plan"]],
                        CardBody![p!["Everything you need."]],
                        CardFooter![Button![size = Size::Sm, "Upgrade"]]
                    ],
                    Stat![
                        label = "Students",
                        value = "1,204",
                        delta = "12%",
                        trend = Trend::Up,
                        icon = icons::Users().size(20)
                    ],
                ],
            ),
        ],
    )
}

fn charts() -> Node {
    category(
        "charts",
        "Charts",
        "Rendered on the server as HTML and SVG. Hover a bar or a point for its value; no script involved.",
        vec![
            demo(
                "BarChart, LineChart",
                r#"BarChart![title = "Signups", data = [("Mon", 12.0), ("Tue", 30.0), ("Wed", 22.0), ("Thu", 41.0), ("Fri", 35.0)]]
LineChart![title = "Revenue", unit = "k", labels = ["Jan", "Feb", "Mar", "Apr", "May", "Jun"],
    series = [("2025", vec![10.0, 14.0, 12.0, 18.0, 17.0, 21.0]), ("2026", vec![12.0, 18.0, 25.0, 24.0, 31.0, 38.0])]]"#,
                Grid![
                    cols = 2,
                    BarChart![
                        title = "Signups",
                        data = [("Mon", 12.0), ("Tue", 30.0), ("Wed", 22.0), ("Thu", 41.0), ("Fri", 35.0)]
                    ],
                    LineChart![
                        title = "Revenue",
                        unit = "k",
                        labels = ["Jan", "Feb", "Mar", "Apr", "May", "Jun"],
                        series = [
                            ("2025", vec![10.0, 14.0, 12.0, 18.0, 17.0, 21.0]),
                            ("2026", vec![12.0, 18.0, 25.0, 24.0, 31.0, 38.0])
                        ]
                    ],
                ],
            ),
            demo(
                "Donut, Sparkline, Meter",
                r#"Donut![title = "Traffic", center_label = "visits", data = [("Direct", 4200.0), ("Search", 3100.0), ("Social", 1400.0)]]
Sparkline![values = [3.0, 5.0, 4.0, 8.0, 7.0, 11.0], color = Color::Success, label = "Signups"]
Meter![label = "Storage", value = 92.0, low = 60.0, high = 85.0, high_is_bad = true, value_label = "92 GB of 100 GB"]
Meter![label = "Password strength", value = 3.0, max = 4.0, segments = 4, color = Color::Success, value_label = "Good"]"#,
                Grid![
                    cols = 2,
                    Donut![
                        title = "Traffic",
                        center_label = "visits",
                        data = [("Direct", 4200.0), ("Search", 3100.0), ("Social", 1400.0)]
                    ],
                    Stack![
                        gap = 5,
                        Sparkline![values = [3.0, 5.0, 4.0, 8.0, 7.0, 11.0], color = Color::Success, label = "Signups"],
                        Meter![
                            label = "Storage",
                            value = 92.0,
                            low = 60.0,
                            high = 85.0,
                            high_is_bad = true,
                            value_label = "92 GB of 100 GB"
                        ],
                        Meter![
                            label = "Password strength",
                            value = 3.0,
                            max = 4.0,
                            segments = 4,
                            color = Color::Success,
                            value_label = "Good"
                        ],
                    ],
                ],
            ),
        ],
    )
}

fn feedback() -> Node {
    category(
        "feedback",
        "Feedback",
        "Telling people what is happening.",
        vec![
            demo(
                "Alert, Banner",
                r#"Alert![color = Color::Success, title = "Saved", closable = true, "Your changes are live."]
Banner![soft = true, href = "/docs", link_label = "Read more", "Version 2 is out."]   // dismissed with CSS alone"#,
                Stack![
                    gap = 4,
                    Alert![color = Color::Success, title = "Saved", closable = true, "Your changes are live."],
                    Banner![soft = true, href = "/docs", link_label = "Read more", "Version 2 is out."],
                ],
            ),
            demo(
                "Progress, CircularProgress, Spinner, Skeleton",
                r#"Progress![label = "Uploading", value = 62.0, show_value = true]
CircularProgress![value = 75.0, show_value = true]   Spinner![label = "Loading…"]   Skeleton![lines = 3]"#,
                Grid![
                    cols = 2,
                    Stack![
                        gap = 4,
                        Progress![label = "Uploading", value = 62.0, show_value = true],
                        Skeleton![lines = 3]
                    ],
                    Stack![
                        row = true,
                        gap = 6,
                        align = Align::Center,
                        CircularProgress![value = 75.0, show_value = true],
                        Spinner![label = "Loading…"]
                    ],
                ],
            ),
            demo(
                "LoadingOverlay",
                r#"LoadingOverlay![visible = true, blur = true, label = "Saving…", Skeleton![lines = 4]]"#,
                LoadingOverlay![visible = true, blur = true, label = "Saving…", Skeleton![lines = 4]],
            ),
            demo(
                "ResultBlock, EmptyState",
                r#"ResultBlock![status = ResultStatus::Success, title = "Payment received", description = "A receipt is on its way.", Button!["Back to orders"]]
EmptyState![title = "No classes yet", description = "Create one to get started.", Button![size = Size::Sm, "New class"]]"#,
                Grid![
                    cols = 2,
                    ResultBlock![
                        status = ResultStatus::Success,
                        title = "Payment received",
                        description = "A receipt is on its way.",
                        Button!["Back to orders"]
                    ],
                    EmptyState![
                        title = "No classes yet",
                        description = "Create one to get started.",
                        Button![size = Size::Sm, "New class"]
                    ],
                ],
            ),
        ],
    )
}

fn sections() -> Node {
    category(
        "sections",
        "Page sections",
        "The blocks a marketing site is made of. HTML and CSS only.",
        vec![
            demo(
                "Hero",
                r#"Hero![eyebrow = "New", title = "Ship faster", subtitle = "One binary, no bundler.", glow = true,
    actions = fragment![Button![size = Size::Lg, "Get started"], Button![size = Size::Lg, variant = Variant::Bordered, color = Color::Default, "Docs"]]]"#,
                Hero![
                    eyebrow = "New",
                    title = "Ship faster",
                    subtitle = "One binary, no bundler.",
                    glow = true,
                    actions = fragment![
                        Button![size = Size::Lg, "Get started"],
                        Button![size = Size::Lg, variant = Variant::Bordered, color = Color::Default, "Docs"]
                    ],
                ],
            ),
            demo(
                "Section, FeatureGrid, LogoCloud",
                r#"Section![eyebrow = "Features", title = "Everything included", compact = true,
    FeatureGrid![cols = 3,
        Feature![icon = icons::Zap().size(20), title = "Fast", card = true, "Renders in microseconds."],
        Feature![icon = icons::Palette().size(20), color = Color::Secondary, title = "Themeable", card = true, "CSS variables."],
        Feature![icon = icons::Accessibility().size(20), color = Color::Success, title = "Accessible", card = true, "Built in."]],
    LogoCloud![title = "Trusted by", span!["Acme"], span!["Globex"], span!["Initech"]]]"#,
                Section![
                    eyebrow = "Features",
                    title = "Everything included",
                    compact = true,
                    FeatureGrid![
                        cols = 3,
                        Feature![icon = icons::Zap().size(20), title = "Fast", card = true, "Renders in microseconds."],
                        Feature![
                            icon = icons::Palette().size(20),
                            color = Color::Secondary,
                            title = "Themeable",
                            card = true,
                            "CSS variables."
                        ],
                        Feature![
                            icon = icons::Accessibility().size(20),
                            color = Color::Success,
                            title = "Accessible",
                            card = true,
                            "Built in."
                        ],
                    ],
                    div![
                        class("mt-10"),
                        LogoCloud![title = "Trusted by", span!["Acme"], span!["Globex"], span!["Initech"]]
                    ],
                ],
            ),
            demo(
                "PricingTable",
                r#"PricingTable![
    PricingCard![name = "Hobby", price = "Free", cta = Button![full_width = true, variant = Variant::Bordered, "Start"], PricingFeature!["1 project"], PricingFeature![included = false, "SSO"]],
    PricingCard![name = "Pro", price = "$19", period = "/month", featured = true, badge = "Popular", cta = Button![full_width = true, "Choose Pro"], PricingFeature!["Unlimited projects"], PricingFeature!["SSO"]],
]"#,
                PricingTable![
                    PricingCard![
                        name = "Hobby",
                        price = "Free",
                        description = "For side projects.",
                        cta = Button![full_width = true, variant = Variant::Bordered, color = Color::Default, "Start"],
                        PricingFeature!["1 project"],
                        PricingFeature![included = false, "SSO"]
                    ],
                    PricingCard![
                        name = "Pro",
                        price = "$19",
                        period = "/month",
                        description = "For teams.",
                        featured = true,
                        badge = "Popular",
                        cta = Button![full_width = true, "Choose Pro"],
                        PricingFeature!["Unlimited projects"],
                        PricingFeature!["SSO"]
                    ],
                ],
            ),
            demo(
                "Carousel, Testimonial",
                r#"Carousel![label = "Testimonials", per_view = 2,
    CarouselItem![Testimonial![name = "Ada Lovelace", role = "CTO", rating = 5.0, "We deleted our front-end build."]],
    CarouselItem![Testimonial![name = "Grace Hopper", role = "Rear Admiral", rating = 5.0, "It compiles, it ships."]],
    CarouselItem![Testimonial![name = "Alan Turing", role = "Researcher", rating = 4.5, "Pages load before I finish clicking."]],
]"#,
                Carousel![
                    label = "Testimonials",
                    per_view = 2,
                    CarouselItem![Testimonial![
                        name = "Ada Lovelace",
                        role = "CTO",
                        rating = 5.0,
                        "We deleted our front-end build."
                    ]],
                    CarouselItem![Testimonial![
                        name = "Grace Hopper",
                        role = "Rear Admiral",
                        rating = 5.0,
                        "It compiles, it ships."
                    ]],
                    CarouselItem![Testimonial![
                        name = "Alan Turing",
                        role = "Researcher",
                        rating = 4.5,
                        "Pages load before I finish clicking."
                    ]],
                ],
            ),
            demo(
                "Bento, Marquee",
                r#"Bento![cols = 3,
    BentoItem![col_span = 2, title = "Wide tile", description = "Spans two columns."],
    BentoItem![title = "Small tile"],
    BentoItem![col_span = 3, title = "Marquee", Marquee![speed = 25, span!["Fast"], span!["Typed"], span!["Accessible"], span!["Themeable"]]],
]"#,
                Bento![
                    cols = 3,
                    BentoItem![col_span = 2, title = "Wide tile", description = "Spans two columns."],
                    BentoItem![title = "Small tile", description = "One column."],
                    BentoItem![
                        col_span = 3,
                        title = "Marquee",
                        Marquee![speed = 25, span!["Fast"], span!["Typed"], span!["Accessible"], span!["Themeable"]]
                    ],
                ],
            ),
            demo(
                "Cta, Footer",
                r#"Cta![title = "Ready to build?", description = "One crate, one binary.", split = true, Button![color = Color::Default, "Get started"]]
Footer![brand = strong!["Acme"], description = "Tools for teams.", bottom = "© 2026 Acme",
    FooterColumn![title = "Product", FooterLink![href = "/docs", "Docs"], FooterLink![href = "https://lucide.dev", external = true, "Icons"]]]"#,
                Stack![
                    gap = 6,
                    Cta![
                        title = "Ready to build?",
                        description = "One crate, one binary.",
                        split = true,
                        Button![color = Color::Default, "Get started"]
                    ],
                    Footer![
                        brand = strong!["Acme"],
                        description = "Tools for teams.",
                        bottom = "© 2026 Acme",
                        FooterColumn![
                            title = "Product",
                            FooterLink![href = "/docs", "Docs"],
                            FooterLink![href = "https://lucide.dev", external = true, "Icons"]
                        ],
                        FooterColumn![title = "Company", FooterLink![href = "/", "About"]],
                    ],
                ],
            ),
        ],
    )
}

fn layout() -> Node {
    category(
        "layout",
        "Layout",
        "Containers and helpers. AppShell, Navbar and Sidebar make a whole app frame (see the components guide).",
        vec![
            demo(
                "Stack, Grid, Spacer, Divider",
                r#"Stack![row = true, gap = 3, align = Align::Center, Chip!["Left"], Spacer![], Chip!["Right"]]
Divider![]
Grid![cols = 3, Card![CardBody!["1"]], Card![CardBody!["2"]], Card![CardBody!["3"]]]"#,
                Stack![
                    gap = 4,
                    Stack![row = true, gap = 3, align = Align::Center, Chip!["Left"], Spacer![], Chip!["Right"]],
                    Divider![],
                    Grid![cols = 3, Card![CardBody!["1"]], Card![CardBody!["2"]], Card![CardBody!["3"]]],
                ],
            ),
            demo(
                "Show, Hide, ScrollArea, Sticky",
                r#"Show![above = Breakpoint::Md, Chip!["Only from 768px up"]]
Hide![above = Breakpoint::Md, Chip!["Only below 768px"]]
ScrollArea![max_height = "6rem", label = "Changelog", p!["…"]]
Sticky![blurred = true, bordered = true, Navbar![brand = "Acme"]]      // a sticky header"#,
                Stack![
                    gap = 4,
                    Show![
                        above = Breakpoint::Md,
                        Chip![color = Color::Primary, variant = Variant::Flat, "Only from 768px up"]
                    ],
                    Hide![
                        above = Breakpoint::Md,
                        Chip![color = Color::Secondary, variant = Variant::Flat, "Only below 768px"]
                    ],
                    ScrollArea![
                        max_height = "6rem",
                        label = "Changelog",
                        p!["A focusable region with thin scrollbars."],
                        p!["Scroll with the wheel, touch or the arrow keys."],
                        p!["It never traps the page scroll."],
                        p!["Line four."],
                        p!["Line five."],
                    ],
                ],
            ),
        ],
    )
}
