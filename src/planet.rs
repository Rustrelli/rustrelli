use common_game::components::planet::{PlanetAI, PlanetState};
use common_game::components::resource::{Combinator, Generator};
use common_game::components::rocket::Rocket;
use common_game::protocols::messages::{
    ExplorerToPlanet, OrchestratorToPlanet, PlanetToExplorer, PlanetToOrchestrator,
};

pub(crate) struct AI {}

impl PlanetAI for AI {
    fn handle_orchestrator_msg(
        &mut self,
        state: &mut PlanetState,
        generator: &Generator,
        combinator: &Combinator,
        msg: OrchestratorToPlanet,
    ) -> Option<PlanetToOrchestrator> {
        match msg {
            OrchestratorToPlanet::Sunray(sunray) => {}
            OrchestratorToPlanet::Asteroid(_) => {}
            OrchestratorToPlanet::StartPlanetAI => {}
            OrchestratorToPlanet::StopPlanetAI => {}
            OrchestratorToPlanet::InternalStateRequest => {}
            OrchestratorToPlanet::IncomingExplorerRequest { .. } => {}
            OrchestratorToPlanet::OutgoingExplorerRequest { .. } => {}
        }
    }

    fn handle_explorer_msg(
        &mut self,
        state: &mut PlanetState,
        generator: &Generator,
        combinator: &Combinator,
        msg: ExplorerToPlanet,
    ) -> Option<PlanetToExplorer> {
        todo!()
    }

    fn handle_asteroid(
        &mut self,
        state: &mut PlanetState,
        generator: &Generator,
        combinator: &Combinator,
    ) -> Option<Rocket> {
        todo!()
    }

    fn start(&mut self, state: &PlanetState) {
        todo!()
    }

    fn stop(&mut self, state: &PlanetState) {
        todo!()
    }
}
