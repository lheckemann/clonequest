use std::collections::HashSet;
use rand::{Rng, thread_rng};
use rand::seq::SliceRandom;
use rand_distr::Binomial;

type Pos = (usize, usize);

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct PlayerId(usize);
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct PlanetId(usize);
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct FleetId(usize);

#[derive(Clone, PartialEq, Eq)]
pub struct Player {
    pub name: String
}
#[derive(Clone)]
pub struct Fleet {
    pub ships: usize,
    pub strength: usize,
    pub turns_to_arrival: usize,
    pub destination: PlanetId,
    pub owner: PlayerId,
}

#[derive(Clone)]
pub struct Planet {
    pub name: String,
    pub ships: usize,
    pub strength: usize,
    pub production: usize,
    pub pos: Pos,
    pub owner: Option<PlayerId>,
}

#[derive(Clone)]
struct SendShipsCommand {
    source_planet_id: PlanetId,
    destination_planet_id: PlanetId,
    count: usize,
}

pub fn distance(a: &Planet, b: &Planet) -> usize {
    let (xa, ya) = a.pos;
    let (xb, yb) = b.pos;
    let dx = (xa as f32 - xb as f32).abs();
    let dy = (ya as f32 - yb as f32).abs();
    // Divide distance by 2 since the game pace is pretty slow otherwise
    ((dx * dx + dy * dy).sqrt() * 0.5).ceil() as usize
}
const PLANET_NAMES : &'static str = "ABCDEFGHIJKLMNOPQRSTUVWXYZ";

#[derive(Clone)]
pub struct Game {
    _planets: Vec<Planet>,
    _players: Vec<Player>,
    _fleets: Vec<Fleet>,
    _queued_commands: Vec<(PlayerId, SendShipsCommand)>,
    _w: usize,
    _h: usize,
}

#[derive(Debug)]
pub enum CouldNotSend {
    NoSuchPlanet,
    NotYourPlanet,
    NotEnoughShips,
}

#[derive(Debug)]
pub enum CouldNotCreateGame {
    TooManyPlanets,
}

pub enum Message {
    AttackFailed(Fleet),
    AttackSucceeded(Fleet),
    ReinforcementsArrived(Fleet),
    PlayerEliminated(Player),
}

