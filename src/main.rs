mod app;
mod cpu;
mod font;
mod config;
mod cli;
mod buzzer;

use std::io::Read;

use app::App;
use buzzer::Buzzer;
use cli::Cli;
use config::Config;
use cpu::Cpu;
use winit::{
    error::EventLoopError,
    event_loop::{ControlFlow, EventLoop},
    platform::run_on_demand::EventLoopExtRunOnDemand,
};

fn main() -> Result<(), EventLoopError> {
    let cli = match Cli::new() {
        Ok(cli) => cli,
        Err(e) => {
            eprintln!("{}", e);
            std::process::exit(1);
        }
    };

    // Read game binary
    // TEMP: For now you should guarantee that you will specify at least one game!
    let mut file = std::fs::File::open(&cli.game_paths.as_ref().expect("Specify the path to the game")[0]).unwrap();
    let mut game = vec![];
    file.read_to_end(&mut game).unwrap();

    // Init buzzer
    let buzzer = match Buzzer::new() {
        Ok(mut buzzer) => {
            buzzer.set_muted(cli.mute);
            Some(buzzer)
        }
        Err(e) => {
            eprintln!("Buzzer error: {}", e);
            eprintln!("Ignored. You won't be able to listen to biiip :(");
            None
        }
    };

    // Init config
    let config = Config::new(cli);

    // Init cpu
    let mut cpu = Cpu::default();
    cpu.load(&game);

    let mut event_loop = EventLoop::new()?;
    event_loop.set_control_flow(ControlFlow::Wait);

    let mut win = App::new(config, cpu, buzzer);
    
    // If i use `run_app`, a segmentation fault occurs after closing the app
    // but with `run_app_on_demand` it works well (please just tell me, that i am stupid)
    event_loop.run_app_on_demand(&mut win)
}
