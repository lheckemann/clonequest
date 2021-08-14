extern crate rand;
extern crate rand_distr;
use std::collections::HashSet;
use std::io;
use std::io::Write;
use std::fmt;
use rand::{thread_rng, Rng};
use rand::seq::SliceRandom;
use rand_distr::Binomial;

/*
   Map<Player, Planet>
   Map<Player, Fleet>
 */
type Pos = (usize, usize);
#[derive(Clone, PartialEq, Eq)]
struct Player {
    name: String
}
#[derive(Clone)]
struct Fleet {
    ships: usize,
    strength: usize,
    turns_to_arrival: usize,
    destination: usize,
    owner: usize,
}
#[derive(Clone)]
struct Planet {
    ships: usize,
    strength: usize,
    production: usize,
    pos: Pos,
    owner: Option<usize>,
}

fn distance(a: &Planet, b: &Planet) -> usize {
    let (xa, ya) = a.pos;
    let (xb, yb) = b.pos;
    let dx = (xa as f32 - xb as f32).abs();
    let dy = (ya as f32 - yb as f32).abs();
    // Divide distance by 2 since the game pace is pretty slow otherwise
    ((dx * dx + dy * dy).sqrt() * 0.5).ceil() as usize
}
const PLANET_NAMES : &'static str = "ABCDEFGHIJKLMNOPQRSTUVWXYZ";

#[derive(Clone)]
struct Game {
    planets: Vec<Planet>,
    players: Vec<Player>,
    fleets: Vec<Fleet>,
    current_player_index: usize,
    w: usize,
    h: usize,
}

#[derive(Debug)]
enum CouldNotSend {
    NotEnoughShips,
    NotYourPlanet,
}
impl fmt::Display for CouldNotSend {
    fn fmt(&self, f : &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{:?}", self)
    }
}

#[derive(Debug)]
enum CouldNotCreateGame {
    TooManyPlanets,
}

enum Message {
    AttackFailed(Fleet),
    AttackSucceeded(Fleet),
    ReinforcementsArrived(Fleet),
    PlayerEliminated(Player),
}

