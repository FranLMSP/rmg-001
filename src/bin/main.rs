use rmg_001::rom::ROM;
use rmg_001::console::Console;

fn main() -> std::io::Result<()> {
    let mut console = Console::new();
    console.cpu_run();
    Ok(())
}
