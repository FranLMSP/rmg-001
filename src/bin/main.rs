use yargbe::rom::ROM;
use yargbe::console::Console;

fn main() -> std::io::Result<()> {
    /* let mut console = Console::new();
    console.cpu_run(); */
    let val: u8 = 0b11110000;
    println!("{:08b}", !val);
    Ok(())
}
