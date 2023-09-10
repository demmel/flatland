use roots::simulation::{config::Config, State};

fn main() {
    let mut state: State = State::gen(Config::default(), 320, 180);
    loop {
        state.update();
    }
}
