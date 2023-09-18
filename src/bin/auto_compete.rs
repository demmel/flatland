use std::{collections::BinaryHeap, error::Error, fs::File};

use genetic::{Crossover, Gen, Mutate};
use ordered_float::OrderedFloat;
use rand::prelude::*;
use rayon::prelude::*;

use flatland::{
    grid::GridLike,
    simulation::{config::Config, Element, State},
};
use serde::{Deserialize, Serialize};
use statistical::{mean, standard_deviation};

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
            } else if i == 1 {
                return Config::default();
            }
            Config::gen(&mut rng)
        })
        .collect();

    let mut elites = if let Ok(file) = File::open("elites.json") {
        serde_json::from_reader(file).unwrap()
    } else {
        BinaryHeap::new()
    };
    loop {
        let config_scores = score_configs(&configs);
        let scores: Vec<_> = config_scores
            .iter()
            .take(config_scores.len() / 2)
            .map(|x| x.1 .0)
            .collect();

        println!("Winner: {:?}", scores[0]);
        serde_json::to_writer_pretty(File::create("winner.json").unwrap(), &config_scores[0].0)
            .unwrap();

        let mu = mean(&scores);
        let sigma = standard_deviation(&scores, Some(mu));
        println!("Top 50% - Mu: {mu} Sigma: {sigma}");

        if sigma / mu < 0.05 {
            println!("Diversify");
            configs = next_diversify_generation(&mut elites, &config_scores);
        } else {
            println!("Incremental");
            configs = next_incremental_generation(&config_scores);
        }
    }
}

#[derive(Clone, Serialize, Deserialize)]
struct ConfigScore(Config, OrderedFloat<f32>);

impl PartialEq for ConfigScore {
    fn eq(&self, other: &Self) -> bool {
        self.0 == other.0
    }
}

impl Eq for ConfigScore {}

impl PartialOrd for ConfigScore {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        other.1.partial_cmp(&self.1)
    }
}

impl Ord for ConfigScore {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.partial_cmp(other).unwrap()
    }
}

fn next_diversify_generation(
    elites: &mut BinaryHeap<ConfigScore>,
    config_scores: &[ConfigScore],
) -> Vec<Config> {
    elites.push(config_scores[0].clone());
    if elites.len() > 16 {
        elites.pop();
    }
    serde_json::to_writer_pretty(File::create("elites.json").unwrap(), &elites).unwrap();
    let mut rng: ThreadRng = thread_rng();
    let mut new_configs = Vec::with_capacity(config_scores.len());

    if elites.len() < 4 {
        for _ in 0..(config_scores.len()) {
            new_configs.push(Config::gen(&mut rng));
        }
    } else {
        let sorted_elites = elites.clone().into_sorted_vec();
        new_configs.extend(
            sorted_elites
                .choose_multiple_weighted(&mut rng, 2, |cs| cs.1 .0)
                .unwrap()
                .map(|x| x.0.clone()),
        );
        for _ in 0..(config_scores.len() - 2) {
            new_configs.push(Config::gen(&mut rng));
        }
    }
    new_configs
}

fn next_incremental_generation(configs_scores: &[ConfigScore]) -> Vec<Config> {
    let mut rng: ThreadRng = thread_rng();
    let mut new_configs = Vec::with_capacity(configs_scores.len());

    new_configs.push(configs_scores[0].0.clone());
    new_configs.extend(
        configs_scores
            .choose_multiple_weighted(
                &mut rng,
                (configs_scores.len() - 2) / 2,
                |ConfigScore(_, s)| s.0,
            )
            .unwrap()
            .map(|x| {
                let mut m = x.0.clone();
                m.mutate(0.1, &mut rng);
                m
            }),
    );
    new_configs.push(Config::gen(&mut rng));
    for _ in 0..((configs_scores.len() - 2) / 2) {
        let mut competitors = configs_scores
            .choose_multiple_weighted(&mut rng, 2, |ConfigScore(_, s)| s.0)
            .unwrap();
        let a = competitors.next().unwrap().0.clone();
        let b = competitors.next().unwrap().0.clone();
        let c = a.crossover(&b, &mut rng);
        new_configs.push(c);
    }

    new_configs
}

