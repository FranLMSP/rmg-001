use rust_boy::rom::ROM;

fn main() -> std::io::Result<()> {
    let myrom = ROM::load_file("roms/cpu_instrs.gb".to_string())?;
    myrom.print_content();
    Ok(())
}
