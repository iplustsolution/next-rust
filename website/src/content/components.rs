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

/// The box that opens the page: the feature without the jargon.
fn plain_words(text: impl View) -> Node {
    div![class("callout"), p![strong!["In plain words. "], text]].into_node()
}

pub fn content() -> Node {
    fragment![
        plain_words(
            "A component is a ready-made piece of a web page: a button, a date picker, a pricing table. Think of \
             furniture that arrives assembled. Instead of building a dropdown from raw HTML, styling it and making it \
             work with a keyboard, you write one line saying \"a dropdown with these items\" and get one that \
             already looks right, works on phones, supports dark mode and can be used by people with screen readers."
        ),
        p![
            "Next Rust ships a set of ready-made components: buttons and text fields, a custom select, a date picker, comboboxes, sliders, ratings and file uploads; dropdowns, popovers, drawers, modals, toasts and a command palette; heroes, pricing tables, testimonials, carousels and footers; tables, lists, code blocks, callouts and server-rendered charts. They are written, configured and rendered in Rust, look good without any CSS of your own, and every property is optional.",
        ],
        div![
            class("callout"),
            p![
                strong!["See them all: "],
                a![href("/components"), "the component gallery"],
                " shows every component with the code that makes it. Each example has a ",
                strong!["Preview"],
                " button that opens the live, working component in a popup, so you can click, type and tab through \
                 it before copying the code. This page is the reference for the properties; the gallery is the \
                 place to look and try.",
            ],
            p![
                strong!["Looking for an icon? "],
                a![href("/icons"), "The icon browser"],
                " searches every built-in icon and copies it as a Rust function call or as SVG.",
            ],
        ],
        p![
            "Nothing needs installing: the components are part of the ",
            code!["next-rust"],
            " crate, and one line, ",
            code!["use next_rust::ui::*;"],
            ", brings them into a file. They are optional. A page can mix them freely with plain element macros (",
            code!["div!"],
            ", ",
            code!["p!"],
            "), and a project that never uses them ships none of their CSS or script.",
        ],
        p![
            "The example below is a complete sign-up page, for example ",
            code!["app/signup/page.rs"],
            ": a card containing a form that posts to a ",
            a![href("/docs/server-actions"), "server action"],
            ", with an email field that shows the server's validation error, a password field, a select, a date \
             picker, a checkbox and a submit button.",
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
        p![
            "To run the same showcase on your own machine, clone the framework repository and run ",
            code!["cargo run -p example-ui"],
            " (the project lives in ",
            code!["examples/ui"],
            ").",
        ],
        p!["Words used on this page:"],
        ul![
            li![
                strong!["Property"],
                " (prop): a named setting of a component, such as ",
                code!["color"],
                " or ",
                code!["label"],
                ".",
            ],
            li![strong!["Variant"], ": one of a component's visual styles (solid, bordered, flat, ...).",],
            li![
                strong!["Native control"],
                ": the browser's own version of a control, such as its built-in date input. Components fall back to \
                 it when JavaScript is off.",
            ],
            li![
                strong!["Progressive enhancement"],
                ": everything works as plain HTML first; scripts only add comfort on top.",
            ],
            li![
                strong!["ARIA"],
                ": the attributes that tell screen readers what an element is and what state it is in. The \
                 components set them for you.",
            ],
        ],
        h2("syntax", "Syntax"),
        p![
            "Every snippet on this page goes inside a view: a ",
            code!["Page"],
            ", a ",
            code!["Layout"],
            " or a component function of your own.",
        ],
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
        p![
            "Colors, radii and shadows are CSS variables (named values that every rule reads). Set them once to \
             restyle every component. This block goes in your global stylesheet (",
            code!["app/globals.css"],
            ", or a file listed in ",
            code!["[tailwind] stylesheets"],
            "; see ",
            a![href("/docs/styling-and-assets#global-css"), "Global CSS"],
            "):",
        ],
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
        h2("alerts", "Alert, Badge and Kbd"),
        rust(
            r#"Alert![color = Color::Success, title = "Saved", closable = true, "Your changes are live."]
Alert![color = Color::Danger, variant = Variant::Bordered, title = "Payment failed",
    end_content = Button![size = Size::Sm, "Retry"], "The card was declined."]
Badge![content = "3", Avatar![name = "Ada Lovelace"]]                    // a count on a corner
Badge![dot = true, color = Color::Success, placement = Placement::BottomRight, Avatar![name = "Grace"]]
Kbd!["⌘", "K"]"#,
        ),
        p![
            "An alert's icon follows its color (info, check, warning, error) unless you give ",
            code!["icon"],
            " or ",
            code!["hide_icon"],
            ". Warnings and errors have ",
            code!["role=\"alert\""],
            ", so screen readers announce them. ",
            code!["closable"],
            " adds a dismiss button; the component script removes the alert and fires ",
            code!["nr:dismiss"],
            ".",
        ],
        h2("progress", "Progress, CircularProgress and Skeleton"),
        rust(
            r#"Progress![label = "Uploading", value = 62.0, show_value = true]          // 62%
Progress![color = Color::Success, size = Size::Sm, value = 8.0, max = 10.0, value_label = "8 of 10"]
Progress![label = "Working…", striped = true]                             // no value: indeterminate
CircularProgress![value = 75.0, show_value = true, label = "Mastery"]
Skeleton![class("h-40")]                                                  // one shimmering box
Skeleton![lines = 3]                                                      // three text lines"#,
        ),
        p![
            "Both progress components are ",
            code!["role=\"progressbar\""],
            " with the ARIA values set, and animate between values.",
        ],
        h2("stat", "Stat"),
        rust(
            r#"Stat![label = "Students", value = "1,204", delta = "12%", trend = Trend::Up,
    icon = icons::Users().size(20), color = Color::Primary]
Stat![label = "Average score", value = "81%", delta = "3%", trend = Trend::Down, description = "vs. last term"]"#,
        ),
        h2("tabs", "Tabs"),
        rust(
            r#"// Panels switched in place (the component script does the switching):
Tabs![selected = "grades",
    Tab![key = "overview", title = "Overview", p!["…"]],
    Tab![key = "grades", title = "Grades", end_content = Chip![size = Size::Sm, "12"], p!["…"]],
    Tab![key = "settings", title = "Settings", disabled = true, p!["…"]],
]
// Links between pages (no script; the server marks the current one):
Tabs![variant = TabsVariant::Underlined, color = Color::Primary,
    Tab![title = "Photos", href = "/album?tab=photos", selected = tab == "photos"],
    Tab![title = "Music", href = "/album?tab=music", selected = tab == "music"],
]"#,
        ),
        p![
            "Variants: ",
            code!["Solid"],
            " (a segmented control, the default), ",
            code!["Underlined"],
            ", ",
            code!["Bordered"],
            " and ",
            code!["Light"],
            "; plus ",
            code!["size"],
            ", ",
            code!["color"],
            ", ",
            code!["radius"],
            " and ",
            code!["full_width"],
            ". Tabs with panels follow the ARIA tabs pattern: arrow keys, Home and End move between them, and the strip fires ",
            code!["nr:change"],
            " with the new key. Without the script, the selected panel shows and the others stay hidden, so prefer link tabs for pages that must work without JavaScript.",
        ],
        h2("breadcrumbs", "Breadcrumbs and Steps"),
        rust(
            r#"Breadcrumbs![
    BreadcrumbItem![href = "/", start_content = icons::House().size(14), "Home"],
    BreadcrumbItem![href = "/classes", "Classes"],
    BreadcrumbItem!["Grade 9 Maths"],                              // the last one is the current page
]
Steps![current = 1,
    Step![title = "Account", description = "Name and email", href = "/signup"],  // done: a check mark
    Step![title = "Plan"],                                                       // current
    Step![title = "Payment"], Step![title = "Done"],                             // upcoming
]
Steps![vertical = true, Step![title = "Ordered"], Step![title = "Shipped", status = StepStatus::Error]]"#,
        ),
        h2("accordion", "Accordion"),
        rust(
            r#"Accordion![variant = AccordionVariant::Splitted,
    AccordionItem![title = "What is Next Rust?", subtitle = "The short version", expanded = true, p!["…"]],
    AccordionItem![title = "Do I need JavaScript?", p!["No."]],
]
Accordion![multiple = true, compact = true, ..]     // several open at once, tighter"#,
        ),
        p![
            "Each item is a ",
            code!["<details>"],
            " element: it opens and closes with no script at all, and items of one accordion share a ",
            code!["name"],
            " so opening one closes the others (unless ",
            code!["multiple"],
            ").",
        ],
        h2("modal", "Modal"),
        rust(
            r#"Button![on_press = Press::open_modal("archive"), "Archive…"]

Modal![id = "archive", title = "Archive this class?",
    ModalBody![p!["Students keep their work; the class leaves your list."]],
    ModalFooter![
        Button![variant = Variant::Light, on_press = Press::close_modal(), "Cancel"],
        form![action!(crate::actions::archive), input![r#type("hidden"), name("id"), value(id)],
              Button![submit = true, color = Color::Danger, "Archive"]],
    ],
]"#,
        ),
        p![
            "A ",
            code!["<dialog>"],
            " element. ",
            code!["Press::open_modal"],
            " shows it as a true modal: focus stays inside, the page behind is inert, ",
            code!["Escape"],
            " and a click on the backdrop close it (",
            code!["dismissable = false"],
            " keeps the backdrop click from closing). Its close button is a ",
            code!["method=\"dialog\""],
            " form, so it works without the script too; ",
            code!["open = true"],
            " shows a modal on page load. Sizes are ",
            code!["Width::Sm"],
            " to ",
            code!["Width::Full"],
            "; ",
            code!["placement"],
            " puts it at the top or as a bottom sheet; ",
            code!["scroll_inside"],
            " keeps the header and footer in place while a long body scrolls. The dialog fires ",
            code!["nr:open"],
            " and ",
            code!["nr:close"],
            ".",
        ],
        h2("table", "Table"),
        rust(
            r#"Table![columns = ["Student", "Score", "Status"], striped = true, hoverable = true,
    each(rows, |r| tr![td![r.name.clone()], td![r.score.to_string()], td![Chip![size = Size::Sm, r.status.clone()]]]),
    empty = EmptyState![compact = true, title = "No students yet", Button![size = Size::Sm, "Invite"]],
]"#,
        ),
        p![
            "Rows are ordinary ",
            code!["tr!"],
            " elements; the table adds the header, the styling (",
            code!["striped"],
            ", ",
            code!["compact"],
            ", ",
            code!["bordered"],
            ", ",
            code!["hoverable"],
            ", ",
            code!["sticky_header"],
            "), a ",
            code!["caption"],
            ", horizontal scrolling for wide tables, and shows ",
            code!["empty"],
            " across the table when there are no rows.",
        ],
        h2("tooltip", "Tooltip and EmptyState"),
        rust(
            r#"Tooltip![content = "Copy the link", side = Side::Bottom, Button![icon_only = true, icons::Link().size(18)]]
EmptyState![title = "No classes yet", description = "Create one to get started.", Button!["New class"]]"#,
        ),
        p![
            "Tooltips are CSS only: they appear on hover and on keyboard focus, and the anchor gets ",
            code!["aria-describedby"],
            ". Make the anchor focusable (a button, a link) so keyboard users get the text too.",
        ],
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
        h2("overlays", "Dropdown, Popover, Drawer, ConfirmDialog and ContextMenu"),
        rust(
            r#"Dropdown![trigger = Button![variant = Variant::Bordered, "Options"],
    DropdownSection![title = "Project",
        DropdownItem![href = "/edit", start_content = icons::Pencil().size(16), shortcut = "⌘E", "Edit"],
        DropdownItem![on_press = action!(duplicate), description = "Make a copy", "Duplicate"]],
    DropdownSection![DropdownItem![color = Color::Danger, on_press = Press::open_modal("confirm"), "Delete"]],
]
Popover![trigger = Button!["Share"], title = "Share this page", side = Side::Top, p!["Anyone with the link can view."]]
Drawer![id = "filters", title = "Filters", side = DrawerSide::Right, size = Width::Md, ModalBody![...], ModalFooter![...]]
ConfirmDialog![id = "confirm", title = "Delete the class?", confirm_label = "Delete",
    on_confirm = action!(delete), "Students keep their work."]
