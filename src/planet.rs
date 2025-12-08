use common_game::components::planet::{PlanetAI, PlanetState};
use common_game::components::resource::{BasicResource, BasicResourceType, Combinator, ComplexResource, ComplexResourceRequest, Generator, GenericResource};
use common_game::components::rocket::Rocket;
use common_game::protocols::messages::{
    ExplorerToPlanet, OrchestratorToPlanet, PlanetToExplorer, PlanetToOrchestrator,
};

// TODO: Logging and docs

// features:
// - user of the planet can choose between: fair-share resource generation between explorers or
//   explorers priority list to assign priority levels to each explorer -> planet tracks explorer requests to estimate resources usage
// - [probably cheating by game rules] speculative resource generation to prevent sunray waste (all cells are full),
//   based on generation requests history of specific explorers.

pub struct AI {}

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
                state.charge_cell(sunray);
                Some(PlanetToOrchestrator::SunrayAck {
                    planet_id: state.id(),
                })
            }

            OrchestratorToPlanet::InternalStateRequest => {
                Some(PlanetToOrchestrator::InternalStateResponse {
                    planet_id: state.id(),
                    planet_state: state.to_dummy(),
                })
            }

            _ => None,
        }
    }

    fn handle_explorer_msg(
        &mut self,
        state: &mut PlanetState,
        generator: &Generator,
        combinator: &Combinator,
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
                    combination_list: combinator.all_available_recipes(),
                })
            }

            ExplorerToPlanet::GenerateResourceRequest {
                explorer_id,
                resource,
            } => {
                // TODO: check for stored resource (see Sunray response in handle_orchestrator_msg())
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
                    Some(PlanetToExplorer::GenerateResourceResponse { resource: None })
                }
            }

            ExplorerToPlanet::CombineResourceRequest { msg, .. } => {
                let input_resources = extract_generic_resources(msg);

                Some(PlanetToExplorer::CombineResourceResponse {
                    complex_response: Err((
                        "This planet type can't combine resources.".to_string(),
                        input_resources.0,
                        input_resources.1
                    ))
                })
            },

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

impl AI {
    pub fn new() -> Self {
        AI {}
    }
}

/// Constructs a pair of [GenericResource] containing the two resources used in `request`.
fn extract_generic_resources(request: ComplexResourceRequest) -> (GenericResource, GenericResource) {
    match request {
        ComplexResourceRequest::Water(h, o) => (
            GenericResource::BasicResources(BasicResource::Hydrogen(h)),
            GenericResource::BasicResources(BasicResource::Oxygen(o))
        ),
        ComplexResourceRequest::Diamond(c1, c2) => (
            GenericResource::BasicResources(BasicResource::Carbon(c1)),
            GenericResource::BasicResources(BasicResource::Carbon(c2))
        ),
        ComplexResourceRequest::Life(w, c) => (
            GenericResource::ComplexResources(ComplexResource::Water(w)),
            GenericResource::BasicResources(BasicResource::Carbon(c))
        ),
        ComplexResourceRequest::Robot(s, l) => (
            GenericResource::BasicResources(BasicResource::Silicon(s)),
            GenericResource::ComplexResources(ComplexResource::Life(l))
        ),
        ComplexResourceRequest::Dolphin(w, l) => (
            GenericResource::ComplexResources(ComplexResource::Water(w)),
            GenericResource::ComplexResources(ComplexResource::Life(l))
        ),
        ComplexResourceRequest::AIPartner(r, d) => (
            GenericResource::ComplexResources(ComplexResource::Robot(r)),
            GenericResource::ComplexResources(ComplexResource::Diamond(d))
        )
    }
}