//! Integration tests for planet behavior.
//!
//! Tests verify core functionality through realistic message-passing scenarios:
//! - Energy cell charging via sunrays
//! - Resource generation and energy consumption
//! - State queries and explorer coordination
//!
//! Each test documents its scenario and validation goals.

use common_game::components::resource::BasicResourceType;
use common_game::components::sunray::Sunray;
use common_game::protocols::orchestrator_planet::{
    PlanetToOrchestrator, OrchestratorToPlanet
};
use common_game::protocols::planet_explorer::{
    PlanetToExplorer, ExplorerToPlanet
};
use rustrelli::{create_planet, ExplorerRequestLimit};
use std::thread;
use std::time::Duration;
use crossbeam_channel::{unbounded, Receiver, Sender};
// ============================================================================
// Test Helpers
// ============================================================================

#[allow(clippy::type_complexity)]
fn setup_test_planet() -> (
    Sender<OrchestratorToPlanet>,
    Receiver<PlanetToOrchestrator>,
    Sender<ExplorerToPlanet>,
    thread::JoinHandle<Result<(), String>>,
) {
    let (tx_orch_to_planet, rx_orch_to_planet) = unbounded();
    let (tx_planet_to_orch, rx_planet_to_orch) = unbounded();
    let (tx_expl_to_planet, rx_expl_to_planet) = unbounded();

    let mut planet = create_planet(rx_orch_to_planet, tx_planet_to_orch, rx_expl_to_planet, ExplorerRequestLimit::None);

    let handle = thread::spawn(move || {
        planet.run()
    });

    tx_orch_to_planet
        .send(OrchestratorToPlanet::StartPlanetAI)
        .unwrap();
    rx_planet_to_orch.recv().unwrap();
    thread::sleep(Duration::from_millis(50));

    (
        tx_orch_to_planet,
        rx_planet_to_orch,
        tx_expl_to_planet,
        handle,
    )
}

fn register_explorer(
    explorer_id: u32,
    tx_orch: &Sender<OrchestratorToPlanet>,
    rx_orch: &Receiver<PlanetToOrchestrator>,
) -> Receiver<PlanetToExplorer> {
    let (tx_planet_to_expl, rx_planet_to_expl) = unbounded();
    tx_orch
        .send(OrchestratorToPlanet::IncomingExplorerRequest {
            explorer_id,
            new_sender: tx_planet_to_expl,
        })
        .unwrap();
    let _ = rx_orch.recv_timeout(Duration::from_millis(200));
    rx_planet_to_expl
}

fn charge_cells(
    count: usize,
    tx_orch: &Sender<OrchestratorToPlanet>,
    rx_orch: &Receiver<PlanetToOrchestrator>,
) {
    for _ in 0..count {
        tx_orch
            .send(OrchestratorToPlanet::Sunray(Sunray::default()))
            .unwrap();
        let _ = rx_orch.recv_timeout(Duration::from_millis(200));
    }
}

// ============================================================================
// Tests: Planet State & Configuration
// ============================================================================

/// **Scenario:** Orchestrator queries planet state
/// **Validates:** Correct ID, 5 cells, no initial energy, no rocket
#[test]
fn test_internal_state_query() {
    let (tx_orch, rx_orch, _, _) = setup_test_planet();

    tx_orch
        .send(OrchestratorToPlanet::InternalStateRequest)
        .unwrap();

    // handle.join().unwrap().unwrap();

    match rx_orch.recv() {
        Ok(PlanetToOrchestrator::InternalStateResponse {
            planet_id,
            planet_state,
        }) => {
            assert_eq!(planet_id, 1);
            assert_eq!(planet_state.energy_cells.len(), 5);
            assert_eq!(planet_state.charged_cells_count, 0);
            assert!(!planet_state.has_rocket);
        }
        other => panic!("Expected InternalStateResponse, got {:?}", other),
    }
}

// ============================================================================
// Tests: Explorer Queries
// ============================================================================

