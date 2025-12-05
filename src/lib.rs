mod planet;

use planet::AI;
use common_game::components::planet::{Planet, PlanetType};
use common_game::components::resource::BasicResourceType;
use common_game::protocols::messages;
use std::sync::mpsc;

/// Creation function for the planet.
pub fn create_planet(
    rx_orchestrator: mpsc::Receiver<messages::OrchestratorToPlanet>,
    tx_orchestrator: mpsc::Sender<messages::PlanetToOrchestrator>,
    rx_explorer: mpsc::Receiver<messages::ExplorerToPlanet>,
) -> Planet {
    let id = 1;
    let ai = AI {};
    let gen_rules = vec![
        BasicResourceType::Carbon,
        BasicResourceType::Silicon,
        BasicResourceType::Oxygen,
        BasicResourceType::Hydrogen,
    ];
    let comb_rules = vec![];

    // Constructs the planet and returns it
    match Planet::new(
        id,
        PlanetType::D,
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
