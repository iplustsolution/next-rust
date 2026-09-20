use next_rust::icons;
use next_rust::prelude::*;
use next_rust::ui::*;

fn demo(title: &str, body: impl View) -> impl View {
    Card![shadow = Shadow::Sm, CardHeader![h3![title]], CardBody![body]]
}

pub fn Page() -> impl View {
    Container![
        width = Width::Lg,
        Stack![
            gap = 6,
            div![h1!["Overlays and menus"], p!["Popovers and dialogs are the browser's own; the script adds keys and polish."]],
            demo(
                "Dropdown, popover, context menu",
                Stack![
                    row = true,
                    wrap = true,
                    gap = 3,
                    align = Align::Center,
                    Dropdown![
                        id = "demo-menu",
                        trigger = Button![variant = Variant::Bordered, end_content = icons::ChevronDown().size(16), "Options"],
                        DropdownSection![
                            title = "Project",
                            DropdownItem![href = "/", start_content = icons::Pencil().size(16), shortcut = "⌘E", "Edit"],
                            DropdownItem![on_press = Press::toast("Duplicated"), start_content = icons::Copy().size(16), description = "Make a copy", "Duplicate"],
                            DropdownItem![disabled = true, start_content = icons::Archive().size(16), "Archive"],
                        ],
                        DropdownSection![DropdownItem![
                            color = Color::Danger,
                            on_press = Press::open_modal("confirm-delete"),
                            start_content = icons::Trash2().size(16),
                            "Delete"
                        ]],
                    ],
                    Dropdown![align = MenuAlign::End, label = "More", DropdownItem![href = "/", "Profile"], DropdownItem![href = "/plain", "Plain page"]],
                    Popover![
                        id = "demo-popover",
                        trigger = Button![variant = Variant::Flat, "Share"],
                        title = "Share this page",
                        side = Side::Top,
                        p!["Anyone with the link can view."],
                        Button![size = Size::Sm, on_press = Press::toast("Link copied"), "Copy link"],
                    ],
                    ContextMenu![
                        target = Card![bordered = true, shadow = Shadow::None, CardBody!["Right click this card"]],
                        DropdownItem![on_press = Press::toast("Renamed"), "Rename"],
                        DropdownItem![color = Color::Danger, on_press = Press::toast("Removed"), "Remove"],
                    ],
                ]
            ),
            demo(
                "Drawer, confirm dialog, command palette, toasts",
                Stack![
                    row = true,
                    wrap = true,
                    gap = 3,
                    Button![on_press = Press::open_modal("filters"), "Open drawer"],
                    Button![variant = Variant::Bordered, on_press = Press::open_modal("sheet"), "Bottom sheet"],
                    Button![color = Color::Danger, variant = Variant::Flat, on_press = Press::open_modal("confirm-delete"), "Delete…"],
                    Button![
                        variant = Variant::Faded,
                        color = Color::Default,
                        on_press = Press::open_modal("commands"),
                        start_content = icons::Search().size(16),
                        end_content = Kbd!["⌘", "K"],
                        "Commands"
                    ],
                    Button![color = Color::Success, on_press = Press::toast("Saved your changes"), "Toast"],
                ]
            ),
            demo(
                "Loading overlay",
                LoadingOverlay![visible = true, blur = true, label = "Crunching numbers…", Skeleton![lines = 4]]
            ),
            Drawer![
                id = "filters",
                title = "Filters",
                ModalBody![Stack![
                    gap = 4,
                    SegmentedControl![name = "status", value = "open", label = "Status", full_width = true, [("open", "Open"), ("closed", "Closed")]],
                    Slider![label = "Price", name = "price", value = 60.0],
                    Checkbox![name = "stock", checked = true, "In stock only"],
                ]],
                ModalFooter![
                    Button![variant = Variant::Light, color = Color::Default, on_press = Press::close_modal(), "Reset"],
                    Button![on_press = Press::close_modal(), "Apply"]
                ],
            ],
            Drawer![id = "sheet", title = "Share", side = DrawerSide::Bottom, size = Width::Sm, ModalBody![p!["A sheet from the bottom edge."]]],
            // A toaster and a palette live on the pages that use them: a page
            // without them never receives their CSS.
            Toaster![],
            CommandPalette![
                id = "commands",
                CommandGroup![
                    title = "Pages",
                    CommandItem![href = "/", keywords = "home buttons inputs", "All components"],
                    CommandItem![href = "/forms", keywords = "slider rating combobox", "Forms"],
                    CommandItem![href = "/overlays", keywords = "dropdown drawer toast", "Overlays and menus"],
                    CommandItem![href = "/data", keywords = "charts table list", "Data display"],
                    CommandItem![href = "/marketing", keywords = "hero pricing footer", "Marketing sections"],
                ],
                CommandGroup![
                    title = "Actions",
                    CommandItem![on_press = Press::toast("Hello from the palette"), shortcut = "T", "Show a toast"]
                ],
            ],
            ConfirmDialog![
                id = "confirm-delete",
                title = "Delete this project?",
                confirm_label = "Delete",
                on_confirm = Press::toast("Deleted"),
                "This cannot be undone."
            ],
        ]
    ]
}