impl Game {
    pub fn current_player(&self) -> &Player {
        &self.players[self.current_player_index]
    }
    pub fn send_fleet(
        &mut self,
        source_planet_id: usize,
        dest_planet_id: usize,
        count: usize,
    ) -> Result<(), CouldNotSend> {
        if self.planets[source_planet_id].owner != Some(self.current_player_index) {
            Err(CouldNotSend::NotYourPlanet)
        } else if self.planets[source_planet_id].ships < count {
            Err(CouldNotSend::NotEnoughShips)
        } else {
            let dist = distance(
                &self.planets[source_planet_id],
                &self.planets[dest_planet_id],
            );
            let source_planet = &mut self.planets[source_planet_id];
            source_planet.ships -= count;
            let fleet: Fleet = Fleet {
                ships: count,
                strength: source_planet.strength,
                turns_to_arrival: dist,
                owner: self.current_player_index,
                destination: dest_planet_id,
            };
            self.fleets.push(fleet);
            Ok(())
        }
    }
    pub fn end_turn(&mut self) -> Vec<Message> {
        let mut messages = Vec::new();
        self.current_player_index = (self.current_player_index + 1) % self.players.len();
        if self.current_player_index == 0 {
            let alive_before = self.remaining_players();
            for planet in self.planets.iter_mut().filter(|p| p.owner != None) {
                planet.ships += planet.production;
            }
            for fleet in self.fleets.iter_mut() {
                fleet.turns_to_arrival -= 1;
                if fleet.turns_to_arrival == 0 {
                    let mut dest_planet = &mut self.planets[fleet.destination];
                    if Some(fleet.owner) == dest_planet.owner {
                        messages.push(Message::ReinforcementsArrived(fleet.clone()));
                        dest_planet.ships += fleet.ships
                    } else {
                        loop {
                            // defender roll
                            if thread_rng().gen_bool(dest_planet.strength as f64 / 100.0) {
                                fleet.ships -= 1;
                                // defender wins
                                if fleet.ships <= 0 {
                                    messages.push(Message::AttackFailed(fleet.clone()));
                                    break;
                                }
                            }
                            // attacker roll
                            if thread_rng().gen_bool(fleet.strength as f64 / 100.0) {
                                // attacker wins
                                if dest_planet.ships <= 0 {
                                    dest_planet.owner = Some(fleet.owner);
                                    dest_planet.ships = fleet.ships;
                                    messages.push(Message::AttackSucceeded(fleet.clone()));
                                    break;
                                }
                                dest_planet.ships -= 1;
                            }
                        }
                    }
                }
            }
            let new_fleets = self.fleets.drain(..)
                                        .filter(|f| f.turns_to_arrival > 0 && f.ships > 0)
                                        .collect();

            let alive_after = self.remaining_players();
            alive_before
                .difference(&alive_after)
                .for_each(|player_index| {
                    messages.push(Message::PlayerEliminated(self.players[*player_index].clone()));
                });
            self.fleets = new_fleets;
        }
        messages
    }
    pub fn new(
        w: usize,
        h: usize,
        players: Vec<Player>,
        neutral_planets: usize,
    ) -> Result<Game, CouldNotCreateGame> {
        let total_planets = players.len() as usize + neutral_planets;
        if total_planets > w * h {
            return Err(CouldNotCreateGame::TooManyPlanets);
        }
        let mut planets = Vec::new();
        let mut rng = thread_rng();
        let all_positions: Vec<Pos> = (0..w).flat_map(|x| {
            (0..h).map(move |y| {
                (x, y)
            })
        }).collect();
        let mut positions = all_positions.choose_multiple(&mut rng, total_planets as usize);
        for (id, _player) in players.iter().enumerate() {
            planets.push(Planet {
                ships: 10,
                strength: 40,
                production: 10,
                pos: *positions.next().expect("Not enough positions!?"),
                owner: Some(id),
            });
        }
        let strength_distribution = Binomial::new(100, 0.55).expect("Static binomial parameters should be ok!");
        let production_distribution = Binomial::new(10, 0.5).expect("Static binomial parameters should be ok!");
        positions.map(|pos| Planet {
            ships: 0,
            strength: rng.sample(strength_distribution) as usize,
            production: rng.sample(production_distribution) as usize + 5,
            pos: *pos,
            owner: None,
        }).for_each(|p| planets.push(p));
        Ok(Game {
            planets,
            players,
            fleets: Vec::new(),
            current_player_index: 0,
            w,
            h,
        })
    }

    fn remaining_players(&self) -> HashSet<usize> {
        let players_with_planets : HashSet<usize> = self.planets.iter().filter_map(|p| p.owner).collect();
        let players_with_fleets : HashSet<usize> = self.fleets.iter().map(|f| f.owner).collect();
        players_with_planets.union(&players_with_fleets).map(|p| *p).collect()
    }

    fn get_winner(&self) -> Option<usize> {
        let players = self.remaining_players();
        if players.len() == 1 {
            Some(*players.iter().next().unwrap())
        } else {
            None
        }
    }

