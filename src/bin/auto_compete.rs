use std::{cmp::Reverse, error::Error, fs::File};

use ordered_float::OrderedFloat;
use rand::prelude::*;
use rayon::prelude::*;

use roots::{
    grid::GridLike,
    simulation::{config::Config, Element, State},
};

#[show_image::main]
fn main() -> Result<(), Box<dyn Error>> {
    let mut rng = thread_rng();

    let mut configs: Vec<_> = (0..32)
        .map(|i| {
            if i == 0 {
                if let Ok(f) = File::open("winner.json") {
                    println!("Loaded winner to position 0");
                    return serde_json::from_reader(f).unwrap();
                }
            }
            Config::gen(&mut rng)
        })
        .collect();

    loop {
        let mut scores: Vec<(_, f32)> = configs
            .par_iter()
            .map(|c| {
                let mut state = State::gen(c.clone(), 64, 64);
                (
                    c,
                    (0..1000)
                        .map(|_| {
                            state.update();
                            score_state(&state)
                        })
                        .sum(),
                )
            })
            .collect();

        scores.sort_unstable_by_key(|(_, s)| Reverse(OrderedFloat(*s)));

        println!("Winner: {:?}", scores[0].1);

        serde_json::to_writer_pretty(File::create("winner.json").unwrap(), &scores[0].0).unwrap();

        let mut rng = thread_rng();
        let mut new_configs = Vec::with_capacity(scores.len());

        new_configs.push(scores[0].0.clone());
        new_configs.push(scores[0].0.clone().mutate());
        for _ in 0..((scores.len() - 2) / 2) {
            new_configs.push(Config::gen(&mut rng));
        }
        for _ in 0..((scores.len() - 2) / 2) {
            let mut competitors = scores[0..(scores.len() / 2)].choose_multiple(&mut rng, 2);
            let a = competitors.next().unwrap().0;
            let b = competitors.next().unwrap().0;
            let c = a.crossover(b);
            new_configs.push(c);
        }

        configs = new_configs;
    }
}

fn score_state(state: &State) -> f32 {
    let element_count = state.elements.iter().count();

    let avg_saturation =
        state.elements.iter().map(|t| t.saturation().0).sum::<f32>() / element_count as f32;
    let saturation_score = state
        .elements
        .iter()
        .map(|t| (t.saturation() - avg_saturation).abs())
        .sum::<f32>()
        / element_count as f32;

    let position_score = state
        .elements
        .enumerate()
        .map(|(_x, y, t)| match t.element() {
            Element::Air => (state.elements.height() - y) as f32 / state.elements.height() as f32,
            Element::Soil => y as f32 / state.elements.height() as f32,
            Element::Water => {
                ((state.elements.height() / 2) as f32 - y as f32).abs()
                    / (state.elements.height() / 2) as f32
            }
        })
        .sum::<f32>()
        / element_count as f32;

    let one_third_count = (1.0 / 3.0) * element_count as f32;
    let distributsion_score = ((one_third_count
        - (state
            .elements
            .iter()
            .filter(|t| t.element() == Element::Air)
            .count() as f32
            - one_third_count)
            .abs())
        / one_third_count
        + (one_third_count
            - (state
                .elements
                .iter()
                .filter(|t| t.element() == Element::Water)
                .count() as f32
                - one_third_count)
                .abs())
            / one_third_count)
        / 2.0;

    let pattern_score = state
        .elements
        .windows(3)
        .map(|w| {
            use Element::*;

            match (
                w.get(-1, -1).map(|t| t.element()),
                w.get(0, -1).map(|t| t.element()),
                w.get(1, -1).map(|t| t.element()),
                w.get(-1, 0).map(|t| t.element()),
                w.get(0, 0).map(|t| t.element()),
                w.get(1, 0).map(|t| t.element()),
                w.get(-1, 1).map(|t| t.element()),
                w.get(0, 1).map(|t| t.element()),
                w.get(1, 1).map(|t| t.element()),
            ) {
                (
                    Some(Water),
                    Some(Water),
                    Some(Water | Air),
                    Some(Air),
                    Some(Air),
                    Some(Air),
                    Some(Water),
                    Some(Water),
                    Some(Water | Air),
                )
                | (
                    Some(Water | Air),
                    Some(Water),
                    Some(Water),
                    Some(Air),
                    Some(Air),
                    Some(Air),
                    Some(Water | Air),
                    Some(Water),
                    Some(Water),
                ) => 0.0,
                _ => 1.0,
            }
        })
        .sum::<f32>()
        / element_count as f32;

    saturation_score + position_score + distributsion_score + pattern_score
}
