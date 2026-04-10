use colored::{Color, Colorize};

pub const BANNER: &str = r"
                _                     
    ____  _____(_)________ ___  ____ _
   / __ \/ ___/ / ___/ __ `__ \/ __ `/
  / /_/ / /  / /__  / / / / / / /_/ / 
 / .___/_/  /_/____/_/ /_/ /_/\__,_/  
/_/                                   ";

pub fn print_banner() {
    let colors = [
        Color::BrightRed,
        Color::BrightYellow,
        Color::BrightGreen,
        Color::BrightCyan,
        Color::BrightBlue,
        Color::BrightMagenta,
    ];

    for (i, line) in BANNER.lines().enumerate() {
        let color = colors[i % colors.len()];
        println!("{}", line.color(color).bold());
    }
    println!("{}\n", " 💎🌈 L4 Proxy Multiplexer ".dimmed());
}

pub fn print_info(protocol: &str, bind: &str, port: u16) {
    println!(
        "{} {} {}:{}",
        "[!]".cyan(),
        "Listening on".bright_black(),
        bind.green(),
        port.to_string().yellow()
    );
    println!("{} Protocol: {}\n", "[!]".cyan(), protocol.magenta());
}

pub fn print_success(msg: &str) {
    println!("{} {}", "[!]".green().bold(), msg.bright_green());
}

pub fn print_config_guide() {
    println!("{}", "Configuration not found.".yellow().bold());
    println!(
        "Use {} to generate a default config file.",
        "prisma init".green()
    );

    println!("\n{}", "Manual Configuration (prisma.toml):".bold());
    println!(
        "{}",
        r#"
[server]
bind = "0.0.0.0"
port = 8080

[[protocols]]
name = "my_protocol"
patterns = ["\x05\x01\x00", "PROTOCOL_HEADER"]
forward_to = "127.0.0.1:9000"
transport = "tcp"

[[protocols]]
name = "http"
forward_to = "127.0.0.1:3000"
"#
        .dimmed()
    );

    println!("\n{}", "Command line overrides:".bold());
    println!("  prisma --forward \"name=addr\"");
}

pub fn print_error(msg: &str) {
    eprintln!("{} {}", "[x]".red().bold(), msg.bright_red());
}

pub fn print_error_details(details: &str) {
    eprintln!("{} {}", "    Details:".dimmed(), details.bright_black());
}

pub fn print_resolve_error(addr: &str, error: &str) {
    print_error(&format!(
        "The bind address '{addr}' is invalid or could not be resolved."
    ));
    print_error_details(error);
}
