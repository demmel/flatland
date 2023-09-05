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

    let update_image = |state: &State| window.set_image("image", state.to_image());
    let update_state = |state: &mut State| state.update();

    update_image(&state)?;

    let window_events = window.event_channel()?;
    loop {
        match window_events.try_recv() {
            Ok(WindowEvent::KeyboardInput(event)) => {
                if !event.input.state.is_pressed() {
                    continue;
                }
                match event.input.key_code {
                    Some(VirtualKeyCode::Escape) => return Ok(()),
                    Some(VirtualKeyCode::Space) if !running => update_state(&mut state),
                    Some(VirtualKeyCode::S) => running = !running,
                    _ => continue,
                }
                update_image(&state)?;
            }
            Err(TryRecvError::Empty) if running => {
                update_state(&mut state);
                update_image(&state)?;
            }
            Err(TryRecvError::Disconnected) => return Ok(()),
            _ => continue,
        }
    }
}