    pub fn print(&self) {
        for y in 0..self.h {
            for x in 0..self.w {
                print!("│{}",
                self.planets.iter()
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

    fn planet_name(&self, index: usize) -> Option<String> {
        PLANET_NAMES.chars().take(self.planets.len()).nth(index).map(|c| c.to_string())
    }

    fn get_planet_index(&self, name : &String) -> Result<usize, String> {
        if name.len() != 1 {
            Err("Planet names are a single character".to_string())
        } else {
            PLANET_NAMES.chars()
                        .take(self.planets.len())
                        .position(|c| c.to_string() == *name)
                        .ok_or("no such planet".to_string())
        }
    }


    pub fn play(&mut self) {
        while self.get_winner().is_none() {
            self.do_turn();
        }
    }

    fn do_turn(&mut self) {
        let mut input = String::new();
        self.print();
        print!("
s A B n - send n ships from A to B
d - show distances between all planets
d A B C … - show distance for trips between A, B, C…
i - info on planets
i A B … - info on specific planets
n - finish turn
Player {}: ", self.players[self.current_player_index].name);
        io::stdout().flush();

        match io::stdin().read_line(&mut input) {
            Ok(_) => {
                let cmd = self.do_command(input.split_whitespace().map(|s| s.to_string()).collect());
                cmd.unwrap_or_else(|e| println!("{}", e));
            },
            Err(e) => ()  // TODO: handle error, e is IoError
        }
    }

    fn do_command(&mut self, tokens: Vec<String>) -> Result<(), String> {
        if tokens.len() < 1 {
            return Err("No command provided".to_string())
        }
        match tokens[0].as_str() {
            "n" => {
                for message in self.end_turn() {
                    match message {
                        Message::AttackFailed(fleet) => {
                            let player = &self.players[fleet.owner].name;
                            let planet = self.planet_name(fleet.destination).unwrap_or("<unknown>".into());
                            println!("Fleet from player {} failed to take planet {}.", player, planet);
                        }
                        Message::AttackSucceeded(fleet) => {
                            let player = &self.players[fleet.owner].name;
                            let planet = self.planet_name(fleet.destination).unwrap_or("<unknown>".into());
                            println!("Fleet from player {} took over planet {}!", player, planet);
                        }
                        Message::ReinforcementsArrived(fleet) => {
                            let planet = self.planet_name(fleet.destination).unwrap_or("<unknown>".into());
                            println!("Reinforcements of {} ships have arrived at planet {}.", fleet.ships, planet);
                        }
                        Message::PlayerEliminated(player) => {
                            println!("Player {} was eliminated!", player.name);
                        }
                    }
                }
                return Ok(());
            },
            "i" => {
                println!(" Planet | Ships  | Power  | Prod   | Owner");
                let print_planet = |(planet_index, planet): (usize, &Planet)| {
                    println!(" {: ^6} | {: >6} | {: >6} | {: >6} | {}",
                            self.planet_name(planet_index).unwrap_or(".".to_string()),
                            planet.ships,
                            planet.strength,
                            planet.production,
                            planet.owner.map(|i| self.players[i].name.clone()).unwrap_or("-".into())
                    )
                };
                let mut chosen = tokens.iter().skip(1).filter_map(|tok| {
                    let planet_index = self.get_planet_index(tok).map_err(|e| println!("Planet {}: {}, skipping", tok, e));
                    planet_index.map(|i| (i, &self.planets[i])).ok()
                }).peekable();
                if chosen.peek().is_some() {
                    chosen.for_each(print_planet)
                } else {
                    self.planets.iter().enumerate().for_each(print_planet)
                }
                return Ok(());
            },
            "s" => {
                if tokens.len() != 4 {
                    return Err("Need a source and destination planet and a number of ships".to_string());
                }
                let src = self.get_planet_index(&tokens[1])?;
                let dest = self.get_planet_index(&tokens[2])?;
                let count = usize::from_str_radix(tokens[3].as_str(), 10)
                                   .map_err(|_| "Invalid number of ships".to_string())?;
                return self.send_fleet(src, dest, count).map_err(|e| e.to_string());
            },
            "d" => {
                let chosen : Vec<usize> = tokens.iter().skip(1).filter_map(|tok| {
                    self.get_planet_index(tok).map_err(|e| println!("Planet {}: {}, skipping", tok, e)).ok()
                }).collect();
                if chosen.is_empty() {
                    self.show_distances();
                } else {
                    self.show_distances_for(chosen);
                }
                return Ok(());
            }
            _ => return Err("No command".to_string())
        }
    }

    pub fn show_distances(&self) {
        self.show_distances_for((0..self.planets.len()).collect())
    }

    pub fn show_distances_for(&self, planets: Vec<usize>) {
        print!("\\|");
        for p in planets.iter() {
            print!("{: ^3}|", self.planet_name(*p).unwrap());
        }
        for p1 in planets.iter() {
            print!("\n{}|", self.planet_name(*p1).unwrap());
            for p2 in planets.iter() {
                let d = distance(&self.planets[*p1], &self.planets[*p2]);
                if p1 != p2 {
                    print!("{: >3}|", d);
                } else {
                    print!("   |");
                }
            }
        }
        print!("\n\n");
    }
}

fn main() {
    let mut g = Game::new(8, 8, vec![
        Player {name: "Alice".into()}, Player {name: "Bob".into()}, Player {name: "Charlotte".into()}
    ], 5).unwrap();
    g.show_distances();
    g.play();
}
