use rust_boy::rom::ROM;
use rust_boy::console::Console;

fn main() -> std::io::Result<()> {
    let mut console = Console::new();
    console.cpu_run();
    Ok(())
}
