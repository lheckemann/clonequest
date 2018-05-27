extern crate rand;
use rand::{thread_rng, Rng};
use rand::seq::sample_slice;
use rand::distributions::{Uniform, Binomial};
/* 
   Map<Player, Planet>
   Map<Player, Fleet>
 */
type Pos = (u32, u32);
#[derive(Clone, PartialEq, Eq)]
struct Player {}
#[derive(Clone)]
struct Fleet {
    ships: u32,
    strength: u32,
    turns_to_arrival: u32,
    destination: usize,
    owner: usize,
}
#[derive(Clone)]
struct Planet {
    ships: u32,
    strength: u32,
    production: u32,
    pos: Pos,
    owner: Option<usize>,
}

fn distance(a: &Planet, b: &Planet) -> u32 {
    let (xa, ya) = a.pos;
    let (xb, yb) = b.pos;
    let dx = (xa as f32 - xb as f32).abs();
    let dy = (ya as f32 - yb as f32).abs();
    (dx * dx + dy * dy).sqrt().ceil() as u32
}

#[derive(Clone)]
struct Game {
    planets: Vec<Planet>,
    players: Vec<Player>,
    fleets: Vec<Fleet>,
    current_player_index: usize,
    w: u32,
    h: u32,
}

enum CouldNotSend {
    NotEnoughShips,
    NotYourPlanet,
}

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
        count: u32,
    ) -> Result<(), CouldNotSend> {
        if self.planets[source_planet_id].ships < count {
            Err(CouldNotSend::NotEnoughShips)
        } else if self.planets[source_planet_id].owner != Some(self.current_player_index) {
            Err(CouldNotSend::NotYourPlanet)
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
            for fleet in self.fleets.iter_mut() {
                fleet.turns_to_arrival -= 1;
                if fleet.turns_to_arrival == 0 {
                    // TODO
                }
            }
        }
    }
    pub fn new(
        w: u32,
        h: u32,
        players: Vec<Player>,
        neutral_planets: u32,
    ) -> Result<Game, CouldNotCreateGame> {
        let total_planets = players.len() as u32 + neutral_planets;
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
        for (id, player) in players.iter().enumerate() {
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
            strength: rng.sample(strength_distribution) as u32,
            production: rng.sample(production_distribution) as u32 + 5,
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

    pub fn print(&self) {
        for y in 0..self.h {
            for x in 0..self.w {
                print!("{}",
                self.planets.iter()
                            .filter(|p| p.pos == (x, y))
                            .take(1).last()
                            .map(|p| "O")
                            .unwrap_or(" ")
                            );
            }
            println!("")
        }
    }
}
fn main() {
    Game::new(10, 5, vec![Player {}, Player {}, Player {}], 5).map(|g| g.print());
}