impl Game {
    pub fn end_turn(&mut self) -> Vec<Message> {
        let mut messages = Vec::new();
        let alive_before = self.remaining_players();
        for planet in self._planets.iter_mut().filter(|p| p.owner != None) {
            planet.ships += planet.production;
        }
        for (player, command) in self._queued_commands.drain(..) {
            self._planets[command.source_planet_id.0].ships -= command.count;
            let source_planet = &self._planets[command.source_planet_id.0];
            let destination_planet = &self._planets[command.destination_planet_id.0];
            self._fleets.push(Fleet {
                ships: command.count,
                strength: source_planet.strength,
                turns_to_arrival: distance(source_planet, destination_planet),
                destination: command.destination_planet_id,
                owner: player,
            });
        }
        for fleet in self._fleets.iter_mut() {
            fleet.turns_to_arrival -= 1;
            if fleet.turns_to_arrival == 0 {
                let mut dest_planet = &mut self._planets[fleet.destination.0];
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
        let new_fleets = self._fleets.drain(..)
                                    .filter(|f| f.turns_to_arrival > 0 && f.ships > 0)
                                    .collect();

        let alive_after = self.remaining_players();
        alive_before
            .difference(&alive_after)
            .for_each(|player_index| {
                messages.push(Message::PlayerEliminated(self._players[player_index.0].clone()));
            });
        self._fleets = new_fleets;
        messages
    }

    pub fn new<R: Rng>(
        w: usize,
        h: usize,
        players: Vec<Player>,
        neutral_planets: usize,
        rng: &mut R
    ) -> Result<Game, CouldNotCreateGame> {
        let total_planets = players.len() as usize + neutral_planets;
        if total_planets > w * h {
            return Err(CouldNotCreateGame::TooManyPlanets);
        }
        let mut planets = Vec::new();
        let all_positions: Vec<Pos> = (0..w).flat_map(|x| {
            (0..h).map(move |y| {
                (x, y)
            })
        }).collect();
        let mut positions = all_positions.choose_multiple(rng, total_planets as usize);
        let mut names = PLANET_NAMES.chars();
        for (id, _player) in players.iter().enumerate() {
            planets.push(Planet {
                name: names.next().expect("Ran out of planet names!").into(),
                ships: 10,
                strength: 40,
                production: 10,
                pos: *positions.next().expect("Not enough positions!?"),
                owner: Some(PlayerId(id)),
            });
        }
        let strength_distribution = Binomial::new(100, 0.55).expect("Static binomial parameters should be ok!");
        let production_distribution = Binomial::new(10, 0.5).expect("Static binomial parameters should be ok!");
        positions.map(|pos| Planet {
            name: names.next().expect("Ran out of planet names!").into(),
            ships: 0,
            strength: rng.sample(strength_distribution) as usize,
            production: rng.sample(production_distribution) as usize + 5,
            pos: *pos,
            owner: None,
        }).for_each(|p| planets.push(p));
        Ok(Game {
            _planets: planets,
            _players: players,
            _fleets: Vec::new(),
            _queued_commands: vec![],
            _w: w,
            _h: h,
        })
    }

    pub fn queue_fleet(
        &mut self,
        player_id: PlayerId,
        source_planet_id: PlanetId,
        destination_planet_id: PlanetId,
        count: usize,
    ) -> Result<(), CouldNotSend> {
        if self._planets.len() <= source_planet_id.0 || self._planets.len() <= destination_planet_id.0 {
            return Err(CouldNotSend::NoSuchPlanet)
        }
        if self._planets[source_planet_id.0].owner != Some(player_id) {
            return Err(CouldNotSend::NotYourPlanet)
        }
        let planet_queued_ships: usize = self._queued_commands.iter()
            .filter(|(_player, command)| command.source_planet_id == source_planet_id)
            .map(|(_player, command)| command.count)
            .sum();
        let planet_remaining_ships = self._planets[source_planet_id.0].ships - planet_queued_ships;
        if planet_remaining_ships < count {
            return Err(CouldNotSend::NotEnoughShips)
        }
        self._queued_commands.push((player_id, SendShipsCommand {
            source_planet_id,
            destination_planet_id,
            count,
        }));
        Ok(())
    }

    pub fn remaining_players(&self) -> HashSet<PlayerId> {
        let players_with_planets : HashSet<PlayerId> = self._planets.iter().filter_map(|p| p.owner).collect();
        let players_with_fleets : HashSet<PlayerId> = self._fleets.iter().map(|f| f.owner).collect();
        players_with_planets.union(&players_with_fleets).map(|p| *p).collect()
    }

    pub fn get_winner(&self) -> Option<PlayerId> {
        let players = self.remaining_players();
        if players.len() == 1 {
            Some(*players.iter().next().unwrap())
        } else {
            None
        }
    }

    pub fn get_planet_id(&self, name : &String) -> Result<PlanetId, String> {
        if name.len() != 1 {
            Err("Planet names are a single character".to_string())
        } else {
            PLANET_NAMES.chars()
                        .take(self._planets.len())
                        .position(|c| c.to_string() == *name)
                        .map(PlanetId)
                        .ok_or("no such planet".to_string())
        }
    }

    pub fn planets(&self) -> impl Iterator<Item = (PlanetId, &Planet)> {
        self._planets.iter().enumerate().map(|(id, planet)| (PlanetId(id), planet))
    }
    pub fn planet(&self, id: PlanetId) -> Result<&Planet, ()> {
        if self._planets.len() <= id.0 {
            Err(())
        } else {
            Ok(&self._planets[id.0])
        }
    }

    pub fn players(&self) -> impl Iterator<Item = (PlayerId, &Player)> {
        self._players.iter().enumerate().map(|(id, player)| (PlayerId(id), player))
    }
    pub fn player(&self, id: PlayerId) -> Result<&Player, ()> {
        if self._players.len() <= id.0 {
            Err(())
        } else {
            Ok(&self._players[id.0])
        }
    }

    pub fn size(&self) -> (usize, usize) {
        (self._w, self._h)
    }
}
