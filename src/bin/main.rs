use yargbe::rom::ROM;
use yargbe::console::Console;

fn main() -> std::io::Result<()> {
    let mut console = Console::new();
    console.cpu_run();
    Ok(())
}
