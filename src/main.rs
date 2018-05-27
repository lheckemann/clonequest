/* 
   Map<Player, Planet>
   Map<Player, Fleet>
 */
type Pos = (u32, u32);
#[derive(Clone, PartialEq, Eq)]
struct Player {
}
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
    pub fn send_fleet(&mut self,
                     source_planet_id: usize,
                     dest_planet_id: usize,
                     count: u32)
        -> Result<(), CouldNotSend>
    {
        if self.planets[source_planet_id].ships < count {
            Err(CouldNotSend::NotEnoughShips)
        } else if self.planets[source_planet_id].owner != Some(self.current_player_index) {
            Err(CouldNotSend::NotYourPlanet)
        } else {
            let dist = distance(&self.planets[source_planet_id], &self.planets[dest_planet_id]);
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
    pub fn new(w: u32,
               h: u32,
               players: Vec<Player>,
               neutral_planets: u32)
        -> Result<Game, CouldNotCreateGame>
    {
        if players.len() as u32 + neutral_planets > w * h {
            return Err(CouldNotCreateGame::TooManyPlanets)
        }
        let planets = Vec::new();
        Ok(Game {
            planets: planets,
            players: players,
            fleets: Vec::new(),
            current_player_index: 0,
        })
    }
}
fn main() {
    println!("Hello, world!");
}
