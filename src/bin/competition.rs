use std::{cmp::Ordering, error::Error, fs::File, sync::mpsc::TryRecvError};

use image::{GenericImage, RgbImage};
use rand::prelude::*;
use show_image::{
    create_window,
    event::{VirtualKeyCode, WindowEvent},
    WindowOptions,
};

use roots::{
    ranker::Ranker,
    simulation::{config::Config, State},
};

#[show_image::main]
fn main() -> Result<(), Box<dyn Error>> {
    let competition_config = CompeitionConfig {
        size: (64, 64),
        population_size: 8,
    };
    let (mut competitors, mut selected) = setup(&competition_config);
    let mut states = vec![
        State::gen(
            competitors.competitors()[selected.0[0]].clone(),
            competition_config.size.0,
            competition_config.size.1,
        ),
        State::gen(
            competitors.competitors()[selected.0[1]].clone(),
            competition_config.size.0,
            competition_config.size.1,
        ),
    ];

    let mut running: bool = false;

    let window = create_window(
        "",
        WindowOptions {
            fullscreen: true,
            ..Default::default()
        },
    )?;

    let update_image = |state: &[State]| -> Result<(), Box<dyn Error>> {
        let mut img = RgbImage::new(
            competition_config.size.0 as u32 * 2 + 1,
            competition_config.size.1 as u32,
        );

        for (i, s) in state.iter().map(|s| s.to_image()).enumerate() {
            img.sub_image(
                (competition_config.size.0 as u32 + 1) * i as u32,
                0,
                competition_config.size.0 as u32,
                competition_config.size.1 as u32,
            )
            .copy_from(&s, 0, 0)?;
        }

        window.set_image("image", img)?;

        Ok(())
    };
    let update_state = |states: &mut [State]| {
        for state in states {
            state.update();
        }
    };

    update_image(&states)?;

    let window_events = window.event_channel()?;
    loop {
        match window_events.try_recv() {
            Ok(WindowEvent::KeyboardInput(event)) => {
                if !event.input.state.is_pressed() {
                    continue;
                }
                match event.input.key_code {
                    Some(VirtualKeyCode::Escape) => return Ok(()),
                    Some(VirtualKeyCode::Space) if !running => update_state(&mut states),
                    Some(VirtualKeyCode::S) => running = !running,
                    Some(VirtualKeyCode::Left) => {
                        rank_selected(Ordering::Greater, &mut competitors, &mut selected);
                        states = vec![
                            State::gen(
                                competitors.competitors()[selected.0[0]].clone(),
                                competition_config.size.0,
                                competition_config.size.1,
                            ),
                            State::gen(
                                competitors.competitors()[selected.0[1]].clone(),
                                competition_config.size.0,
                                competition_config.size.1,
                            ),
                        ];
                    }
                    Some(VirtualKeyCode::Right) => {
                        rank_selected(Ordering::Less, &mut competitors, &mut selected);
                        states = vec![
                            State::gen(
                                competitors.competitors()[selected.0[0]].clone(),
                                competition_config.size.0,
                                competition_config.size.1,
                            ),
                            State::gen(
                                competitors.competitors()[selected.0[1]].clone(),
                                competition_config.size.0,
                                competition_config.size.1,
                            ),
                        ];
                    }
                    _ => continue,
                }
                update_image(&states)?;
            }
            Err(TryRecvError::Empty) if running => {
                update_state(&mut states);
                update_image(&states)?;
            }
            Err(TryRecvError::Disconnected) => return Ok(()),
            _ => continue,
        }
    }
}

#[derive(Clone)]
pub struct CompeitionConfig {
    pub size: (usize, usize),
    pub population_size: usize,
}

fn setup(competition_config: &CompeitionConfig) -> (Ranker<Config>, SelectedConfigs) {
    let mut rng = thread_rng();

    let competitors = Ranker::new(
        (0..competition_config.population_size)
            .map(|i| {
                if i == 0 {
                    if let Ok(f) = File::open("winner.json") {
                        println!("Loaded winner to position 0");
                        return serde_json::from_reader(f).unwrap();
                    }
                } else if i == 1 {
                    return Config::default();
                }
                Config::gen(&mut rng)
            })
            .collect(),
        competition_config.population_size / 2,
    );

    let selected = SelectedConfigs(vec![competitors.current(), competitors.pivot()]);
    (competitors, selected)
}

fn rank_selected(
    ordering: Ordering,
    competitors: &mut Ranker<Config>,
    selected_competitors: &mut SelectedConfigs,
) {
    if !competitors.rank(ordering) {
        let competitors_inner = competitors.competitors();

        println!("Winner: {:?}", competitors_inner[0]);

        serde_json::to_writer_pretty(File::create("winner.json").unwrap(), &competitors_inner[0])
            .unwrap();

        let mut rng = thread_rng();
        let mut new_competitors = Vec::with_capacity(competitors_inner.len());

        new_competitors.push(competitors_inner[0].clone());
        new_competitors.push(competitors_inner[0].clone().mutate(0.1));
        for _ in 0..((competitors_inner.len() - 2) / 2) {
            new_competitors.push(Config::gen(&mut rng));
        }
        // Put the crossovers at the end because they likely make the best pivots
        for _ in 0..((competitors_inner.len() - 2) / 2) {
            let mut competitors =
                competitors_inner[0..(competitors_inner.len() / 2)].choose_multiple(&mut rng, 2);
            let a = competitors.next().unwrap();
            let b = competitors.next().unwrap();
            let c = a.crossover(b);
            new_competitors.push(c);
        }

        *competitors = Ranker::new(new_competitors, competitors_inner.len() / 2);
    }

    *selected_competitors = SelectedConfigs(vec![competitors.current(), competitors.pivot()]);
    println!("{selected_competitors:?}");
}

#[derive(Debug)]
struct SelectedConfigs(Vec<usize>);
