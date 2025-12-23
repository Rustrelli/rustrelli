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
//! - (TESTING) Fair-share resource generation between explorers + (WIP) Priority-based explorer request handling
//! - (TO BE DEFINED) Speculative resource generation to prevent sunray waste
//!   (e.g. in place resource generation when all cells are currently full based on the most requested type of resource by explorers to preemptively help them)

use crate::ExplorerRequestLimit;
use common_game::components::energy_cell::EnergyCell;
use common_game::components::planet::{DummyPlanetState, PlanetAI, PlanetState};
use common_game::components::resource::{
    BasicResource, BasicResourceType, Combinator, ComplexResource, ComplexResourceRequest,
    Generator, GenericResource,
};
use common_game::components::rocket::Rocket;
use common_game::components::sunray::Sunray;
use common_game::protocols::planet_explorer::{ExplorerToPlanet, PlanetToExplorer};
use std::collections::HashMap;
use std::time::{Duration, SystemTime};
// features:
// - user of the planet can choose between: fair-share resource generation between explorers or
//   explorers priority list to assign priority levels to each explorer -> planet tracks explorer requests to estimate resources usage
// - [probably cheating by game rules] speculative resource generation to prevent sunray waste (all cells are full),
//   based on generation requests history of specific explorers.

/// Struct for tracking statistics about the
/// generation requests made by an explorer to the planet.
struct StatsRecord {
    /// Usage score. Tracks the generation requests rate.
    score: f32,
    /// Timestamp of latest generation request.
    last_req: SystemTime,
}

impl Default for StatsRecord {
    fn default() -> Self {
        StatsRecord {
            score: 0.0,
            last_req: SystemTime::now(),
        }
    }
}

pub struct AI {
    explorer_stats: HashMap<u32, StatsRecord>,
    limit_mode: ExplorerRequestLimit,
}

impl AI {
    const CONTENTION_WINDOW: Duration = Duration::from_secs(3);
    const DECAY_RATE: f32 = 0.5;
    const INACTIVE_TIMESPAN: Duration = Duration::new(Self::CONTENTION_WINDOW.as_secs(), 0);
    const ALLOWED_REQ_BURST: f32 = 3.0;

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
    /// use rustrelli::ExplorerRequestLimit;
    /// use rustrelli::planet::AI;
    ///
    /// let ai = AI::new(ExplorerRequestLimit::None);
    /// ```
    pub fn new(limit_mode: ExplorerRequestLimit) -> Self {
        AI {
            explorer_stats: HashMap::new(),
            limit_mode,
        }
    }

    /// Applies linear decay to the usage scores of all tracked explorers.
    ///
    /// This method iterates through every explorer in the statistics map and reduces their
    /// score proportional to the time elapsed since their last request. The decay is calculated
    /// using `Self::DECAY_RATE`.
    ///
    /// The score is clamped at `0.0` to prevent negative usage values. If the elapsed time
    /// cannot be determined (e.g., due to system time errors), `Self::INACTIVE_TIMESPAN`
    /// is used as a fallback duration.
    fn decay_scores(&mut self) {
        for (_, stats) in self.explorer_stats.iter_mut() {
            stats.score = 0.0_f32.max(
                stats.score
                    - Self::DECAY_RATE
                        * stats
                            .last_req
                            .elapsed()
                            .unwrap_or(Self::INACTIVE_TIMESPAN)
                            .as_secs_f32(),
            )
        }
    }

    /// Increments the usage score for a specific explorer by the standard request cost.
    ///
    /// This represents the "heat" added to an explorer's tracking profile when they
    /// perform an action (like requesting a resource). The cost is currently fixed at `1.0`.
    ///
    /// # Arguments
    /// * `explorer_id` - The unique identifier of the explorer incurring the cost.
    ///
    /// # Notes
    /// This method uses `and_modify`, so it will **do nothing** if the `explorer_id`
    /// is not already present in `self.explorer_stats`. The explorer must be registered
    /// before costs can be added.
    fn add_req_cost(&mut self, explorer_id: u32) {
        self.explorer_stats
            .entry(explorer_id)
            .and_modify(|stats| stats.score += 1.0);
    }

    /// Retrieves the current usage score for a specific explorer.
    ///
    /// # Arguments
    /// * `explorer_id` - The unique identifier of the explorer to look up.
    ///
    /// # Returns
    /// * `Some(f32)` - The current score if the explorer is being tracked.
    /// * `None` - If the explorer is not found in the statistics.
    fn score(&self, explorer_id: u32) -> Option<f32> {
        self.explorer_stats
            .get(&explorer_id)
            .map(|stats| stats.score)
    }

    /// Calculates the average usage score across all currently tracked explorers.
    ///
    /// This metric is useful for determining the dynamic threshold for rate limiting.
    ///
    /// # Returns
    /// The arithmetic mean of all scores. Returns `NaN` if `self.explorer_stats` is empty.
    fn avg_score(&self) -> f32 {
        let mut sum = 0.0_f32;

        for (_, stats) in self.explorer_stats.iter() {
            sum += stats.score
        }
        sum / self.explorer_stats.len() as f32
    }

