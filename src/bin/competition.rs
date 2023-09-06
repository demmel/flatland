use std::{cmp::Ordering, error::Error, fs::File, sync::mpsc::TryRecvError};

use image::{GenericImage, RgbImage};
use rand::prelude::*;
use show_image::{
    create_window,
    event::{VirtualKeyCode, WindowEvent},
    WindowOptions,
};

use roots::simulation::{config::Config, State};

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

fn setup(competition_config: &CompeitionConfig) -> (CompetingConfigs, SelectedConfigs) {
    let mut rng = thread_rng();

    let competitors = CompetingConfigs::new(
        (0..competition_config.population_size)
            .map(|i| {
                if i == 0 {
                    if let Ok(f) = File::open("winner.json") {
                        return serde_json::from_reader(f).unwrap();
                    }
                }
                Config::gen(&mut rng)
            })
            .collect(),
    );

    let selected = SelectedConfigs(vec![competitors.current(), competitors.pivot()]);
    (competitors, selected)
}

fn rank_selected(
    ordering: Ordering,
    competitors: &mut CompetingConfigs,
    selected_competitors: &mut SelectedConfigs,
) {
    if !competitors.rank(ordering) {
        let competitors_inner = competitors.competitors();

        serde_json::to_writer_pretty(File::create("winnder.json").unwrap(), &competitors_inner[0])
            .unwrap();

        let mut rng = thread_rng();
        let mut new_competitors = Vec::with_capacity(competitors_inner.len());

        new_competitors.push(competitors_inner[0].clone());
        for _ in 0..((competitors_inner.len() - 1) / 2) {
            let mut competitors =
                competitors_inner[0..(competitors_inner.len() / 2)].choose_multiple(&mut rng, 2);
            let a = competitors.next().unwrap();
            let b = competitors.next().unwrap();
            let c = a.crossover(b);
            new_competitors.push(c);
        }
        for _ in 0..((competitors_inner.len()) / 2) {
            new_competitors.push(Config::gen(&mut rng));
        }

        *competitors = CompetingConfigs::new(new_competitors);
    }

    *selected_competitors = SelectedConfigs(vec![competitors.current(), competitors.pivot()]);
}

struct SelectedConfigs(Vec<usize>);

struct CompetingConfigs {
    i: usize,
    j: usize,
    remaining: Vec<(usize, usize)>,
    competitors: Vec<Config>,
}

impl CompetingConfigs {
    fn new(competitors: Vec<Config>) -> Self {
        Self {
            i: 0,
            j: 0,
            remaining: vec![(0, competitors.len() - 1)],
            competitors,
        }
    }

    fn current(&self) -> usize {
        self.j
    }

    fn pivot(&self) -> usize {
        self.remaining.last().unwrap().1
    }

    fn competitors(&self) -> &[Config] {
        &self.competitors
    }

    fn rank(&mut self, ordering: Ordering) -> bool {
        if matches!(ordering, Ordering::Greater) {
            self.competitors.swap(self.i, self.j);
            self.i += 1;
        }
        self.j += 1;
        if self.j == self.pivot() {
            let (start, end) = self.remaining.pop().unwrap();
            self.competitors.swap(self.i, end);
            if start + 1 < self.i {
                self.remaining.push((start, self.i - 1));
            }
            if self.i + 1 < end {
                self.remaining.push((self.i + 1, end));
            }
            if let Some((start, end)) = self.remaining.last() {
                self.i = *start;
                self.j = *start;
                let mid = ((end + 1) - start) / 2;
                self.competitors.swap(mid, *end);
                true
            } else {
                false
            }
        } else {
            true
        }
    }
}
