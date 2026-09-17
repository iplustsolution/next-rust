fn main() {
    next_rust::run_with(
        next_rust::App::new(next_rust_website::routes()).get("/logo.svg", next_rust_website::logo::serve),
    );
}