fn score_configs(configs: &[Config]) -> Vec<ConfigScore> {
    let mut config_score: Vec<ConfigScore> = configs
        .par_iter()
        .map(|c| {
            let mut state = State::gen(c.clone(), 64, 64);
            ConfigScore(
                c.clone(),
                (0..100)
                    .map(|_| {
                        let old_state = state.clone();
                        state.update();
                        score_state(&old_state, &state)
                    })
                    .sum::<f32>()
                    .into(),
            )
        })
        .collect();

    config_score.sort_unstable_by_key(|cs| cs.clone());

    config_score
}

fn score_state(old_state: &State, state: &State) -> f32 {
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
                    Some(Water | Air),
                    Some(Water),
                    Some(Water | Air),
                    Some(Air),
                    Some(Air),
                    Some(Air),
                    Some(Water | Air),
                    Some(Water),
                    Some(Water | Air),
                )
                | (
                    Some(Water | Air),
                    Some(Air),
                    Some(Water | Air),
                    Some(Water),
                    Some(Water),
                    Some(Water),
                    Some(Water | Air),
                    Some(Air),
                    Some(Water | Air),
                )
                | (
                    Some(Water | Soil),
                    Some(Water),
                    Some(Water | Soil),
                    Some(Soil),
                    Some(Soil),
                    Some(Soil),
                    Some(Water | Soil),
                    Some(Water),
                    Some(Water | Soil),
                )
                | (
                    Some(Water | Soil),
                    Some(Soil),
                    Some(Water | Soil),
                    Some(Water),
                    Some(Water),
                    Some(Water),
                    Some(Water | Soil),
                    Some(Soil),
                    Some(Water | Soil),
                )
                | (
                    Some(Air | Soil),
                    Some(Air),
                    Some(Air | Soil),
                    Some(Soil),
                    Some(Soil),
                    Some(Soil),
                    Some(Air | Soil),
                    Some(Air),
                    Some(Air | Soil),
                )
                | (
                    Some(Air | Soil),
                    Some(Soil),
                    Some(Air | Soil),
                    Some(Air),
                    Some(Air),
                    Some(Air),
                    Some(Air | Soil),
                    Some(Soil),
                    Some(Air | Soil),
                )
                | (
                    Some(Water | Soil),
                    Some(Water),
                    Some(Water | Soil),
                    Some(Soil),
                    Some(Water),
                    Some(Soil),
                    Some(Water | Soil),
                    Some(Water),
                    Some(Water | Soil),
                )
                | (
                    Some(Water | Soil),
                    Some(Soil),
                    Some(Water | Soil),
                    Some(Water),
                    Some(Soil),
                    Some(Water),
                    Some(Water | Soil),
                    Some(Soil),
                    Some(Water | Soil),
                )
                | (
                    Some(Water | Air),
                    Some(Water),
                    Some(Water | Air),
                    Some(Air),
                    Some(Water),
                    Some(Air),
                    Some(Water | Air),
                    Some(Water),
                    Some(Water | Air),
                )
                | (
                    Some(Water | Air),
                    Some(Air),
                    Some(Water | Air),
                    Some(Water),
                    Some(Air),
                    Some(Water),
                    Some(Water | Air),
                    Some(Air),
                    Some(Water | Air),
                )
                | (
                    Some(Soil | Air),
                    Some(Soil),
                    Some(Soil | Air),
                    Some(Air),
                    Some(Soil),
                    Some(Air),
                    Some(Soil | Air),
                    Some(Soil),
                    Some(Soil | Air),
                )
                | (
                    Some(Soil | Air),
                    Some(Air),
                    Some(Soil | Air),
                    Some(Soil),
                    Some(Air),
                    Some(Soil),
                    Some(Soil | Air),
                    Some(Air),
                    Some(Soil | Air),
                ) => -1.0,
                (
                    Some(Air),
                    Some(Air),
                    Some(Air),
                    Some(Water),
                    Some(Water),
                    Some(Water),
                    Some(Water),
                    Some(Water),
                    Some(Water),
                )
                | (
                    Some(Water),
                    Some(Water),
                    Some(Water),
                    Some(Soil),
                    Some(Soil),
                    Some(Soil),
                    Some(Soil),
                    Some(Soil),
                    Some(Soil),
                ) => 1.0,
                _ => 0.0,
            }
        })
        .sum::<f32>()
        / element_count as f32;

    let conflict_score = 1.0 / state.conflict_iters.max(1) as f32;

    2.0 * position_score + pattern_score + conflict_score
}
