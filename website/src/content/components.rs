//! The "components" documentation page.

use next_rust::prelude::*;

fn h2(id_: &'static str, text: &'static str) -> Node {
    h2![id(id_), a![class("anchor"), href(format!("#{id_}")), text]].into_node()
}

fn rust(src: &'static str) -> Node {
    pre![code![class("language-rust"), src]].into_node()
}

fn props(rows: &[(&'static str, &'static str)]) -> Node {
    div![
        class("table-wrap"),
        table![
            thead![tr![th!["property"], th!["what it does"]]],
            tbody![each(rows.iter().copied(), |(name, what)| tr![td![code![name]], td![what]])],
        ]
    ]
    .into_node()
}

pub fn content() -> Node {
    fragment![
        p![
            "Next Rust ships a set of ready-made components: buttons, text fields, a password field, a custom select, a date picker, checkboxes, switches, radios, avatars, cards, chips, spinners and layout. They are written, configured and rendered in Rust, look good without any CSS of your own, and every property is optional.",
        ],
        rust(
            r#"use next_rust::prelude::*;
use next_rust::ui::*;

pub fn Page(form: FormState) -> impl View {
    Card![
        CardHeader![h2!["Create an account"]],
        CardBody![form![
            action!(crate::actions::sign_up),
            Input![name = "email", label = "Email", kind = "email", error_message = form.error("email")],
            PasswordInput![name = "password", label = "Password", new_password = true],
            Select![name = "plan", label = "Plan", [("free", "Free"), ("pro", "Pro")]],
            DatePicker![name = "birthday", label = "Birthday", max = "2010-12-31"],
            Checkbox![name = "terms", required = true, "I accept the terms"],
            Button![submit = true, full_width = true, "Sign up"],
        ]],
    ]
}"#,
        ),
        p!["Run the showcase with ", code!["cargo run -p example-ui"], " from ", code!["examples/ui"], ".",],
        h2("syntax", "Syntax"),
        p![
            "Each component is a builder with a macro of the same name. Inside the macro, ",
            code!["key = value"],
            " sets a property; anything else is added the way it is in an element macro. Attributes such as ",
            code!["class(..)"],
            ", ",
            code!["id(..)"],
            ", ",
            code!["aria(..)"],
            " and ",
            code!["data(..)"],
            " are merged in, and everything else becomes a child. The builder form works too: ",
            code!["Button::new().color(Color::Danger).with(\"Delete\")"],
            ".",
        ],
        p![
            "On fields (",
            code!["Input"],
            ", ",
            code!["Select"],
            ", ",
            code!["DatePicker"],
            ", ...), extra attributes go to the control itself, so ",
            code!["attr(\"autocomplete\", \"email\")"],
            " lands on the ",
            code!["<input>"],
            ". ",
            code!["class(..)"],
            " always goes to the outer element.",
        ],
        h2("styling", "Styling and overriding"),
        ul![
            li![
                "Every component has default classes (",
                code!["nr-btn"],
                ", ",
                code!["nr-field"],
                ", ...) whose rules live in the CSS ",
                code!["components"],
                " layer. Classes you add, whether Tailwind utilities or your own CSS, always win over them, without ",
                code!["!important"],
                ": ",
                code!["Button![class(\"rounded-full px-8\"), \"Go\"]"],
                ".",
            ],
            li![
                "Inner parts take their own classes: ",
                code!["label_class"],
                ", ",
                code!["wrapper_class"],
                ", ",
                code!["input_class"],
                ", ",
                code!["description_class"],
                " and ",
                code!["error_class"],
                " on fields.",
            ],
            li![code!["unstyled = true"], " drops the default classes of the outer element entirely."],
            li![
                "Common properties: ",
                code!["color"],
                " (",
                code!["Color::Default"],
                ", ",
                code!["Primary"],
                ", ",
                code!["Secondary"],
                ", ",
                code!["Success"],
                ", ",
                code!["Warning"],
                ", ",
                code!["Danger"],
                "), ",
                code!["size"],
                " (",
                code!["Size::Sm"],
                ", ",
                code!["Md"],
                ", ",
                code!["Lg"],
                ") and ",
                code!["radius"],
                " (",
                code!["Radius::None"],
                " to ",
                code!["Full"],
                ").",
            ],
            li![
                "Pages receive only the CSS of the components they render, and on client-side navigations only what the browser doesn't have yet.",
            ],
        ],
        h2("theming", "Theming and dark mode"),
        p!["Colors, radii and shadows are CSS variables. Set them once to restyle every component:"],
        pre![code![
            class("language-css"),
            ":root {\n  --nr-primary: #e11d48;       /* also: --nr-secondary, --nr-success, --nr-warning, --nr-danger */\n  --nr-primary-text: #be123c;  /* the color as text on a tint */\n  --nr-radius-md: 10px;        /* --nr-radius-sm, --nr-radius-lg */\n  --nr-font: \"Inter\", sans-serif;\n}",
        ]],
        p![
            "Dark mode follows the operating system. ",
            code!["class=\"dark\""],
            " or ",
            code!["data-theme=\"dark\""],
            " on an ancestor forces it, and ",
            code!["class=\"light\""],
            " / ",
            code!["data-theme=\"light\""],
            " on ",
            code!["<html>"],
            " opts out.",
        ],
        h2("buttons", "Button"),
        rust(
            r#"Button![color = Color::Danger, variant = Variant::Bordered, "Delete"]
Button![href = "/settings", variant = Variant::Flat, "Settings"]     // a link, client-side navigation
Button![icon_only = true, aria("label", "Close"), raw_html(CLOSE_SVG)]
Button![loading = true, "Saving"]
ButtonGroup![Button!["Day"], Button!["Week"], Button!["Month"]]"#,
        ),
        props(&[
            ("variant", "Solid (default), Bordered, Light, Flat, Faded, Shadow, Ghost"),
            ("color, size, radius", "see above; buttons are primary by default"),
            ("submit", "type=\"submit\"; shows a spinner while its form posts to a server action"),
            ("href", "render a link that looks like a button"),
            ("loading, disabled, full_width, icon_only", "states and shapes"),
            ("start_content, end_content", "icons around the label"),
            ("on_press / on_click", "what a press does (below)"),
        ]),
        h2("on-press", "Pressing things"),
        p![
            "The code in a Rust view runs on the server, so ",
            code!["on_press"],
            " takes a description of what the browser should do:",
        ],
        rust(
            r#"Button![on_press = action!(archive), "Archive"]                   // call a server action
Button![on_press = Press::from(action!(like)).input(&post.id), "Like"]
Button![on_press = Press::navigate("/settings"), "Settings"]
Button![on_press = Press::emit("open-cart"), "Cart"]               // an `nr:open-cart` DOM event
Card![on_press = Press::navigate("/post/1"), ...]"#,
        ),
        p![
            "After an action succeeds the page refreshes, unless you add ",
            code![".no_refresh()"],
            ". The element receives ",
            code!["nr:success"],
            " / ",
            code!["nr:error"],
            " events, and shows a spinner while the action runs.",
        ],
        h2("fields", "Input, PasswordInput and Textarea"),
        rust(
            r#"Input![label = "Email", kind = "email", placeholder = "you@example.com", start_content = raw_html(MAIL_SVG)]
Input![label = "Search", clearable = true, variant = FieldVariant::Bordered]
PasswordInput![label = "Password", name = "password", new_password = true]
Textarea![label = "Message", rows = 4]"#,
        ),
        props(&[
            ("label, placeholder, description", "texts; without a placeholder the label floats inside the box"),
            ("label_placement", "LabelPlacement::Inside (default), Outside, OutsideLeft"),
            ("variant", "FieldVariant::Flat (default), Bordered, Faded, Underlined"),
            ("name, value, id, kind", "form name, initial value, control id, input type"),
            ("error_message, invalid", "show an error; accepts form.error(\"name\") directly"),
            ("required, disabled, readonly", "the usual states; required fields get an asterisk"),
            ("start_content, end_content, clearable", "content inside the box, and a clear button"),
        ]),
        p![
            "Fields with a ",
            code!["name"],
            " render an error slot that the client runtime fills when a server action returns validation errors. Forms enhanced by the runtime therefore show errors under the right fields without a reload, and the field turns red on its own.",
        ],
        h2("select", "Select"),
        rust(
            r#"Select![label = "Country", name = "country", placeholder = "Choose a country",
    SelectItem![value = "in", description = "Asia", "India"],
    SelectItem![value = "de", start_content = Avatar![name = "DE"], "Germany"],
    SelectItem![value = "us", disabled = true, "United States"],
]
Select![label = "Toppings", multiple = true, values = ["cheese"], [("cheese", "Cheese"), ("basil", "Basil")]]"#,
        ),
        p![
            "A styled listbox with keyboard navigation (arrows, Home, End, Enter, Escape), type-ahead, single or multiple choice. It opens above everything else on the page, so a card or a scrolling container never cuts it off. Underneath it is a real ",
            code!["<select>"],
            ": forms submit it as usual, and without JavaScript the native control is shown.",
        ],
        h2("date-picker", "DatePicker"),
        rust(
            r#"DatePicker![label = "Check-in", name = "check_in", min = "2026-01-01", max = "2026-12-31", first_day_of_week = 1]"#,
        ),
        p![
            "A custom calendar with month and year views, keyboard navigation (arrows, Page Up/Down, Home, End), a Today and a Clear button, and ",
            code!["min"],
            "/",
            code!["max"],
            ". Dates display in the visitor's locale (or ",
            code!["locale = \"de-DE\""],
            ") and are submitted as ",
            code!["YYYY-MM-DD"],
            ". Without JavaScript it is the browser's date input.",
        ],
        h2("toggles", "Checkbox, Switch and RadioGroup"),
        rust(
            r#"Checkbox![name = "remember", checked = true, "Remember me"]
Switch![name = "wifi", color = Color::Success, "Wi-Fi"]
RadioGroup![label = "Plan", name = "plan", value = "pro", horizontal = true,
    Radio![value = "free", "Free"],
    Radio![value = "pro", description = "For teams", "Pro"],
]"#,
        ),
        p!["These are real inputs, styled with CSS only. They need no script."],
        h2("avatars", "Avatar and AvatarGroup"),
        rust(
            r#"Avatar![src = user.photo_url, name = "Ada Lovelace", size = Size::Lg, bordered = true]
Avatar![name = "Ada Lovelace"]                     // initials: "AL"
AvatarGroup![max = 3, total = 12, each(team, |m| Avatar![src = m.photo, name = m.name])]"#,
        ),
        p!["Circular by default. When the image is missing or fails to load, the initials show instead."],
        h2("display", "Card, Chip, Spinner and Divider"),
        rust(
            r#"Card![shadow = Shadow::Sm, hoverable = true,
    CardHeader![h3!["Pro plan"]],
    CardBody![p!["Everything you need."]],
    CardFooter![Button!["Upgrade"]],
]
Chip![color = Color::Success, variant = Variant::Flat, dot = true, "Online"]
Spinner![color = Color::Secondary, label = "Loading…"]
Divider![]"#,
        ),
        h2("layout", "Layout"),
        rust(
            r#"// app/layout.rs: a responsive app layout in a few lines.
AppShell![
    navbar = Navbar![brand = strong!["Acme"], menu_toggle = true, NavbarItem![href = "/", "Home"]],
    sidebar = Sidebar![title = "Workspace", SidebarItem![href = "/projects", prefix = true, "Projects"]],
    children,
]

Container![width = Width::Lg, ...]                          // centered, padded
Stack![row = true, gap = 4, align = Align::Center, ...]     // flex with spacing
Grid![cols = 3, ...]                                        // 3 columns, 2 on tablets, 1 on phones
Grid![min_width = "16rem", ...]                             // as many columns as fit"#,
        ),
        p![
            "Navbar and sidebar links highlight themselves on their page. On small screens the sidebar becomes a drawer, opened by the navbar's menu button.",
        ],
        h2("icons", "Icons"),
        p![
            "Every ",
            a![href("https://lucide.dev/icons"), "Lucide icon"],
            " is built in as ",
            code!["next_rust::icons"],
            ": a function per icon, named like on lucide.dev in PascalCase (",
            code!["arrow-right"],
            " → ",
            code!["ArrowRight"],
            "). Former names such as ",
            code!["Home"],
            " and ",
            code!["Trash2"],
            " work too.",
        ],
        rust(
            r##"use next_rust::icons;

icons::House()                                              // 24px, currentColor, stroke 2
icons::ArrowRight().size(16).color("#e11d48").stroke_width(1.5)
icons::Search().size("1.25em").absolute_stroke_width(true)
icons::Heart().fill("currentColor").class("text-rose-500")  // Tailwind classes work
icons::Bell().title("Notifications")                        // accessible name + tooltip
icons::Star().with(data("rating", "5"))                     // any attribute
icons::by_name("house")                                     // from a string (keeps every icon in the binary)

Button![start_content = icons::Plus().size(16), "New project"]
Input![label = "Search", start_content = icons::Search().size(18)]"##,
        ),
        p![
            "Icons are inline SVG: no font, no request, and unused icons are left out of the binary. Hovering an icon function in your editor shows a preview.",
        ],
        h2("without-javascript", "Scripts and progressive enhancement"),
        p![
            "Interactive components (select, date picker, password toggle, clear buttons, ",
            code!["on_press"],
            ", the drawer) are driven by a small script, ",
            code!["/_nr/ui.js"],
            " (about 4.6 KB gzipped). It is added only to pages that render one of them. Without it, every component falls back to the native control, so forms still submit.",
        ],
    ]
}