ContextMenu![target = Card![...], DropdownItem![href = "/rename", "Rename"]]"#,
        ),
        p![
            "Menus and popovers are the browser's own ",
            code!["popover"],
            " elements placed with CSS anchor positioning, so they open, close on a click outside or ",
            code!["Escape"],
            " and return focus with no script at all; the component script adds arrow keys, ",
            code!["Home"],
            "/",
            code!["End"],
            " and type-ahead, and positions them where anchor positioning is missing. Drawers and confirm dialogs are ",
            code!["<dialog>"],
            " elements opened with ",
            code!["Press::open_modal"],
            ", which also renders ",
            code!["commandfor"],
            " so browsers with invoker commands open them without JavaScript. Only the context menu needs the script; without it the browser's own menu appears.",
        ],
        h2("toasts", "Toaster and CommandPalette"),
        rust(
            r#"// Once, in the root layout. A server flash message becomes a toast with no script:
Toaster![placement = ToastPlacement::BottomRight,
    flash.map(|m| Toast![color = Color::Success, title = "Saved", duration = 5000, m])]

Button![on_press = Press::toast("Copied"), "Copy"]        // from a press
// From your own scripts: nextRust.toast({ title: "Saved", message: "…", color: "success", duration: 5000 })

CommandPalette![id = "commands", hotkey = "k",             // ⌘K / Ctrl K
    CommandGroup![title = "Pages", CommandItem![href = "/docs", keywords = "guide manual", "Documentation"]],
    CommandGroup![title = "Actions", CommandItem![on_press = action!(sign_out), shortcut = "⇧Q", "Sign out"]],
]"#,
        ),
        p![
            "Toasts rendered by the server fade out on their own, with CSS alone. The palette filters as you type and moves with the arrow keys once the script is there; without it the dialog still lists every command as a link.",
        ],
        h2("more-fields", "More form controls"),
        rust(
            r##"SearchInput![action = "/search", value = query, shortcut = "⌘K"]   // a whole GET form, name = "q"
Combobox![label = "Country", name = "country", options = ["India", "Germany"]]   // <input list> + <datalist>
TagsInput![label = "Topics", name = "topics", values = ["rust"], max = 5]        // submits "rust, web"
NumberInput![label = "Seats", name = "seats", value = "3", min = 1.0, max = 10.0]
Slider![label = "Volume", name = "volume", value = 40.0, unit = "%"]
PinInput![label = "Code", name = "code", length = 6]                             // one input: paste and SMS autofill work
Rating![name = "stars", label = "Your rating", value = 4.0]   Rating![value = 4.5, readonly = true]
SegmentedControl![name = "view", value = "week", label = "View", [("day", "Day"), ("week", "Week")]]
FileUpload![name = "photo", label = "Photo", accept = "image/*", hint = "PNG or JPG, up to 5 MB"]
TimePicker![label = "Starts at", name = "start", value = "09:30", step = 900]
ColorPicker![label = "Brand color", name = "brand", value = "#7828c8"]
DateRangePicker![label = "Stay", name = "stay", start = "2026-09-01", end = "2026-09-05"]  // stay_start, stay_end

FormField![label = "Avatar", name = "avatar", control_id = "avatar", any_control]   // label + errors around anything
FormSection![title = "Profile", description = "Shown on your public page.", fields…]
FormActions![justify = Justify::Between, bordered = true, buttons…]"##,
        ),
        p![
            "Every one of these is a native control underneath, so they submit, validate and autofill like the browser's own, and they keep working with JavaScript off: the combobox falls back to a ",
            code!["<datalist>"],
            ", the tags field to comma separated text, the rating to radio buttons, the pin field to one text input. Like the other fields, those with a ",
            code!["name"],
            " render an error slot that server-action validation errors land in.",
        ],
        h2("sections", "Page sections"),
        rust(
            r#"Hero![eyebrow = "New", title = "Ship faster", subtitle = "One binary.", glow = true, split = true,
    actions = Button![size = Size::Lg, href = "/signup", "Get started"], media = img![src(shot), alt("")]]
