pub mod bsl_psd;

// Common algorithm traits
use crate::models::{Location, ShoppingList, ShoppingRoute};

/// Trait for Personal Shopper's Dilemma solvers
pub trait PSDSolver {
    /// Solve the Personal Shopper's Dilemma and return a skyline of routes
    fn solve(
        &self,
        shopping_list: &ShoppingList,
        shopper_location: Location,
        customer_location: Location,
    ) -> Vec<ShoppingRoute>;

    /// Check if a route satisfies a shopping list
    fn satisfies_list(&self, route: &[u32], shopping_list: &ShoppingList) -> bool;

    /// Calculate shopping time for a route
    fn calculate_shopping_time(
        &self,
        route: &[u32],
        shopper_location: Location,
        customer_location: Location,
    ) -> f64;

    /// Calculate shopping cost for a route
    fn calculate_shopping_cost(&self, route: &[u32], shopping_list: &ShoppingList) -> f64;
}
