use next_rust::icons;
use next_rust::prelude::*;
use next_rust::ui::*;

/// A landing page made only of kit components.
pub fn Page() -> impl View {
    fragment![
        // `persist` makes the dismissal stick: closing it is a checkbox and
        // `:has()` with no script at all, and the browser layer remembers it.
        Banner![
            persist = "charts-2026",
            href = "/data",
            link_label = "See the charts",
            "New: server-rendered charts, no JavaScript."
        ],
        Hero![
            eyebrow = "Next Rust UI",
            title = "Attractive sites with the framework alone",
            subtitle = "Heroes, pricing, testimonials, carousels and footers: written in Rust, shipped as HTML and CSS.",
            glow = true,
            actions = fragment![
                Button![size = Size::Lg, href = "/", end_content = icons::ArrowRight().size(18), "Browse components"],
                Button![size = Size::Lg, variant = Variant::Bordered, color = Color::Default, href = "/forms", "See the forms"],
            ],
        ],
        Section![
            compact = true,
            LogoCloud![title = "Trusted by teams at", span!["Acme"], span!["Globex"], span!["Initech"], span!["Umbrella"], span!["Hooli"]],
        ],
        Section![
            eyebrow = "Features",
            title = "Everything a marketing site needs",
            subtitle = "Each block is a builder with sensible defaults.",
            muted = true,
            FeatureGrid![
                cols = 3,
                Feature![icon = icons::Zap().size(20), title = "No JavaScript required", card = true, "Details, dialogs, popovers and scroll snapping do the work."],
                Feature![icon = icons::Palette().size(20), color = Color::Secondary, title = "Themeable", card = true, "A handful of CSS variables restyle every component."],
                Feature![icon = icons::Accessibility().size(20), color = Color::Success, title = "Accessible", card = true, "Roles, labels, focus rings and reduced motion are built in."],
            ],
        ],
        Section![
            title = "A bento grid",
            Bento![
                cols = 3,
                BentoItem![col_span = 2, row_span = 2, title = "Charts on the server", description = "SVG and CSS, themed in light and dark.",
                    LineChart![labels = ["Jan", "Feb", "Mar", "Apr", "May", "Jun"], values = [12.0, 18.0, 15.0, 24.0, 30.0, 41.0], height = 160]],
                BentoItem![title = "Tiny CSS", description = "Pages receive only the rules of what they render."],
                BentoItem![title = "One binary", description = "No Node, no bundler."],
                BentoItem![col_span = 3, title = "Marquee", Marquee![speed = 25, span!["Fast"], span!["Typed"], span!["Accessible"], span!["Themeable"], span!["Server rendered"]]],
            ],
        ],
        Section![
            eyebrow = "Pricing",
            title = "Simple plans",
            muted = true,
            PricingTable![
                PricingCard![name = "Hobby", price = "Free", description = "For side projects.",
                    cta = Button![full_width = true, variant = Variant::Bordered, color = Color::Default, "Start"],
                    PricingFeature!["1 project"], PricingFeature!["Community support"], PricingFeature![included = false, "SSO"]],
                PricingCard![name = "Pro", price = "$19", period = "/month", featured = true, badge = "Popular", description = "For growing teams.",
                    cta = Button![full_width = true, "Choose Pro"],
                    PricingFeature!["Unlimited projects"], PricingFeature!["Priority support"], PricingFeature![included = false, "SSO"]],
                PricingCard![name = "Enterprise", price = "Custom", description = "For large organizations.",
                    cta = Button![full_width = true, variant = Variant::Bordered, color = Color::Default, "Contact us"],
                    PricingFeature!["Everything in Pro"], PricingFeature!["Dedicated support"], PricingFeature!["SSO"]],
            ],
        ],
        Section![
            title = "Loved by developers",
            Carousel![
                label = "Testimonials",
                per_view = 3,
                CarouselItem![Testimonial![name = "Ada Lovelace", role = "CTO, Analytical", rating = 5.0, "We deleted our front-end build and nobody noticed."]],
                CarouselItem![Testimonial![name = "Grace Hopper", role = "Rear Admiral", rating = 5.0, "It compiles, it ships. That is the whole workflow."]],
                CarouselItem![Testimonial![name = "Alan Turing", role = "Researcher", rating = 4.5, "The pages load before I finish clicking."]],
                CarouselItem![Testimonial![name = "Linus T.", role = "Maintainer", rating = 4.0, "Finally a UI kit that does not need a package manager."]],
                CarouselItem![Testimonial![name = "Margaret H.", role = "Software lead", rating = 5.0, "Accessible by default, which is how it should be."]],
            ],
        ],
        Section![
            title = "Questions",
            width = Width::Md,
            muted = true,
            Faq![structured_data = true, [
                ("Do these need JavaScript?", "No. A small script adds keyboard polish where the platform has gaps."),
                ("Can I restyle them?", "Yes: CSS variables for the theme, your own classes win over the defaults."),
                ("Dark mode?", "Follows the system, or force it with class=\"dark\"."),
            ]],
        ],
        Section![
            Cta![title = "Ready to build?", description = "One crate, one binary.", split = true,
                Button![size = Size::Lg, color = Color::Default, variant = Variant::Solid, href = "/", "Get started"]],
        ],
        Footer![
            brand = strong!["Next Rust UI"],
            description = "Components for sites and apps, rendered on the server.",
            bottom = "© 2026 Next Rust",
            bottom_end = fragment![a![href("/plain"), "Privacy"], a![href("/plain"), "Terms"]],
            FooterColumn![title = "Product", FooterLink![href = "/", "Components"], FooterLink![href = "/forms", "Forms"], FooterLink![href = "/data", "Data"]],
            FooterColumn![title = "Resources", FooterLink![href = "/overlays", "Overlays"], FooterLink![href = "https://lucide.dev", external = true, "Icons"]],
            FooterColumn![title = "Company", FooterLink![href = "/plain", "About"], FooterLink![href = "/plain", "Contact"]],
        ],
    ]
}