/// **Scenario:** Explorer queries planet capabilities
/// **Validates:**
/// - Type D reports 4 basic resources (O, H, C, Si)
/// - Type D reports 0 combinations (generator-only)
/// - Reports 0 available energy initially
#[test]
fn test_explorer_capability_queries() {
    let (tx_orch, rx_orch, tx_expl, _) = setup_test_planet();
    let explorer_id = 42;
    let rx_expl = register_explorer(explorer_id, &tx_orch, &rx_orch);

    // Test supported resources
    tx_expl
        .send(ExplorerToPlanet::SupportedResourceRequest { explorer_id })
        .unwrap();
    match rx_expl.recv_timeout(Duration::from_millis(200)) {
        Ok(PlanetToExplorer::SupportedResourceResponse { resource_list }) => {
            assert_eq!(resource_list.len(), 4);
            assert!(resource_list.contains(&BasicResourceType::Oxygen));
        }
        _ => panic!("Expected SupportedResourceResponse"),
    }

    // Test supported combinations
    tx_expl
        .send(ExplorerToPlanet::SupportedCombinationRequest { explorer_id })
        .unwrap();
    match rx_expl.recv_timeout(Duration::from_millis(200)) {
        Ok(PlanetToExplorer::SupportedCombinationResponse { combination_list }) => {
            assert_eq!(combination_list.len(), 0, "Type D has no combinations");
        }
        _ => panic!("Expected SupportedCombinationResponse"),
    }

    // Test available energy cells
    tx_expl
        .send(ExplorerToPlanet::AvailableEnergyCellRequest { explorer_id })
        .unwrap();
    match rx_expl.recv_timeout(Duration::from_millis(200)) {
        Ok(PlanetToExplorer::AvailableEnergyCellResponse { available_cells }) => {
            assert_eq!(available_cells, 0, "No energy initially");
        }
        _ => panic!("Expected AvailableEnergyCellResponse"),
    }
}

/// **Scenario:** Explorer leaves, tries to communicate
/// **Validates:** Planet doesn't respond to removed explorers
#[test]
fn test_explorer_removal() {
    let (tx_orch, rx_orch, tx_expl, _) = setup_test_planet();
    let explorer_id = 42;
    let rx_expl = register_explorer(explorer_id, &tx_orch, &rx_orch);

    // Works before removal
    tx_expl
        .send(ExplorerToPlanet::SupportedResourceRequest { explorer_id })
        .unwrap();
    assert!(rx_expl.recv_timeout(Duration::from_millis(200)).is_ok());

    // Remove explorer
    tx_orch
        .send(OrchestratorToPlanet::OutgoingExplorerRequest { explorer_id })
        .unwrap();
    let _ = rx_orch.recv_timeout(Duration::from_millis(200));
    thread::sleep(Duration::from_millis(50));

    // No response after removal
    tx_expl
        .send(ExplorerToPlanet::SupportedResourceRequest { explorer_id })
        .unwrap();
    assert!(
        rx_expl.recv_timeout(Duration::from_millis(200)).is_err(),
        "Planet shouldn't respond to removed explorer"
    );
}

/// **Scenario:** Multiple explorers communicate simultaneously
/// **Validates:** Each gets isolated responses on their channel
#[test]
fn test_multiple_explorers() {
    let (tx_orch, rx_orch, tx_expl, _) = setup_test_planet();

    let rx_expl1 = register_explorer(1, &tx_orch, &rx_orch);
    let rx_expl2 = register_explorer(2, &tx_orch, &rx_orch);

    // Both query simultaneously
    tx_expl
        .send(ExplorerToPlanet::SupportedResourceRequest { explorer_id: 1 })
        .unwrap();
    tx_expl
        .send(ExplorerToPlanet::SupportedResourceRequest { explorer_id: 2 })
        .unwrap();

    // Both receive responses
    assert!(rx_expl1.recv_timeout(Duration::from_millis(200)).is_ok());
    assert!(rx_expl2.recv_timeout(Duration::from_millis(200)).is_ok());
}

// ============================================================================
// Tests: Energy Charging (Sunrays)
// ============================================================================

