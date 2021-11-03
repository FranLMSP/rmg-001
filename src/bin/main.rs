use rmg_001::render::start_eventloop;
use rmg_001::emulator::Emulator;

fn main() -> std::io::Result<()> {
    start_eventloop();
    /* let mut emulator = Emulator::new();
    emulator.cpu_loop(); */
    Ok(())
}
