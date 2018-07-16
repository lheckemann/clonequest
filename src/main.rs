extern crate rand;
use std::collections::HashSet;
use std::io;
use std::fmt;
use rand::{thread_rng, Rng};
use rand::seq::sample_slice;
use rand::distributions::Binomial;

/* 
   Map<Player, Planet>
   Map<Player, Fleet>
 */
type Pos = (usize, usize);
#[derive(Clone, PartialEq, Eq)]
struct Player {}
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
    (dx * dx + dy * dy).sqrt().ceil() as usize
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
    pub fn end_turn(&mut self) {
        self.current_player_index = (self.current_player_index + 1) % self.players.len();
        if self.current_player_index == 0 {
            for planet in self.planets.iter_mut() {
                planet.ships += planet.production;
            }
            for fleet in self.fleets.iter_mut() {
                fleet.turns_to_arrival -= 1;
                if fleet.turns_to_arrival == 0 {
                    let mut dest_planet = &mut self.planets[fleet.destination];
                    if Some(fleet.owner) == dest_planet.owner {
                        // TODO: work out messaging
                        dest_planet.ships += fleet.ships
                    } else {
                        loop {
                            // defender roll
                            if thread_rng().gen_bool(dest_planet.strength as f64 / 100.0) {
                                fleet.ships -= 1;
                            }
                            // defender wins
                            if fleet.ships <= 0 {
                                break;
                            }
                            // attacker roll
                            if thread_rng().gen_bool(fleet.strength as f64 / 100.0) {
                                dest_planet.ships -= 1;
                            }
                            // attacker wins
                            if dest_planet.ships <= 0 {
                                dest_planet.owner = Some(fleet.owner);
                                dest_planet.ships = fleet.ships;
                                break;
                            }
                        }
                    }
                }
            }
            let new_fleets = self.fleets.drain(..)
                                        .filter(|f| f.turns_to_arrival > 0 && f.ships > 0)
                                        .collect();
            self.fleets = new_fleets;
        }
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
        let mut available_positions: Vec<Pos> = Vec::new();
        for x in 0..w {
            for y in 0..h {
                available_positions.push((x, y));
            }
        }
        let mut positions = sample_slice(&mut rng, &available_positions, total_planets as usize);
        for (id, _player) in players.iter().enumerate() {
            planets.push(Planet {
                ships: 0,
                strength: 40,
                production: 10,
                pos: positions.pop().expect("Not enough positions!?"),
                owner: Some(id),
            });
        }
        let strength_distribution = Binomial::new(100, 0.55);
        let production_distribution = Binomial::new(10, 0.5);
        positions.drain(..).map(|pos| Planet {
            ships: 0,
            strength: rng.sample(strength_distribution) as usize,
            production: rng.sample(production_distribution) as usize + 5,
            pos: pos,
            owner: None,
        }).for_each(|p| planets.push(p));
        Ok(Game {
            planets: planets,
            players: players,
            fleets: Vec::new(),
            current_player_index: 0,
            w: w,
            h: h,
        })
    }

    fn get_winner(&self) -> Option<usize> {
        let players_with_planets : HashSet<usize> = self.planets.iter().filter_map(|p| p.owner).collect();
        let players_with_fleets : HashSet<usize> = self.fleets.iter().map(|f| f.owner).collect();
        let remaining_players : Vec<&usize> = players_with_planets.union(&players_with_fleets).collect();
        if remaining_players.len() == 1 {
            Some(*remaining_players[0])
        } else {
            None
        }
    }

    pub fn print(&self) {
        for y in 0..self.h {
            for x in 0..self.w {
                print!("{}",
                self.planets.iter()
                            .zip(PLANET_NAMES.chars())
                            .filter(|(p, _)| p.pos == (x, y))
                            .take(1).last()
                            .map(|(_p, c)| c.to_string())
                            .unwrap_or(" ".to_string())
                            );
            }
            println!("")
        }
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
        self.print();
        while self.get_winner().is_none() {
            self.do_turn();
        }
    }

    fn do_turn(&mut self) {
        let mut input = String::new();
        println!("
s A B n - send n ships from A to B
d A B - distance between A and B
i - info on planets
n - next
");

        match io::stdin().read_line(&mut input) {
            Ok(_) => { self.do_command(input.split_whitespace().map(|s| s.to_string()).collect()); },
            Err(e) => ()  // handle error, e is IoError
        }
    }

    fn do_command(&mut self, tokens: Vec<String>) -> Result<(), String>{
        if tokens.len() < 1 {
            return Err("No command provided".to_string())
        }
        match tokens[0].as_str() {
            "n" => {
                self.end_turn();
                return Ok(());
            },
            "i" => {
                return Ok(()); //TODO
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
                return Ok(()); // TODO
            }
            _ => return Err("No command".to_string())
        }
    }
}

fn main() {
    Game::new(10, 5, vec![Player {}, Player {}, Player {}], 5).unwrap().play();
}