/// **Scenario:** Orchestrator sends 1 sunray
/// **Validates:** Cell is charged, planet acknowledges
#[test]
fn test_single_sunray_charges_cell() {
    let (tx_orch, rx_orch, _, _) = setup_test_planet();

    tx_orch
        .send(OrchestratorToPlanet::Sunray(Sunray::default()))
        .unwrap();

    // Verify acknowledgment
    match rx_orch.recv_timeout(Duration::from_millis(200)) {
        Ok(PlanetToOrchestrator::SunrayAck { planet_id }) => assert_eq!(planet_id, 1),
        _ => panic!("Expected SunrayAck"),
    }

    // Verify cell charged
    tx_orch
        .send(OrchestratorToPlanet::InternalStateRequest)
        .unwrap();
    match rx_orch.recv_timeout(Duration::from_millis(200)) {
        Ok(PlanetToOrchestrator::InternalStateResponse { planet_state, .. }) => {
            assert_eq!(planet_state.charged_cells_count, 1);
            assert!(planet_state.energy_cells[0]);
        }
        _ => panic!("Expected InternalStateResponse"),
    }
}

/// **Scenario:** Orchestrator sends 3 sunrays
/// **Validates:** Cells 0-2 charged, 3-4 empty
#[test]
fn test_multiple_sunrays_charge_sequentially() {
    let (tx_orch, rx_orch, _, _) = setup_test_planet();

    charge_cells(3, &tx_orch, &rx_orch);

    tx_orch
        .send(OrchestratorToPlanet::InternalStateRequest)
        .unwrap();
    match rx_orch.recv_timeout(Duration::from_millis(200)) {
        Ok(PlanetToOrchestrator::InternalStateResponse { planet_state, .. }) => {
            assert_eq!(planet_state.charged_cells_count, 3);
            assert!(planet_state.energy_cells[0]);
            assert!(planet_state.energy_cells[1]);
            assert!(planet_state.energy_cells[2]);
            assert!(!planet_state.energy_cells[3]);
            assert!(!planet_state.energy_cells[4]);
        }
        _ => panic!("Expected InternalStateResponse"),
    }
}

/// **Scenario:** Fill all 5 cells with sunrays
/// **Validates:** All cells charged (maximum capacity)
#[test]
fn test_all_cells_can_be_charged() {
    let (tx_orch, rx_orch, _, _) = setup_test_planet();

    charge_cells(5, &tx_orch, &rx_orch);

    tx_orch
        .send(OrchestratorToPlanet::InternalStateRequest)
        .unwrap();
    match rx_orch.recv_timeout(Duration::from_millis(200)) {
        Ok(PlanetToOrchestrator::InternalStateResponse { planet_state, .. }) => {
            assert_eq!(planet_state.charged_cells_count, 5);
            for (i, charged) in planet_state.energy_cells.iter().enumerate() {
                assert!(charged, "Cell {} should be charged", i);
            }
        }
        _ => panic!("Expected InternalStateResponse"),
    }
}

// ============================================================================
// Tests: Resource Generation & Energy Consumption
// ============================================================================

/// **Scenario:** Explorer requests resource without energy
/// **Validates:** Planet returns None (no resource generated)
#[test]
fn test_generation_fails_without_energy() {
    let (tx_orch, rx_orch, tx_expl, _) = setup_test_planet();
    let explorer_id = 42;
    let rx_expl = register_explorer(explorer_id, &tx_orch, &rx_orch);

    tx_expl
        .send(ExplorerToPlanet::GenerateResourceRequest {
            explorer_id,
            resource: BasicResourceType::Carbon,
        })
        .unwrap();

    match rx_expl.recv_timeout(Duration::from_millis(200)) {
        Ok(PlanetToExplorer::GenerateResourceResponse { resource }) => {
            assert!(resource.is_none(), "Should fail without energy");
        }
        _ => panic!("Expected GenerateResourceResponse"),
    }
}