Section![id = "pricing", eyebrow = "Pricing", title = "Simple plans", muted = true, children]
FeatureGrid![cols = 3, Feature![icon = icons::Zap().size(20), title = "Fast", card = true, "Renders in microseconds."]]
PricingTable![PricingCard![name = "Pro", price = "$19", period = "/month", featured = true, badge = "Popular",
    cta = Button![full_width = true, "Choose Pro"], PricingFeature!["Unlimited projects"], PricingFeature![included = false, "SSO"]]]
Testimonial![name = "Ada Lovelace", role = "CTO", rating = 5.0, "It just works."]
LogoCloud![title = "Trusted by teams at", img![src("/acme.svg"), alt("Acme")], ...]
Cta![title = "Ready to build?", description = "Start free.", split = true, Button![href = "/signup", "Get started"]]
Faq![structured_data = true, [("Is it free?", "Yes, MIT or Apache-2.0.")]]        // accordion + FAQPage JSON-LD
Carousel![label = "Screenshots", per_view = 3, CarouselItem![...], ...]           // CSS scroll snapping
Marquee![speed = 30, span!["Acme"], span!["Globex"]]   Bento![cols = 3, BentoItem![col_span = 2, title = "…", ...]]
Timeline![TimelineItem![title = "Ordered", time = "09:12", color = Color::Success, "Payment received."]]
Footer![brand = strong!["Acme"], description = "Tools for teams.", bottom = "© 2026 Acme",
    FooterColumn![title = "Product", FooterLink![href = "/pricing", "Pricing"]]]"#,
        ),
        p![
            "A landing page can be written with these alone. The carousel scrolls with touch, the trackpad and the arrow keys through CSS scroll snapping (the previous/next buttons are the only part that needs the script), and the marquee stands still under ",
            code!["prefers-reduced-motion"],
            ", where it scrolls by hand instead.",
        ],
        h2("layout-helpers", "Pagination and layout helpers"),
        rust(
            r#"Pagination![page = 3, total = 20, href = "/posts?page={page}"]     // plain links, rel="prev"/"next"
AspectRatio![ratio = "16 / 9", img![src(cover), alt("")]]
ScrollArea![max_height = "20rem", label = "Changelog", children]
Spacer![]                        // takes the space left in a Stack
Spacer![size = 8]                // a fixed 2rem gap
Show![above = Breakpoint::Md, children]   Hide![above = Breakpoint::Md, children]
Sticky![top = "0", blurred = true, bordered = true, Navbar![...]]"#,
        ),
        h2("data-display", "Lists, code, callouts and tables"),
        rust(
            r#"List![divided = true, bordered = true,
    ListItem![title = "Ada Lovelace", description = "Admin", href = "/u/ada",
        start_content = Avatar![name = "Ada Lovelace", status = AvatarStatus::Online], end_content = Chip![size = Size::Sm, "Owner"]]]
