//! # Rustrelli Planet Implementation
//!
//! This crate provides a Type D planet implementation for the space exploration game.
//! The planet can generate basic resources (Carbon, Silicon, Oxygen, Hydrogen) but
//! cannot combine them into complex resources.
//!
//! ## Example
//! ```
//! use std::sync::mpsc;
//! use rustrelli::create_planet;
//!
//! let (tx_orch, rx_orch) = mpsc::channel();
//! let (tx_planet, rx_planet) = mpsc::channel();
//! let (tx_expl, rx_expl) = mpsc::channel();
//!
//! let planet = create_planet(rx_orch, tx_planet, rx_expl);
//! ```

pub mod planet;

use common_game::components::planet::{Planet, PlanetType};
use common_game::components::resource::BasicResourceType;
use common_game::protocols::messages;
use planet::AI;
use std::sync::mpsc;

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
/// use std::sync::mpsc;
/// use rustrelli::create_planet;
///
/// let (tx_orch_to_planet, rx_orch_to_planet) = mpsc::channel();
/// let (tx_planet_to_orch, rx_planet_to_orch) = mpsc::channel();
/// let (tx_expl_to_planet, rx_expl_to_planet) = mpsc::channel();
///
/// let planet = create_planet(
///     rx_orch_to_planet,
///     tx_planet_to_orch,
///     rx_expl_to_planet,
/// );
/// ```
pub fn create_planet(
    rx_orchestrator: mpsc::Receiver<messages::OrchestratorToPlanet>,
    tx_orchestrator: mpsc::Sender<messages::PlanetToOrchestrator>,
    rx_explorer: mpsc::Receiver<messages::ExplorerToPlanet>,
) -> Planet {
    let id = 1;
    let ai = AI::new();
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
    //! Unit tests for planet construction and configuration.
    //!
    //! These tests validate that `create_planet()` properly initializes a Type D planet
    //! with the correct specifications. Unlike integration tests, these tests directly
    //! access planet internals without running the message-passing loop.

    use super::*;
    use std::sync::mpsc;

    // ============================================================================
    // Test Helper
    // ============================================================================

    fn create_test_channels() -> (
        mpsc::Receiver<messages::OrchestratorToPlanet>,
        mpsc::Sender<messages::PlanetToOrchestrator>,
        mpsc::Receiver<messages::ExplorerToPlanet>,
    ) {
        let (_tx_orch_to_planet, rx_orch_to_planet) = mpsc::channel();
        let (tx_planet_to_orch, _rx_planet_to_orch) = mpsc::channel();
        let (_tx_expl_to_planet, rx_expl_to_planet) = mpsc::channel();

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
        let planet = create_planet(rx_orch, tx_orch, rx_expl);

        assert_eq!(planet.id(), 1, "Planet ID should be 1");
        assert_eq!(
            planet.planet_type() as u8,
            PlanetType::D as u8,
            "Planet type should be D"
        );

        let ai = planet::AI::new();
        assert_eq!(
            std::mem::size_of_val(&ai),
            0,
            "AI should be zero-sized (stateless)"
        );
    }

    /// **Scenario:** Verify Type D generation capabilities
    /// **Validates:**
    /// - Supports exactly 4 basic resource types
    /// - Includes Carbon, Silicon, Oxygen, Hydrogen
    #[test]
    fn test_planet_generation_rules() {
        let (rx_orch, tx_orch, rx_expl) = create_test_channels();
        let planet = create_planet(rx_orch, tx_orch, rx_expl);
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
        let planet = create_planet(rx_orch, tx_orch, rx_expl);
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
        let planet = create_planet(rx_orch, tx_orch, rx_expl);

        assert_eq!(planet.state().cells_count(), 5, "Type D has 5 energy cells");
        assert!(
            !planet.state().can_have_rocket(),
            "Type D cannot build rockets"
        );
        assert!(!planet.state().has_rocket(), "No initial rocket");
    }
}