/// **Scenario:** Charge 1 cell, generate resource, check discharge
/// **Validates:**
/// - Resource generated successfully
/// - Cell count: 1 → 0 (energy consumed)
#[test]
fn test_generation_consumes_energy() {
    let (tx_orch, rx_orch, tx_expl, _) = setup_test_planet();
    let explorer_id = 42;
    let rx_expl = register_explorer(explorer_id, &tx_orch, &rx_orch);

    charge_cells(1, &tx_orch, &rx_orch);

    // Generate resource
    tx_expl
        .send(ExplorerToPlanet::GenerateResourceRequest {
            explorer_id,
            resource: BasicResourceType::Oxygen,
        })
        .unwrap();

    match rx_expl.recv_timeout(Duration::from_millis(200)) {
        Ok(PlanetToExplorer::GenerateResourceResponse { resource }) => {
            assert!(resource.is_some(), "Should generate with energy");
        }
        _ => panic!("Expected GenerateResourceResponse"),
    }

    // Verify energy consumed
    thread::sleep(Duration::from_millis(50));
    tx_orch
        .send(OrchestratorToPlanet::InternalStateRequest)
        .unwrap();
    match rx_orch.recv_timeout(Duration::from_millis(200)) {
        Ok(PlanetToOrchestrator::InternalStateResponse { planet_state, .. }) => {
            assert_eq!(planet_state.charged_cells_count, 0, "Energy consumed");
        }
        _ => panic!("Expected InternalStateResponse"),
    }
}

/// **Scenario:** Charge 3 cells, generate 3 resources, attempt 4th
/// **Validates:** Energy decreases correctly (3→2→1→0), 4th fails
#[test]
fn test_sequential_energy_consumption() {
    let (tx_orch, rx_orch, tx_expl, _) = setup_test_planet();
    let explorer_id = 42;
    let rx_expl = register_explorer(explorer_id, &tx_orch, &rx_orch);

    charge_cells(3, &tx_orch, &rx_orch);

    let resources = [
        BasicResourceType::Carbon,
        BasicResourceType::Hydrogen,
        BasicResourceType::Silicon,
    ];

    // Generate 3 resources, verify count decreases
    for (i, resource_type) in resources.iter().enumerate() {
        tx_expl
            .send(ExplorerToPlanet::GenerateResourceRequest {
                explorer_id,
                resource: *resource_type,
            })
            .unwrap();
        let _ = rx_expl.recv_timeout(Duration::from_millis(200));

        thread::sleep(Duration::from_millis(50));
        tx_orch
            .send(OrchestratorToPlanet::InternalStateRequest)
            .unwrap();
        match rx_orch.recv_timeout(Duration::from_millis(200)) {
            Ok(PlanetToOrchestrator::InternalStateResponse { planet_state, .. }) => {
                assert_eq!(
                    planet_state.charged_cells_count,
                    3 - (i + 1),
                    "After {} generations",
                    i + 1
                );
            }
            _ => panic!("Expected InternalStateResponse"),
        }
    }

    // Try 4th generation - should fail
    tx_expl
        .send(ExplorerToPlanet::GenerateResourceRequest {
            explorer_id,
            resource: BasicResourceType::Oxygen,
        })
        .unwrap();
    match rx_expl.recv_timeout(Duration::from_millis(200)) {
        Ok(PlanetToExplorer::GenerateResourceResponse { resource }) => {
            assert!(resource.is_none(), "Should fail without energy");
        }
        _ => panic!("Expected GenerateResourceResponse"),
    }
}

/// **Scenario:** Explorer queries available cells after charging
/// **Validates:** Planet reports correct count (2) after 2 sunrays
#[test]
fn test_availability_query_after_charging() {
    let (tx_orch, rx_orch, tx_expl, _) = setup_test_planet();
    let explorer_id = 42;
    let rx_expl = register_explorer(explorer_id, &tx_orch, &rx_orch);

    charge_cells(2, &tx_orch, &rx_orch);

    tx_expl
        .send(ExplorerToPlanet::AvailableEnergyCellRequest { explorer_id })
        .unwrap();

    match rx_expl.recv_timeout(Duration::from_millis(200)) {
        Ok(PlanetToExplorer::AvailableEnergyCellResponse { available_cells }) => {
            assert_eq!(available_cells, 2);
        }
        _ => panic!("Expected AvailableEnergyCellResponse"),
    }
}
