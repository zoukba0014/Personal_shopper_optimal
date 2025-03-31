// Store model representing shopping locations with inventory tracking

use crate::models::{Location, Product, ProductId, StoreId};
use std::collections::HashMap;

/// Represents a store where products can be purchased
#[derive(Debug, Clone)]
pub struct Store {
    /// Unique identifier for the store
    pub id: StoreId,

    /// Geographic location of the store
    pub location: Location,

    /// Collection of products available at this store
    pub products: HashMap<ProductId, Product>,

    /// Inventory tracking for each product
    pub inventory: HashMap<ProductId, u32>,
}

impl Store {
    /// Creates a new store with the given ID, location, and products
    pub fn new(id: StoreId, location: Location, products: HashMap<ProductId, Product>) -> Self {
        // Initialize inventory with default quantities (10 units each)
        let inventory = products
            .keys()
            .map(|product_id| (*product_id, 10))
            .collect();

        Self {
            id,
            location,
            products,
            inventory,
        }
    }

    /// Creates a new store with explicit inventory control
    pub fn new_with_inventory(
        id: StoreId,
        location: Location,
        products: HashMap<ProductId, Product>,
        inventory: HashMap<ProductId, u32>,
    ) -> Self {
        Self {
            id,
            location,
            products,
            inventory,
        }
    }

    /// Checks if the store sells a specific product
    pub fn has_product(&self, product_id: &ProductId) -> bool {
        self.products.contains_key(product_id)
    }

    /// Gets the cost of a specific product if available
    pub fn get_product_cost(&self, product_id: &ProductId) -> Option<f64> {
        self.products.get(product_id).map(|p| p.cost)
    }

    /// Gets all product IDs available at this store
    pub fn get_available_product_ids(&self) -> Vec<ProductId> {
        self.products.keys().cloned().collect()
    }

    /// Checks if the store has sufficient quantity of a product
    pub fn has_sufficient_quantity(&self, product_id: &ProductId, quantity: u32) -> bool {
        if let Some(available) = self.inventory.get(product_id) {
            *available >= quantity
        } else {
            false
        }
    }

    /// Reduces the inventory of a product by the specified quantity
    /// Returns true if successful, false if insufficient inventory
    pub fn reduce_inventory(&mut self, product_id: &ProductId, quantity: u32) -> bool {
        if let Some(available) = self.inventory.get_mut(product_id) {
            if *available >= quantity {
                *available -= quantity;
                return true;
            }
        }
        false
    }

    /// Gets the current inventory level of a product
    pub fn get_inventory_level(&self, product_id: &ProductId) -> u32 {
        *self.inventory.get(product_id).unwrap_or(&0)
    }

    /// Restocks a product to the specified quantity
    pub fn restock(&mut self, product_id: &ProductId, quantity: u32) {
        if self.has_product(product_id) {
            *self.inventory.entry(*product_id).or_insert(0) += quantity;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_store() -> Store {
        let mut products = HashMap::new();
        products.insert(1, Product::new("Product A", 10.0));
        products.insert(2, Product::new("Product B", 20.0));

        let mut inventory = HashMap::new();
        inventory.insert(1, 5); // 5 units of Product A
        inventory.insert(2, 3); // 3 units of Product B

        Store::new_with_inventory(1, Location::new(0.0, 0.0), products, inventory)
    }

    #[test]
    fn test_has_product() {
        let store = create_test_store();
        assert!(store.has_product(&1));
        assert!(store.has_product(&2));
        assert!(!store.has_product(&3));
    }

    #[test]
    fn test_has_sufficient_quantity() {
        let store = create_test_store();
        assert!(store.has_sufficient_quantity(&1, 5));
        assert!(store.has_sufficient_quantity(&1, 3));
        assert!(!store.has_sufficient_quantity(&1, 6));
        assert!(!store.has_sufficient_quantity(&3, 1));
    }

    #[test]
    fn test_reduce_inventory() {
        let mut store = create_test_store();

        // Valid reduction
        assert!(store.reduce_inventory(&1, 2));
        assert_eq!(store.get_inventory_level(&1), 3);

        // Reduction to zero
        assert!(store.reduce_inventory(&1, 3));
        assert_eq!(store.get_inventory_level(&1), 0);

        // Invalid reduction (insufficient inventory)
        assert!(!store.reduce_inventory(&1, 1));
        assert_eq!(store.get_inventory_level(&1), 0);

        // Invalid product
        assert!(!store.reduce_inventory(&3, 1));
    }

    #[test]
    fn test_restock() {
        let mut store = create_test_store();

        // Reduce first
        store.reduce_inventory(&1, 3);
        assert_eq!(store.get_inventory_level(&1), 2);

        // Restock
        store.restock(&1, 5);
        assert_eq!(store.get_inventory_level(&1), 7);

        // Restock non-existent product (should not change anything)
        store.restock(&3, 10);
        assert_eq!(store.get_inventory_level(&3), 0);
    }
}
