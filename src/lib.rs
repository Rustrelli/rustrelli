//! # Rustrelli Planet Implementation
//!
//! This crate provides a Type D planet implementation for the space exploration game.
//! The planet can generate basic resources (Carbon, Silicon, Oxygen, Hydrogen) but
//! cannot combine them into complex resources.
//!
//! ## Example
//! ```
//! use crossbeam_channel::{Receiver, Sender, bounded};
//! use rustrelli::{create_planet, ExplorerRequestLimit};
//!
//! let (tx_orch, rx_orch) = bounded(10);
//! let (tx_planet, rx_planet) = bounded(10);
//! let (tx_expl, rx_expl) = bounded(10);
//!
//! let planet = create_planet(1, rx_orch, tx_planet, rx_expl, ExplorerRequestLimit::None);
//! ```

pub mod planet;

use common_game::components::planet::{Planet, PlanetType};
use common_game::components::resource::BasicResourceType;
use common_game::protocols::*;
use common_game::utils::ID;
use planet::AI;

use crossbeam_channel::{Receiver, Sender};

/// Creates and configures a Type D planet.
///
/// This function initializes a planet with the following configuration:
/// - **Planet Type**: D (5 energy cells, unbounded generation, no rockets, no combinations)
/// - **Generation Rules**: Carbon, Silicon, Oxygen, Hydrogen
/// - **Combination Rules**: None
///
/// The planet is created with a custom AI that handles incoming messages from
/// the orchestrator and explorers according to the Type D planet specifications.
///
/// # Arguments
/// * `rx_orchestrator` - Receiver for messages from the orchestrator
/// * `tx_orchestrator` - Sender for messages to the orchestrator
/// * `rx_explorer` - Receiver for messages from explorers
/// * `request_limit` - One of the available modes to limit resource generation requests done by
///   explorers (see [ExplorerRequestLimit])
///
/// # Returns
/// /// A configured [`Planet`] instance ready to run.
///
/// # Panics
/// Panics if the planet construction fails due to invalid configuration.
/// This should not happen with the hardcoded configuration provided.
///
/// # Examples
/// ```
/// use crossbeam_channel::{Receiver, Sender, bounded};
/// use rustrelli::{create_planet, ExplorerRequestLimit};
///
/// let (tx_orch_to_planet, rx_orch_to_planet) = bounded(20);
/// let (tx_planet_to_orch, rx_planet_to_orch) = bounded(20);
/// let (tx_expl_to_planet, rx_expl_to_planet) = bounded(20);
///
/// let planet = create_planet(
///     1,
///     rx_orch_to_planet,
///     tx_planet_to_orch,
///     rx_expl_to_planet,
///     ExplorerRequestLimit::None
/// );
/// ```
pub fn create_planet(
    id: ID,
    rx_orchestrator: Receiver<orchestrator_planet::OrchestratorToPlanet>,
    tx_orchestrator: Sender<orchestrator_planet::PlanetToOrchestrator>,
    rx_explorer: Receiver<planet_explorer::ExplorerToPlanet>,
    request_limit: ExplorerRequestLimit,
) -> Planet {
    let ai = AI::new(request_limit);
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

/// Available explorer limiting modes.
pub enum ExplorerRequestLimit {
    /// No limit to explorer requests.
    None,
    /// Tries to share energy cells usage equally between active explorers.
    /// Uses an algorithm similar to [Token Bucket](https://en.wikipedia.org/wiki/Token_bucket).
    FairShare,
}

#[cfg(test)]
mod tests {
    //! Unit tests for planet construction and configuration.
    //!
    //! These tests validate that `create_planet()` properly initializes a Type D planet
    //! with the correct specifications. Unlike integration tests, these tests directly
    //! access planet internals without running the message-passing loop.

    use super::*;
    use crossbeam_channel::unbounded;

    // ============================================================================
    // Test Helper
    // ============================================================================

    fn create_test_channels() -> (
        Receiver<orchestrator_planet::OrchestratorToPlanet>,
        Sender<orchestrator_planet::PlanetToOrchestrator>,
        Receiver<planet_explorer::ExplorerToPlanet>,
    ) {
        let (_tx_orch_to_planet, rx_orch_to_planet) = unbounded();
        let (tx_planet_to_orch, _rx_planet_to_orch) = unbounded();
        let (_tx_expl_to_planet, rx_expl_to_planet) = unbounded();

        (rx_orch_to_planet, tx_planet_to_orch, rx_expl_to_planet)
    }

    // ============================================================================
    // Tests: Planet Construction
    // ============================================================================

    /// **Scenario:** Create planet with standard configuration
    /// **Validates:**
    /// - Planet ID is 1
    /// - Planet type is D
    /// - AI is zero-sized (stateless)
    #[test]
    fn test_planet_basic_configuration() {
        let (rx_orch, tx_orch, rx_expl) = create_test_channels();
        let planet = create_planet(1, rx_orch, tx_orch, rx_expl, ExplorerRequestLimit::None);

        assert_eq!(planet.id(), 1, "Planet ID should be 1");
        assert_eq!(
            planet.planet_type() as u8,
            PlanetType::D as u8,
            "Planet type should be D"
        );
    }

    /// **Scenario:** Verify Type D generation capabilities
    /// **Validates:**
    /// - Supports exactly 4 basic resource types
    /// - Includes Carbon, Silicon, Oxygen, Hydrogen
    #[test]
    fn test_planet_generation_rules() {
        let (rx_orch, tx_orch, rx_expl) = create_test_channels();
        let planet = create_planet(1, rx_orch, tx_orch, rx_expl, ExplorerRequestLimit::None);
        let recipes = planet.generator().all_available_recipes();

        assert_eq!(recipes.len(), 4, "Type D supports 4 basic resources");
        assert!(
            recipes.contains(&BasicResourceType::Carbon),
            "Should support Carbon"
        );
        assert!(
            recipes.contains(&BasicResourceType::Silicon),
            "Should support Silicon"
        );
        assert!(
            recipes.contains(&BasicResourceType::Oxygen),
            "Should support Oxygen"
        );
        assert!(
            recipes.contains(&BasicResourceType::Hydrogen),
            "Should support Hydrogen"
        );
    }

    /// **Scenario:** Verify Type D combination limitations
    /// **Validates:** Type D cannot combine resources (0 combination rules)
    #[test]
    fn test_planet_combination_rules() {
        let (rx_orch, tx_orch, rx_expl) = create_test_channels();
        let planet = create_planet(1, rx_orch, tx_orch, rx_expl, ExplorerRequestLimit::None);
        let recipes = planet.combinator().all_available_recipes();

        assert_eq!(
            recipes.len(),
            0,
            "Type D cannot combine resources (generator-only)"
        );
    }

    /// **Scenario:** Verify initial planet state
    /// **Validates:**
    /// - Planet has 5 energy cells (Type D specification)
    /// - Cannot have rockets (will be destroyed by asteroids)
    /// - No rocket present
    #[test]
    fn test_planet_initial_state() {
        let (rx_orch, tx_orch, rx_expl) = create_test_channels();
        let planet = create_planet(1, rx_orch, tx_orch, rx_expl, ExplorerRequestLimit::None);

        assert_eq!(planet.state().cells_count(), 5, "Type D has 5 energy cells");
        assert!(
            !planet.state().can_have_rocket(),
            "Type D cannot build rockets"
        );
        assert!(!planet.state().has_rocket(), "No initial rocket");
    }
}
