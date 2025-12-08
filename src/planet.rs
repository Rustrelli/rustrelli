//! Planet AI implementation module.
//!
//! This module contains the AI logic for a Type D planet, which handles:
//! - Sunray reception and energy cell charging
//! - Basic resource generation requests from explorers
//! - Internal state queries from the orchestrator
//! - Explorer capability queries (supported resources and combinations)
//!
//! ## Type D Planet Characteristics
//!
//! - **Energy Cells**: 5 cells for storing sunrays
//! - **Generation**: Can generate all basic resource types (unbounded rules)
//! - **Combination**: Cannot combine resources (0 combination rules)
//! - **Survival**: Cannot build rockets, will be destroyed by asteroids
//!
//! ## Future Features
//!
//! Planned enhancements include:
//! - (IN PROGRESS) Fair-share resource generation between explorers + Priority-based explorer request handling
//! - (TO BE DEFINED) Speculative resource generation to prevent sunray waste
//!   (eg. in place resource generation when all cells are currently full based on the most requested type of resource by explorers to preemptively help them)

use common_game::components::planet::{PlanetAI, PlanetState};
use common_game::components::resource::{
    BasicResource, BasicResourceType, Combinator, ComplexResource, ComplexResourceRequest,
    Generator, GenericResource,
};
use common_game::components::rocket::Rocket;
use common_game::protocols::messages::{
    ExplorerToPlanet, OrchestratorToPlanet, PlanetToExplorer, PlanetToOrchestrator,
};

pub struct AI {}

impl PlanetAI for AI {
    fn handle_orchestrator_msg(
        &mut self,
        state: &mut PlanetState,
        _generator: &Generator,
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
                explorer_id: _,
                resource,
            } => {
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
                        input_resources.1,
                    )),
                })
            }

            ExplorerToPlanet::AvailableEnergyCellRequest { .. } => {
                Some(PlanetToExplorer::AvailableEnergyCellResponse {
                    available_cells: state.to_dummy().charged_cells_count as u32,
                })
            }
        }
    }

    fn handle_asteroid(
        &mut self,
        _state: &mut PlanetState,
        _generator: &Generator,
        _combinator: &Combinator,
    ) -> Option<Rocket> {
        // Type D planets cannot build rockets, so they will be destroyed by asteroids
        None
    }

    fn start(&mut self, _state: &PlanetState) {
        // Planet AI startup logic
        // Currently no initialization needed for Type D planet
    }

    fn stop(&mut self, _state: &PlanetState) {
        // Planet AI shutdown logic
        // Currently no cleanup needed for Type D planet
    }
}

impl Default for AI {
    fn default() -> Self {
        Self::new()
    }
}

impl AI {
    /// Creates a new AI instance.
    ///
    /// This constructor initializes an empty AI struct that implements
    /// the planet's behavior through the [`PlanetAI`] trait.
    ///
    /// # Returns
    /// A new `AI` instance ready to be passed to [`Planet::new`](common_game::components::planet::Planet::new).
    ///
    /// # Examples
    /// ```
    /// use rustrelli::planet::AI;
    ///
    /// let ai = AI::new();
    /// ```
    pub fn new() -> Self {
        AI {}
    }
}

/// Extracts the two resources from a complex resource request.
///
/// This helper function deconstructs a [`ComplexResourceRequest`] and wraps each
/// of its constituent resources into a [`GenericResource`] variant, preserving
/// whether each resource is basic or complex.
///
/// This is useful when handling failed combination attempts, as it allows the planet
/// to return the original resources to the explorer in a uniform format.
///
/// # Arguments
/// * `request` - A [`ComplexResourceRequest`] containing two resources to be combined
///
/// # Returns
/// A tuple of two [`GenericResource`] instances, each wrapping one of the resources
/// from the request. The order matches the combination recipe requirements.
///
/// # Examples
/// ```ignore
/// use common_game::components::resource::{ComplexResourceRequest, GenericResource};
///
/// let request = ComplexResourceRequest::Water(hydrogen, oxygen);
/// let (res1, res2) = extract_generic_resources(request);
/// ```
fn extract_generic_resources(
    request: ComplexResourceRequest,
) -> (GenericResource, GenericResource) {
    match request {
        ComplexResourceRequest::Water(h, o) => (
            GenericResource::BasicResources(BasicResource::Hydrogen(h)),
            GenericResource::BasicResources(BasicResource::Oxygen(o)),
        ),
        ComplexResourceRequest::Diamond(c1, c2) => (
            GenericResource::BasicResources(BasicResource::Carbon(c1)),
            GenericResource::BasicResources(BasicResource::Carbon(c2)),
        ),
        ComplexResourceRequest::Life(w, c) => (
            GenericResource::ComplexResources(ComplexResource::Water(w)),
            GenericResource::BasicResources(BasicResource::Carbon(c)),
        ),
        ComplexResourceRequest::Robot(s, l) => (
            GenericResource::BasicResources(BasicResource::Silicon(s)),
            GenericResource::ComplexResources(ComplexResource::Life(l)),
        ),
        ComplexResourceRequest::Dolphin(w, l) => (
            GenericResource::ComplexResources(ComplexResource::Water(w)),
            GenericResource::ComplexResources(ComplexResource::Life(l)),
        ),
        ComplexResourceRequest::AIPartner(r, d) => (
            GenericResource::ComplexResources(ComplexResource::Robot(r)),
            GenericResource::ComplexResources(ComplexResource::Diamond(d)),
        ),
    }
}
