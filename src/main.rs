mod grid;
mod simulation;

use std::{error::Error, sync::mpsc::TryRecvError};

use show_image::{
    create_window,
    event::{VirtualKeyCode, WindowEvent},
    WindowOptions,
};

use crate::simulation::State;

#[show_image::main]
fn main() -> Result<(), Box<dyn Error>> {
    let mut state: State = State::gen(64, 64);
    let mut running: bool = false;

    let window = create_window(
        "",
        WindowOptions {
            fullscreen: true,
            ..Default::default()
        },
    )?;
    window.set_image("image", state.to_image())?;

    let window_events = window.event_channel()?;
    loop {
        match window_events.try_recv() {
            Ok(WindowEvent::KeyboardInput(event)) => {
                if !event.input.state.is_pressed() {
                    continue;
                }
                match event.input.key_code {
                    Some(VirtualKeyCode::Escape) => return Ok(()),
                    Some(VirtualKeyCode::Space) if !running => state.update(),
                    Some(VirtualKeyCode::S) => running = !running,
                    _ => continue,
                }
                window.set_image("image", state.to_image())?;
            }
            Err(TryRecvError::Empty) if running => {
                state.update();
                window.set_image("image", state.to_image())?;
            }
            Err(TryRecvError::Disconnected) => return Ok(()),
            _ => continue,
        }
    }
}
