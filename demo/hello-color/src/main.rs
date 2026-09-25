use colored::Colorize;

fn main() {
    let border = "════════════════════".bright_magenta();
    let greeting = "  Hello, World!  ".bold().green();
    println!("{}", border);
    println!("{}", greeting);
    println!("{}", border);
}
