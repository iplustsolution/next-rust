use next_rust::prelude::*;
use next_rust::ui::*;

pub fn metadata() -> Metadata {
    Metadata::new().title("Components").title_template("%s | Next Rust UI")
}

pub fn Layout(children: Children) -> impl View {
    AppShell![
        navbar = Navbar![
            menu_toggle = true,
            bordered = true,
            brand = a![href("/"), "Next Rust UI"],
            end_content = Stack![
                row = true,
                gap = 3,
                align = Align::Center,
                Button![href = "/plain", variant = Variant::Flat, size = Size::Sm, "Plain page"],
                Avatar![name = "Ada Lovelace", size = Size::Sm, color = Color::Secondary],
            ],
            NavbarItem![href = "/", "Components"],
            NavbarItem![href = "/plain", "Plain"],
        ],
        sidebar = Sidebar![
            title = "Getting started",
            SidebarItem![href = "/", "All components"],
            SidebarItem![href = "/plain", "A plain page"],
        ],
        children,
    ]
}
