use std::{collections::HashSet, io};
use std::io::Write;
use std::fmt;

use crate::game::{Player, Game, distance, CouldNotSend, Planet, Message};

const PLANET_NAMES : &'static str = "ABCDEFGHIJKLMNOPQRSTUVWXYZ";

impl fmt::Display for CouldNotSend {
    fn fmt(&self, f : &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{:?}", self)
    }
}

pub struct Cli {
    game: Game,
    current_player_id: usize,
    players_to_make_moves: Vec<usize>,
}

fn print_game_map(game: &Game) {
    let (w, h) = game.size();
    for y in 0..h {
        for x in 0..w {
            print!("│{}",
            game.planets()
                   .zip(PLANET_NAMES.chars())
                   .filter(|(p, _)| p.pos == (x, y))
                   .take(1).last()
                   .map(|(_p, c)| c.to_string())
                   .unwrap_or(" ".to_string())
            );
        }
        println!("│")
    }
}

fn print_game_info(game: &Game, planet_names: &[String]) {
    println!(" Planet | Ships  | Power  | Prod   | Owner");
    let print_planet = |(planet_index, planet): (usize, &Planet)| {
        println!(
            " {: ^6} | {: >6} | {: >6} | {: >6} | {}",
            game.planet_name(planet_index).unwrap_or("?".to_string()),
            planet.ships,
            planet.strength,
            planet.production,
            planet.owner.map(|i| game.player(i).map(|p| p.name.clone()).unwrap_or("?".into())).unwrap_or("-".into())
        )
    };
    if planet_names.is_empty() {
        game.planets().enumerate().for_each(print_planet)
    } else {
        let planets = planet_names.iter().filter_map(|tok| {
            let planet_index = game.get_planet_index(tok).map_err(|e| println!("Planet {}: {}, skipping", tok, e)).ok();
            planet_index.and_then(|i| game.planet(i).ok().map(|p| (i, p)))
        });
        planets.for_each(print_planet)
    }
}

fn show_distances(game: &Game) {
    show_distances_for(game, (0..game.planets().len()).collect())
}

fn show_distances_for(game: &Game, planet_ids: Vec<usize>) {
    let planets: Vec<&Planet> = planet_ids.iter().filter_map(|id| game.planet(*id).ok()).collect();
    print!("\\|");
    for p in planets.iter() {
        print!("{: ^3}|", p.name);
    }
    for p1 in planets.iter() {
        print!("\n{}|", p1.name);
        for p2 in planets.iter() {
            let d = distance(p1, p2);
            if d != 0 {
                print!("{: >3}|", d);
            } else {
                print!("   |");
            }
        }
    }
    print!("\n\n");
}

impl Cli {
    pub fn new(game: Game) -> Cli {
        let mut result = Cli {
            game,
            current_player_id: 0,
            players_to_make_moves: vec![],
        };
        result.next_player();
        result
    }

    pub fn play(&mut self) {
        while self.game.get_winner().is_none() {
            self.do_turn();
        }
    }

    fn do_turn(&mut self) {
        let mut input = String::new();
        print_game_map(&self.game);
        print!("
s A B n - send n ships from A to B
d - show distances between all planets
d A B C … - show distance for trips between A, B, C…
i - info on planets
i A B … - info on specific planets
n - finish turn
Player {}: ", self.game.player(self.current_player_id).unwrap().name);
        io::stdout().flush();

        match io::stdin().read_line(&mut input) {
            Ok(count) if count > 0 => {
                let cmd = self.do_command(input.split_whitespace().map(|s| s.to_string()).collect());
                cmd.unwrap_or_else(|e| println!("{}", e));
            },
            _ => panic!("Could not get input"),
        }
    }

    fn reset_moves(&mut self) {
        self.players_to_make_moves = self.game.remaining_players().drain().collect();
        self.players_to_make_moves.sort_by(|a, b| b.cmp(a));
    }

    fn next_player(&mut self) {
        match self.players_to_make_moves.pop() {
            Some(p) => { self.current_player_id = p; },
            None => self.complete_turn(),
        }
    }

    fn complete_turn(&mut self) {
        println!("\n\n\n----- Turn ended ------");
        for message in self.game.end_turn() {
            match message {
                Message::AttackFailed(fleet) => {
                    let player = self.game.player(fleet.owner).map(|p| p.name.clone()).unwrap_or("<unknown>".into());
                    let planet = self.game.planet_name(fleet.destination).unwrap_or("<unknown>".into());
                    println!("Fleet from player {} failed to take planet {}.", player, planet);
                }
                Message::AttackSucceeded(fleet) => {
                    let player = self.game.player(fleet.owner).map(|p| p.name.clone()).unwrap_or("<unknown>".into());
                    let planet = self.game.planet_name(fleet.destination).unwrap_or("<unknown>".into());
                    println!("Fleet from player {} took over planet {}!", player, planet);
                }
                Message::ReinforcementsArrived(fleet) => {
                    let planet = self.game.planet_name(fleet.destination).unwrap_or("<unknown>".into());
                    println!("Reinforcements of {} ships have arrived at planet {}.", fleet.ships, planet);
                }
                Message::PlayerEliminated(player) => {
                    println!("Player {} was eliminated!", player.name);
                }
            }
        }
        self.reset_moves();
        self.next_player();
    }

    fn do_command(&mut self, tokens: Vec<String>) -> Result<(), String> {
        if tokens.len() < 1 {
            return Err("No command provided".to_string())
        }
        match tokens[0].as_str() {
            "n" => {
                self.next_player();
                return Ok(());
            },
            "i" => {
                print_game_info(&self.game, &tokens[1..]);
                return Ok(());
            },
            "s" => {
                if tokens.len() != 4 {
                    return Err("Need a source and destination planet and a number of ships".to_string());
                }
                let src = self.game.get_planet_index(&tokens[1])?;
                let dest = self.game.get_planet_index(&tokens[2])?;
                let count = usize::from_str_radix(tokens[3].as_str(), 10)
                                   .map_err(|_| "Invalid number of ships".to_string())?;
                return self.game.queue_fleet(self.current_player_id, src, dest, count).map_err(|e| e.to_string());
            },
            "d" => {
                let chosen : Vec<usize> = tokens.iter().skip(1).filter_map(|tok| {
                    self.game.get_planet_index(tok).map_err(|e| println!("Planet {}: {}, skipping", tok, e)).ok()
                }).collect();
                if chosen.is_empty() {
                    show_distances(&self.game);
                } else {
                    show_distances_for(&self.game, chosen);
                }
                return Ok(());
            }
            _ => return Err("No command".to_string())
        }
    }

}
