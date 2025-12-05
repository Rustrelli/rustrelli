use common_game::components::planet::{PlanetAI, PlanetState};
use common_game::components::resource::{BasicResource, BasicResourceType, Combinator, Generator};
use common_game::components::rocket::Rocket;
use common_game::protocols::messages::{
    ExplorerToPlanet, OrchestratorToPlanet, PlanetToExplorer, PlanetToOrchestrator,
};
use std::collections::HashSet;

// TODO: ADD LOGGING AND DOCS

pub(crate) struct AI {}

impl PlanetAI for AI {
    fn handle_orchestrator_msg(
        &mut self,
        state: &mut PlanetState,
        generator: &Generator,
        _combinator: &Combinator,
        msg: OrchestratorToPlanet,
    ) -> Option<PlanetToOrchestrator> {
        match msg {
            OrchestratorToPlanet::Sunray(sunray) => {
                // generate resource on the fly and store them based
                // on the request frequency (calculated on the number of times it's requested
                // by explorers)
                // todo!()
                state.charge_cell(sunray);
                Some(PlanetToOrchestrator::SunrayAck {
                    planet_id: state.id(),
                })
            }
            OrchestratorToPlanet::InternalStateRequest => {
                todo!()
            }
            _ => None,
        }
    }

    fn handle_explorer_msg(
        &mut self,
        state: &mut PlanetState,
        generator: &Generator,
        _combinator: &Combinator,
        msg: ExplorerToPlanet,
    ) -> Option<PlanetToExplorer> {
        match msg {
            ExplorerToPlanet::SupportedResourceRequest { .. } => {
                Some(PlanetToExplorer::SupportedResourceResponse {
                    resource_list: generator.all_available_recipes(),
                })
            }
            ExplorerToPlanet::SupportedCombinationRequest { .. } => {
                Some(PlanetToExplorer::SupportedCombinationResponse {
                    combination_list: HashSet::new(),
                })
            }
            ExplorerToPlanet::GenerateResourceRequest {
                explorer_id,
                resource,
            } => {
                // TODO: check for stored resource (see Sunray response in handle_orchestrator_msg()
                if let Some((cell, _)) = state.full_cell() {
                    let resource: BasicResource = match resource {
                        BasicResourceType::Oxygen => {
                            BasicResource::Oxygen(generator.make_oxygen(cell).unwrap())
                        }
                        BasicResourceType::Hydrogen => {
                            BasicResource::Hydrogen(generator.make_hydrogen(cell).unwrap())
                        }
                        BasicResourceType::Carbon => {
                            BasicResource::Carbon(generator.make_carbon(cell).unwrap())
                        }
                        BasicResourceType::Silicon => {
                            BasicResource::Silicon(generator.make_silicon(cell).unwrap())
                        }
                    };

                    Some(PlanetToExplorer::GenerateResourceResponse {
                        resource: Some(resource),
                    })
                } else {
                    None
                }
            }
            ExplorerToPlanet::CombineResourceRequest { .. } => None,
            ExplorerToPlanet::AvailableEnergyCellRequest { .. } => {
                Some(PlanetToExplorer::AvailableEnergyCellResponse {
                    available_cells: state.to_dummy().charged_cells_count as u32,
                })
            }
        }
    }

    fn handle_asteroid(
        &mut self,
        state: &mut PlanetState,
        generator: &Generator,
        combinator: &Combinator,
    ) -> Option<Rocket> {
        // TODO: logging
        None
    }

    fn start(&mut self, state: &PlanetState) {
        todo!()
    }

    fn stop(&mut self, state: &PlanetState) {
        todo!()
    }
}
