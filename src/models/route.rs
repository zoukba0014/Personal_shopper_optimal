// Route models for representing shopping routes

use crate::models::{Cost, StoreId, Time};
use std::cmp::Ordering;

/// Represents a complete shopping route with cost information
#[derive(Debug, Clone, PartialEq)]
pub struct ShoppingRoute {
    /// Sequence of stores to visit
    pub stores: Vec<StoreId>,

    /// Total shopping time
    pub shopping_time: Time,

    /// Total shopping cost
    pub shopping_cost: Cost,
}

impl ShoppingRoute {
    /// Creates a new shopping route
    pub fn new(stores: Vec<StoreId>, shopping_time: Time, shopping_cost: Cost) -> Self {
        Self {
            stores,
            shopping_time,
            shopping_cost,
        }
    }

    /// Checks if this route conventionally dominates another route
    /// A route conventionally dominates another if it is better or equal in all dimensions
    /// and strictly better in at least one dimension
    pub fn conventionally_dominates(&self, other: &ShoppingRoute) -> bool {
        let condition1 =
            self.shopping_time < other.shopping_time && self.shopping_cost <= other.shopping_cost;
        let condition2 =
            self.shopping_time <= other.shopping_time && self.shopping_cost < other.shopping_cost;
        // Add comparison of store count for the equal case
        let condition3 = self.shopping_time == other.shopping_time
            && self.shopping_cost == other.shopping_cost
            && self.stores.len() < other.stores.len();

        condition1 || condition2 || condition3
    }
}

/// Candidate route used in the priority queue for route generation
#[derive(Debug, Clone, PartialEq)]
pub struct RouteCandidate {
    /// Sequence of stores to visit
    pub stores: Vec<StoreId>,

    /// Total shopping time (used for ordering)
    pub shopping_time: Time,
}

// Custom ordering for min-priority queue based on shopping time
impl PartialOrd for RouteCandidate {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        // For floating point comparison in priority queue
        // Convert to ordered floating point representation for comparison
        // Reversed to create min-heap instead of default max-heap
        other.shopping_time.partial_cmp(&self.shopping_time)
    }
}

impl Ord for RouteCandidate {
    fn cmp(&self, other: &Self) -> Ordering {
        self.partial_cmp(other).unwrap_or(Ordering::Equal)
    }
}
impl Eq for RouteCandidate {}
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_route_candidate_ordering() {
        let route1 = RouteCandidate {
            stores: vec![1, 2],
            shopping_time: 10.0,
        };
        let route2 = RouteCandidate {
            stores: vec![1, 3],
            shopping_time: 15.0,
        };

        // In a min-heap, the lesser element comes first
        assert!(route1 > route2);
    }
}
