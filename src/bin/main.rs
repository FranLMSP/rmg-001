use rust_boy::rom::ROM;
use rust_boy::console::Console;

fn main() -> std::io::Result<()> {
    /* let myrom = ROM::load_file("roms/cpu_instrs.gb".to_string())?;
    myrom.print_content(); */
    let mut console = Console::new();
    console.cpu_run();
    /* let val: u8 = 0b00000010;
    println!("{:08b}", val.rotate_left(7)); */
    Ok(())
}
