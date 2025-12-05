mod planet;

use std::sync::mpsc;
use common_game::components::planet::{Planet, PlanetType};
use common_game::protocols::messages;
use crate::planet::AI;

/// Main function creation planet.
pub fn create_planet(
    rx_orchestrator: mpsc::Receiver<messages::OrchestratorToPlanet>,
    tx_orchestrator: mpsc::Sender<messages::PlanetToOrchestrator>,
    rx_explorer: mpsc::Receiver<messages::ExplorerToPlanet>,
) -> Planet {
    let id = 1;
    let ai = AI {};
    let gen_rules = vec![/* your recipes */];
    let comb_rules = vec![/* your recipes */];

    // Construct the planet and return it
    match Planet::new(
        id,
        PlanetType::A,
        Box::new(ai),
        gen_rules,
        comb_rules,
        (rx_orchestrator, tx_orchestrator),
        rx_explorer,
    ) {
        Ok(planet) => planet,
        Err(error) => panic!("{}", error),
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn test() {

    }
}
