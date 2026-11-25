//! Prints the skills the Core finds in your real home directory: `cargo run --example list_skills`.

fn main() {
    let home = std::env::var_os("HOME").expect("HOME is set");
    for skill in skills_pad_core::skills::scan(std::path::Path::new(&home)) {
        println!(
            "{:<40} {}",
            skill.id,
            skill.description.chars().take(70).collect::<String>()
        );
    }
}
