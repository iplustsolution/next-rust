use next_rust::prelude::*;
use next_rust::icons;
use next_rust::ui::*;

use crate::actions::{self, LIKES};


fn section(title: &str, body: impl View) -> impl View {
    Card![shadow = Shadow::Sm, CardHeader![h3![title]], CardBody![body]]
}

pub fn Page(form: FormState) -> impl View {
    let colors = [
        (Color::Default, "Default"),
        (Color::Primary, "Primary"),
        (Color::Secondary, "Secondary"),
        (Color::Success, "Success"),
        (Color::Warning, "Warning"),
        (Color::Danger, "Danger"),
    ];
    let variants = [
        (Variant::Solid, "Solid"),
        (Variant::Bordered, "Bordered"),
        (Variant::Light, "Light"),
        (Variant::Flat, "Flat"),
        (Variant::Faded, "Faded"),
        (Variant::Shadow, "Shadow"),
        (Variant::Ghost, "Ghost"),
    ];
    Container![
        width = Width::Lg,
        Stack![
            gap = 6,
            div![h1!["Components"], p!["Every component below is plain Rust: no JavaScript to write."]],
            section(
                "Buttons",
                Stack![
                    gap = 4,
                    Stack![row = true, wrap = true, gap = 2, each(colors, |(c, name)| Button![color = c, name])],
                    Stack![
                        row = true,
                        wrap = true,
                        gap = 2,
                        each(variants, |(v, name)| Button![variant = v, color = Color::Primary, name])
                    ],
                    Stack![
                        row = true,
                        wrap = true,
                        gap = 2,
                        align = Align::Center,
                        Button![size = Size::Sm, "Small"],
                        Button![size = Size::Md, "Medium"],
                        Button![size = Size::Lg, "Large"],
                        Button![loading = true, "Saving"],
                        Button![disabled = true, "Disabled"],
                        Button![radius = Radius::Full, color = Color::Secondary, variant = Variant::Shadow, "Rounded"],
                        Button![icon_only = true, color = Color::Danger, variant = Variant::Flat, aria("label", "Like"), icons::Heart().fill("currentColor").size("1.15em")],
                        Button![
                            color = Color::Danger,
                            variant = Variant::Bordered,
                            on_press = action!(actions::like),
                            start_content = icons::Heart().fill("currentColor").size("1.15em"),
                            format!("Like ({})", LIKES.load(std::sync::atomic::Ordering::SeqCst))
                        ],
                    ],
                    ButtonGroup![
                        Button![variant = Variant::Bordered, "One"],
                        Button![variant = Variant::Bordered, "Two"],
                        Button![variant = Variant::Bordered, "Three"],
                    ],
                ]
            ),
            section(
                "Inputs",
                Stack![gap = 4, Grid![
                    cols = 2,
                    Input![label = "Email", kind = "email", placeholder = "you@example.com", start_content = icons::Mail().size(18)],
                    Input![label = "Floating label", description = "The label moves up once you type."],
                    Input![label = "Bordered", variant = FieldVariant::Bordered, clearable = true, value = "Clear me"],
                    Input![label = "Faded", variant = FieldVariant::Faded, color = Color::Secondary],
                    Input![label = "Underlined", variant = FieldVariant::Underlined, label_placement = LabelPlacement::Outside],
                    Input![label = "With an error", value = "not-an-email", error_message = "Enter a valid email address"],
                    PasswordInput![label = "Password", description = "Use the eye to show it."],
                    Input![label = "Outside label", label_placement = LabelPlacement::Outside, placeholder = "Placeholder", size = Size::Sm],
                ],
                Textarea![label = "Message", placeholder = "Write something…", rows = 3]],
            ),
            section(
                "Select",
                Grid![
                    cols = 2,
                    Select![
                        label = "Country",
                        placeholder = "Choose a country",
                        SelectItem![value = "in", description = "Asia", "India"],
                        SelectItem![value = "de", description = "Europe", "Germany"],
                        SelectItem![value = "jp", description = "Asia", "Japan"],
                        SelectItem![value = "br", description = "South America", "Brazil"],
                        SelectItem![value = "us", description = "North America", disabled = true, "United States"],
                    ],
                    Select![
                        label = "Team",
                        value = "ada",
                        variant = FieldVariant::Bordered,
                        SelectItem![value = "ada", start_content = Avatar![name = "Ada Lovelace", size = Size::Sm], "Ada Lovelace"],
                        SelectItem![value = "alan", start_content = Avatar![name = "Alan Turing", size = Size::Sm, color = Color::Primary], "Alan Turing"],
                        SelectItem![value = "grace", start_content = Avatar![name = "Grace Hopper", size = Size::Sm, color = Color::Success], "Grace Hopper"],
                    ],
                    Select![
                        label = "Toppings",
                        multiple = true,
                        values = ["cheese", "olives"],
                        [("cheese", "Cheese"), ("olives", "Olives"), ("basil", "Basil"), ("chili", "Chili")],
                    ],
                ]
            ),
            section(
                "Date picker",
                Grid![
                    cols = 2,
                    DatePicker![label = "Birthday", value = "1990-05-17"],
                    DatePicker![
                        label = "Check-in",
                        min = "2026-01-01",
                        max = "2026-12-31",
                        first_day_of_week = 1,
                        variant = FieldVariant::Bordered,
                        description = "Mondays first, 2026 only."
                    ],
                ]
            ),
            section(
                "Checkbox, switch, radio",
                Grid![
                    cols = 3,
                    Stack![
                        gap = 3,
                        Checkbox![checked = true, "Remember me"],
                        Checkbox![color = Color::Success, description = "Weekly, no spam.", "Newsletter"],
                        Checkbox![disabled = true, "Disabled"],
                    ],
                    Stack![
                        gap = 3,
                        Switch![checked = true, "Wi-Fi"],
                        Switch![color = Color::Secondary, "Bluetooth"],
                        Switch![size = Size::Sm, color = Color::Success, checked = true, "Small"],
                    ],
                    RadioGroup![
                        label = "Plan",
                        name = "demo-plan",
                        value = "pro",
                        Radio![value = "free", "Free"],
                        Radio![value = "pro", description = "Everything, for teams.", "Pro"],
                        Radio![value = "enterprise", "Enterprise"],
                    ],
                ]
            ),
            section(
                "Avatars, chips, spinners",
                Stack![
                    gap = 5,
                    Stack![
                        row = true,
                        gap = 4,
                        align = Align::Center,
                        Avatar![name = "Ada Lovelace", size = Size::Lg, color = Color::Primary, bordered = true],
                        Avatar![src = "/does-not-exist.png", name = "Grace Hopper", size = Size::Lg, color = Color::Success],
                        Avatar![name = "Linus", color = Color::Warning],
                        AvatarGroup![
                            max = 3,
                            total = 12,
                            Avatar![name = "A B", color = Color::Primary],
                            Avatar![name = "C D", color = Color::Secondary],
                            Avatar![name = "E F", color = Color::Success],
                            Avatar![name = "G H", color = Color::Danger],
                        ],
                    ],
                    Stack![
                        row = true,
                        wrap = true,
                        gap = 2,
                        Chip!["Default"],
                        Chip![color = Color::Primary, "Primary"],
                        Chip![color = Color::Success, variant = Variant::Flat, dot = true, "Online"],
                        Chip![color = Color::Warning, variant = Variant::Bordered, "Pending"],
                        Chip![color = Color::Danger, variant = Variant::Shadow, "Hot"],
                    ],
                    Stack![
                        row = true,
                        gap = 6,
                        align = Align::Center,
                        Spinner![size = Size::Sm],
                        Spinner![color = Color::Secondary],
                        Spinner![color = Color::Success, size = Size::Lg, label = "Loading…"],
                    ],
                ]
            ),
            section(
                "Alerts, badges, progress",
                Stack![
                    gap = 4,
                    Alert![title = "Heads up", "Your trial ends in 3 days."],
                    Alert![color = Color::Success, variant = Variant::Faded, title = "Saved", closable = true, "Your changes are live."],
                    Alert![
                        color = Color::Danger,
                        variant = Variant::Bordered,
                        title = "Payment failed",
                        end_content = Button![size = Size::Sm, color = Color::Danger, variant = Variant::Flat, "Retry"],
                        "The card was declined."
                    ],
                    Stack![
                        row = true,
                        gap = 6,
                        align = Align::Center,
                        Badge![content = "3", Avatar![name = "Ada Lovelace"]],
                        Badge![dot = true, color = Color::Success, placement = Placement::BottomRight, Avatar![name = "Grace Hopper"]],
                        Badge![content = "99+", color = Color::Primary, variant = Variant::Flat, Button![variant = Variant::Bordered, icon_only = true, icons::Bell().size(18)]],
                        Kbd!["⌘", "K"],
                    ],
                    Progress![label = "Uploading", value = 62.0, show_value = true],
                    Progress![color = Color::Success, size = Size::Sm, value = 100.0, value_label = "Done"],
                    Progress![label = "Working…", striped = true],
                    Stack![
                        row = true,
                        gap = 6,
                        CircularProgress![value = 75.0, show_value = true, label = "Mastery"],
                        CircularProgress![color = Color::Warning, size = Size::Lg, value = 40.0, show_value = true],
                        CircularProgress![size = Size::Sm, label = "Loading"],
                    ],
                    Grid![cols = 3, Stat![label = "Students", value = "1,204", delta = "12%", trend = Trend::Up, icon = icons::Users().size(20), color = Color::Primary],
                        Stat![label = "Average score", value = "81%", delta = "3%", trend = Trend::Down, description = "vs. last term"],
                        Stat![label = "Sessions", value = "48", delta = "0", trend = Trend::Flat]],
                    Skeleton![lines = 3],
                ]
            ),
            section(
                "Tabs, breadcrumbs, steps",
                Stack![
                    gap = 5,
                    Breadcrumbs![BreadcrumbItem![href = "/", start_content = icons::House().size(14), "Home"], BreadcrumbItem![href = "/", "Classes"], BreadcrumbItem!["Grade 9 Maths"]],
                    Tabs![
                        selected = "grades",
                        Tab![key = "overview", title = "Overview", p!["The overview panel."]],
                        Tab![key = "grades", title = "Grades", end_content = Chip![size = Size::Sm, "12"], p!["The grades panel."]],
                        Tab![key = "settings", title = "Settings", disabled = true, p!["Settings."]],
                    ],
                    Tabs![variant = TabsVariant::Underlined, color = Color::Primary, Tab![title = "Photos", href = "/?tab=photos", selected = true], Tab![title = "Music", href = "/?tab=music"], Tab![title = "Videos", href = "/?tab=videos"]],
                    Tabs![variant = TabsVariant::Bordered, size = Size::Sm, full_width = true, Tab![title = "Day", p!["Day"]], Tab![title = "Week", p!["Week"]], Tab![title = "Month", p!["Month"]]],
                    Steps![current = 1, Step![title = "Account", description = "Name and email", href = "/"], Step![title = "Plan", description = "Pick a plan"], Step![title = "Payment"], Step![title = "Done"]],
                ]
            ),
            section(
                "Accordion, table, empty state",
                Stack![
                    gap = 5,
                    Accordion![
                        variant = AccordionVariant::Splitted,
                        AccordionItem![title = "What is Next Rust?", subtitle = "The short version", expanded = true, p!["A web framework: routes are folders, pages are Rust, one binary ships."]],
                        AccordionItem![title = "Do I need JavaScript?", p!["No. Components fall back to native controls."]],
                        AccordionItem![title = "Can I use Tailwind?", p!["Yes, it is on by default."]],
                    ],
                    Table![
                        columns = ["Student", "Score", "Status"],
                        striped = true,
                        hoverable = true,
                        tr![td!["Ada Lovelace"], td!["98"], td![Chip![size = Size::Sm, color = Color::Success, variant = Variant::Flat, "Passed"]]],
                        tr![td!["Grace Hopper"], td!["95"], td![Chip![size = Size::Sm, color = Color::Success, variant = Variant::Flat, "Passed"]]],
                        tr![td!["Alan Turing"], td!["61"], td![Chip![size = Size::Sm, color = Color::Warning, variant = Variant::Flat, "Review"]]],
                    ],
                    Table![columns = ["Name", "Email"], empty = EmptyState![compact = true, title = "No students yet", description = "Invite someone to see them here.", Button![size = Size::Sm, "Invite"]]],
                    Stack![
                        row = true,
                        gap = 3,
                        Tooltip![content = "Opens a dialog", Button![on_press = Press::open_modal("demo-modal"), "Open modal"]],
                        Tooltip![content = "To the right", side = Side::Right, color = Color::Primary, Button![variant = Variant::Bordered, "Hover me"]],
                    ],
                    Modal![
                        id = "demo-modal",
                        title = "Archive this class?",
                        ModalBody![p!["Students keep their work; the class leaves your list."]],
                        ModalFooter![Button![variant = Variant::Light, on_press = Press::close_modal(), "Cancel"], Button![color = Color::Danger, on_press = Press::close_modal(), "Archive"]],
                    ],
                ]
            ),
            section(
                "A real form",
                form![
                    action!(actions::sign_up),
                    Stack![
                        gap = 4,
                        Input![name = "name", label = "Name", value = form.value("name"), error_message = form.error("name")],
                        Input![
                            name = "email",
                            label = "Email",
                            kind = "email",
                            value = form.value("email"),
                            error_message = form.error("email")
                        ],
                        PasswordInput![name = "password", label = "Password", new_password = true, error_message = form.error("password")],
                        Select![
                            name = "plan",
                            label = "Plan",
                            error_message = form.error("plan"),
                            [("free", "Free"), ("pro", "Pro"), ("team", "Team")],
                        ],
                        p![data("nr-error", "_form"), form.message.clone().unwrap_or_default()],
                        Button![submit = true, full_width = true, size = Size::Lg, "Create account"],
                    ],
                ]
            ),
        ],
    ]
}
