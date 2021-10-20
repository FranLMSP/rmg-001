use yargbe::rom::ROM;
use yargbe::console::Console;

fn main() -> std::io::Result<()> {
    let mut console = Console::new();
    console.cpu_run();
    // println!("{:02X}", 0x0FFF + 1);
    Ok(())
}
