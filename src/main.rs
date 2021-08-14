use rand::thread_rng;
use crate::{cli::Cli, game::{Game, Player}};

extern crate rand;
extern crate rand_distr;

mod game;
mod cli;


fn main() {
    let players = vec![
        Player {name: "Alice".into()},
        Player {name: "Bob".into()},
        Player {name: "Charlotte".into()},
    ];
    let mut game = Game::new(8, 8, players, 5, &mut thread_rng()).unwrap();
    Cli::new(game).play()
}
