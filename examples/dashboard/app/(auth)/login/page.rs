use next_rust::prelude::*;

#[derive(serde::Deserialize)]
pub struct LoginInput {
    username: String,
}

#[server_action]
pub async fn login(ctx: ActionContext, input: LoginInput) -> Result<()> {
    let username = input.username.trim();
    if username.is_empty() {
        return Err(Error::validation([("username", "Please enter a username")]));
    }
    ctx.cookies.set(Cookie::encoded("user", username));
    Err(redirect("/dashboard"))
}

pub fn metadata() -> Metadata {
    Metadata::new().title("Sign in")
}

pub fn Page(form: FormState) -> impl View {
    form![
        action!(login),
        label![r#for("username"), "Username"],
        input![id("username"), name("username"), value(form.value("username")), required(true)],
        span![data("nr-error", "username"), form.error("username").unwrap_or_default().to_owned()],
        button![r#type("submit"), "Sign in"],
    ]
}
