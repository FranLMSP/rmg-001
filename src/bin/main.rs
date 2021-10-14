use rust_boy::rom::ROM;
use rust_boy::console::Console;

fn main() -> std::io::Result<()> {
    /* let myrom = ROM::load_file("roms/cpu_instrs.gb".to_string())?;
    myrom.print_content(); */
    let mut console = Console::new();
    console.cpu_run();
    Ok(())
}