    /// Counts the number of explorers considered "active" at this moment.
    ///
    /// An explorer is defined as active if the time elapsed since their last request
    /// is less than the defined `Self::CONTENTION_WINDOW`.
    ///
    /// # Returns
    /// The count of explorers who have interacted with the planet recently enough to
    /// be considered competitors for resources.
    fn active_explorers(&self) -> u32 {
        self.explorer_stats
            .iter()
            .filter(|(_, stats)| {
                stats.last_req.elapsed().unwrap_or(Self::INACTIVE_TIMESPAN)
                    < Self::CONTENTION_WINDOW
            })
            .count() as u32
    }
}

impl PlanetAI for AI {
    fn handle_sunray(
        &mut self,
        state: &mut PlanetState,
        _generator: &Generator,
        _combinator: &Combinator,
        sunray: Sunray,
    ) {
        state.charge_cell(sunray);
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

    fn handle_internal_state_req(
        &mut self,
        state: &mut PlanetState,
        _generator: &Generator,
        _combinator: &Combinator,
    ) -> DummyPlanetState {
        state.to_dummy()
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
                if let Some((cell, _)) = state.full_cell() {
                    match self.limit_mode {
                        ExplorerRequestLimit::None => {
                            return Some(PlanetToExplorer::GenerateResourceResponse {
                                resource: Some(make_basic_resource(resource, cell, generator)),
                            });
                        }
                        ExplorerRequestLimit::FairShare => {}
                    }

                    // Add explorer_id entry to map if not already present
                    // then updates time of latest request.
                    self.explorer_stats
                        .entry(explorer_id)
                        .and_modify(|stats| stats.last_req = SystemTime::now())
                        .or_default();

                    // Apply the "Leaky Bucket" logic.
                    // First decay the score based on the time elapsed since the
                    // *previous* request (rewarding idle time), then add the cost of the *current* request.
                    self.decay_scores();
                    self.add_req_cost(explorer_id);

                    // Calculate Dynamic Tolerance.
                    // We adjust strictness based on contention.
                    // - Low contention (few active explorers): High tolerance. We allow bursts to maximize energy usage.
                    // - High contention (many active explorers): Low tolerance. We enforce strict equality to prevent hogging.
                    let active_explorers = self.active_explorers();
                    let tolerance: f32 = 1.0 + Self::ALLOWED_REQ_BURST / active_explorers as f32;

                    // Access to energy is granted if either:
                    // A) The explorer is the sole active user (Max Utilization Strategy).
                    //    We never want to waste energy if only one explorer is asking for it.
                    // B) The explorer's usage score is within the calculated tolerance of the group average.
                    let result = if active_explorers == 1
                        || self.score(explorer_id).unwrap() <= self.avg_score() * tolerance
                    {
                        // ACCESS GRANTED: Discharge the cell and produce the resource.
                        Some(make_basic_resource(resource, cell, generator))
                    } else {
                        // ACCESS DENIED: Rate limit exceeded.
                        // We return `None` to indicate the planet refused the request due to policy limits,
                        // preserving the energy cell for a "fairer" user.
                        None
                    };

                    Some(PlanetToExplorer::GenerateResourceResponse { resource: result })
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
}

impl Default for AI {
    fn default() -> Self {
        Self::new(ExplorerRequestLimit::None)
    }
}

/// Generates a basic resource based on the specified type.
///
/// This helper function uses the provided [`Generator`] and [`EnergyCell`] to produce
/// a concrete [`BasicResource`] corresponding to the `resource` type requested.
/// It wraps the result in the appropriate `BasicResource` variant.
///
/// # Panics
/// This function will panic if the generation fails (e.g., if the [`EnergyCell`] is not charged
/// or if the generation rule is missing). Callers must ensure preconditions are met.
///
/// # Arguments
/// * `resource` - The [`BasicResourceType`] indicating which resource to generate.
/// * `cell` - A mutable reference to an [`EnergyCell`] to be discharged during generation.
/// * `generator` - Reference to the [`Generator`] instance containing the generation rules.
///
/// # Returns
/// A [`BasicResource`] instance containing the newly generated resource.
fn make_basic_resource(
    resource: BasicResourceType,
    cell: &mut EnergyCell,
    generator: &Generator,
) -> BasicResource {
    match resource {
        BasicResourceType::Oxygen => BasicResource::Oxygen(generator.make_oxygen(cell).unwrap()),
        BasicResourceType::Hydrogen => {
            BasicResource::Hydrogen(generator.make_hydrogen(cell).unwrap())
        }
        BasicResourceType::Carbon => BasicResource::Carbon(generator.make_carbon(cell).unwrap()),
        BasicResourceType::Silicon => BasicResource::Silicon(generator.make_silicon(cell).unwrap()),
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