DescriptionList![horizontal = true, DescriptionItem![term = "Plan", "Pro"]]      // or cols = 3
ChipGroup![label = "Topics", Chip![...], Chip![...]]
Code!["cargo run"]   CodeBlock![language = "rust", title = "src/main.rs", copy = true, source]
Callout![kind = CalloutKind::Warning, "Back up first."]                          // Note, Tip, Important, Warning, Danger
Banner![href = "/blog/launch", link_label = "Read more", persist = "launch-2026", "Version 2 is out."]
Figure![src = "/team.jpg", alt = "The team", caption = "Berlin, 2026", ratio = "3 / 2"]
VideoEmbed![youtube = "VIDEO_ID", title = "Launch video"]        // only the poster loads until play
ResultBlock![status = ResultStatus::Success, title = "Payment received", description = "…", Button![...]]

// Sorting and selection are ordinary links and checkboxes, so the server does the work:
Table![header = fragment![TableSelectAll![],
        SortHeader![href = "/students?sort=name&dir=desc", direction = SortDirection::Asc, "Student"], th!["Score"]],
    each(rows, |r| tr![TableSelect![name = "ids", value = r.id], td![r.name.clone()], td![r.score.to_string()]]),
]"#,
        ),
        p![
            "The code block's copy button needs the script and the clipboard, and hides itself without them. A banner dismisses itself with a checkbox and ",
            code![":has()"],
            "; with ",
            code!["persist"],
            " the script remembers the dismissal for the next visits.",
        ],
        h2("charts", "Charts"),
        rust(
            r#"Sparkline![values = [3.0, 5.0, 4.0, 8.0], color = Color::Success, label = "Signups"]
BarChart![title = "Signups", data = [("Mon", 12.0), ("Tue", 30.0)], unit = "%"]
LineChart![title = "Revenue", labels = ["Jan", "Feb", "Mar"],
    series = [("2025", vec![10.0, 14.0, 12.0]), ("2026", vec![12.0, 18.0, 25.0])]]
Donut![title = "Traffic", center_label = "visits", data = [("Direct", 60.0), ("Search", 40.0)]]
Meter![label = "Storage", value = 92.0, low = 60.0, high = 85.0, high_is_bad = true, value_label = "92 GB of 100 GB"]"#,
        ),
        p![
            "Charts are HTML and SVG rendered on the server: no canvas, no chart library, no JavaScript. Values appear on hover with CSS, the whole chart carries an accessible name listing its data, and the series colors are the CSS variables ",
            code!["--nr-chart-1"],
            " to ",
            code!["--nr-chart-8"],
            ", chosen to stay apart in both themes and for colorblind readers. More than eight series fold into one group.",
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
            ", the drawer, tab panels, modals, closable alerts) are driven by a small script, ",
            code!["/_next-rust/ui.js"],
            " (about 5 KB gzipped). It is added only to pages that render one of them. Without it, every component falls back to the native control, so forms still submit.",
        ],
        h3![id("ui-script-wasm"), "The same behaviour in Rust"],
        p![
            "That behaviour also exists as Rust, compiled into the one WebAssembly module a site shares. ",
            code![r#"[ui] script = "wasm""#],
            " serves it instead of the script: the page then carries a two-line loader rather than 27 KB of \
             JavaScript, and the components' logic is in the module.",
        ],
        pre![code![
            class("language-toml"),
            r#"# next-rust.toml
[ui]
script = "wasm"        # "js" (the default) serves /_next-rust/ui.js

[build]
wasm_islands = true    # the module is compiled from the islands/ crate"#
        ]],
        p![
            "The markup, the stylesheet, the ARIA and the keyboard contract are identical either way, and every \
             component still works with no script at all — so this is a delivery choice, not a behaviour one. It \
             needs an ",
            code!["islands/"],
            " crate beside ",
            code!["next-rust.toml"],
            " whose ",
            code!["islands!"],
            " names ",
            code!["start: next_rust_ui_wasm::boot"],
            "; ",
            code!["examples/ui"],
            " has a working one.",
        ],
        p![
            strong!["It costs bytes."],
            " Measured on the component showcase: the module is about 31 KB brotli against 7.6 KB for the \
             minified script, and it needs the 5.4 KB shim the islands already load. Take this path because you \
             want the behaviour in Rust, or because the site already ships a WebAssembly module and the kit can \
             ride along in it — not to make the page smaller.",
        ],
        p![
            "Two differences worth knowing. ",
            code!["nextRust.toast(…)"],
            " is published by the shim as a forwarder, so it raises the toast but returns nothing, where the \
             script returned the element. And release builds cannot shorten the names a module knows — a wasm \
             data section is not rewritable the way script text is — so the components' classes and ",
            code!["data-"],
            " attributes keep their full names in the HTML and the CSS.",
        ],
        h2("check-it-worked", "How to check it worked"),
        ul![
            li![
                "Open the ",
                a![href("/components"), "gallery"],
                ", press Preview on the component you want, and compare it with your page.",
            ],
            li!["Turn JavaScript off in the browser's developer tools and reload. Every form must still submit, and \
                 selects and date pickers must show the browser's own controls.",],
            li!["Tab through the page with the keyboard. Every control should be reachable and show a focus ring.",],
            li![
                "Run ",
                code!["next-rust build"],
                " and ",
                code!["next-rust start"],
                " once before deploying: component class names (",
                code!["nr-*"],
                ") are shortened in release builds, which is where a stylesheet or script that targets them by name \
                 stops matching.",
            ],
        ],
        h2("common-mistakes", "Common mistakes"),
        ul![
            li![
                strong!["Forgetting the import."],
                " ",
                code!["use next_rust::ui::*;"],
                " is separate from the prelude. Without it ",
                code!["Button!"],
                " is an unknown macro.",
            ],
            li![
                strong!["Copying a component's markup to restyle it."],
                " Add ",
                code!["class(..)"],
                " or set the ",
                code!["--nr-*"],
                " variables instead. Your classes always win over the defaults, and you keep the keyboard and \
                 accessibility behaviour.",
            ],
            li![
                strong!["Targeting ", code!["nr-*"], " classes from your own CSS or scripts."],
                " They are renamed in release builds. Give the component your own class or id.",
            ],
            li![
                strong!["A field without ", code!["name"], "."],
                " It is not submitted with the form and gets no slot for server-side validation errors.",
            ],
            li![
                strong!["A button inside a form that should not submit it."],
                " ",
                code!["Button!"],
                " is ",
                code!["type=\"button\""],
                " unless you set ",
                code!["submit = true"],
                "; a plain ",
                code!["button!"],
                " element is the opposite and submits by default, as in HTML.",
            ],
            li![
                strong!["Expecting ", code!["on_press"], " to run Rust in the browser."],
                " Views run on the server. ",
                code!["on_press"],
                " describes what the browser should do: call a server action, navigate, open a modal, emit an event.",
            ],
            li![strong!["Several ", code!["Toaster!"], "s."], " Render it once, in the root layout.",],
            li![
                strong!["A modal that never opens."],
                " The ",
                code!["id"],
                " of the ",
                code!["Modal!"],
                " and the string in ",
                code!["Press::open_modal(..)"],
                " must be identical.",
            ],
        ],
    ]
}
