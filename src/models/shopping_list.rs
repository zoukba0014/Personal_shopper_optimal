// Shopping list model representing customer requests with inventory awareness

use crate::models::{ProductId, Store, StoreId};
use std::collections::HashMap;

/// Represents a customer's shopping list with products and quantities
#[derive(Debug, Clone)]
pub struct ShoppingList {
    /// Map of product IDs to their required quantities
    pub items: HashMap<ProductId, u32>,

    /// Priority level of this shopping list (for multi-order processing)
    pub priority: u32,
}

impl ShoppingList {
    /// Creates a new empty shopping list with default priority
    pub fn new() -> Self {
        Self {
            items: HashMap::new(),
            priority: 0,
        }
    }

    /// Creates a new empty shopping list with specified priority
    pub fn new_with_priority(priority: u32) -> Self {
        Self {
            items: HashMap::new(),
            priority,
        }
    }

    /// Adds an item to the shopping list with specified quantity
    pub fn add_item(&mut self, product_id: ProductId, quantity: u32) {
        if quantity > 0 {
            *self.items.entry(product_id).or_insert(0) += quantity;
        }
    }

    /// Removes an item from the shopping list
    pub fn remove_item(&mut self, product_id: &ProductId) {
        self.items.remove(product_id);
    }

    /// Updates the quantity of an existing item
    pub fn update_quantity(&mut self, product_id: ProductId, quantity: u32) {
        if quantity > 0 {
            self.items.insert(product_id, quantity);
        } else {
            self.items.remove(&product_id);
        }
    }

    /// Gets the total number of unique products in the list
    pub fn unique_product_count(&self) -> usize {
        self.items.len()
    }

    /// Gets the total number of items (including quantities)
    pub fn total_item_count(&self) -> u32 {
        self.items.values().sum()
    }

    /// Sets the priority of this shopping list
    pub fn set_priority(&mut self, priority: u32) {
        self.priority = priority;
    }

    /// Gets the priority of this shopping list
    pub fn get_priority(&self) -> u32 {
        self.priority
    }

    /// Checks if this shopping list can be fulfilled by the given stores
    /// considering both product availability and inventory levels
    pub fn can_be_fulfilled_by(&self, stores: &HashMap<StoreId, Store>) -> bool {
        // Create a map to track the total available inventory across all stores
        let mut total_available: HashMap<ProductId, u32> = HashMap::new();

        // Collect available inventory from all stores
        for store in stores.values() {
            for product_id in self.items.keys() {
                if store.has_product(product_id) {
                    let inventory = store.get_inventory_level(product_id);
                    *total_available.entry(*product_id).or_insert(0) += inventory;
                }
            }
        }

        // Check if all products have sufficient total inventory
        for (product_id, required_qty) in &self.items {
            if let Some(available_qty) = total_available.get(product_id) {
                if available_qty < required_qty {
                    return false;
                }
            } else {
                return false;
            }
        }

        true
    }

    /// Returns a list of stores that have at least one product from the shopping list
    pub fn find_relevant_stores<'a>(&self, stores: &'a HashMap<StoreId, Store>) -> Vec<&'a Store> {
        stores
            .values()
            .filter(|store| {
                // Check if the store has any product from the shopping list in stock
                self.items.keys().any(|product_id| {
                    store.has_product(product_id) && store.get_inventory_level(product_id) > 0
                })
            })
            .collect()
    }

    /// Creates a filtered version of the shopping list containing only products
    /// that are actually available in the given stores with sufficient inventory
    pub fn create_fulfillable_list(&self, stores: &HashMap<StoreId, Store>) -> ShoppingList {
        let mut fulfillable = ShoppingList::new_with_priority(self.priority);

        // Compute total available inventory across all stores
        let mut total_available: HashMap<ProductId, u32> = HashMap::new();

        for store in stores.values() {
            for product_id in self.items.keys() {
                if store.has_product(product_id) {
                    let inventory = store.get_inventory_level(product_id);
                    *total_available.entry(*product_id).or_insert(0) += inventory;
                }
            }
        }

        // Add items that can be fulfilled (possibly with reduced quantity)
        for (product_id, required_qty) in &self.items {
            if let Some(available_qty) = total_available.get(product_id) {
                if *available_qty > 0 {
                    let quantity = required_qty.min(available_qty);
                    fulfillable.add_item(*product_id, *quantity);
                }
            }
        }

        fulfillable
    }
}

impl Default for ShoppingList {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::{Location, Product};

    fn create_test_stores() -> HashMap<StoreId, Store> {
        let mut stores = HashMap::new();

        // Store 1 with some inventory
        let mut products1 = HashMap::new();
        products1.insert(1, Product::new("Apple", 1.99));
        products1.insert(2, Product::new("Banana", 0.99));

        let mut inventory1 = HashMap::new();
        inventory1.insert(1, 5); // 5 units of Product 1
        inventory1.insert(2, 8); // 8 units of Product 2

        stores.insert(
            1,
            Store::new_with_inventory(1, Location::new(0.0, 0.0), products1, inventory1),
        );

        // Store 2 with different inventory
        let mut products2 = HashMap::new();
        products2.insert(2, Product::new("Banana", 1.29));
        products2.insert(3, Product::new("Orange", 2.49));

        let mut inventory2 = HashMap::new();
        inventory2.insert(2, 3); // 3 units of Product 2
        inventory2.insert(3, 6); // 6 units of Product 3

        stores.insert(
            2,
            Store::new_with_inventory(2, Location::new(10.0, 10.0), products2, inventory2),
        );

        stores
    }

    #[test]
    fn test_can_be_fulfilled_by() {
        let stores = create_test_stores();

        // List with products that can be fulfilled
        let mut list1 = ShoppingList::new();
        list1.add_item(1, 3); // 3 apples (5 available in store 1)
        list1.add_item(2, 10); // 10 bananas (8 + 3 = 11 available across stores)
        assert!(list1.can_be_fulfilled_by(&stores));

        // List with product that can't be fulfilled due to insufficient inventory
        let mut list2 = ShoppingList::new();
        list2.add_item(1, 10); // 10 apples (only 5 available)
        assert!(!list2.can_be_fulfilled_by(&stores));

        // List with unavailable product
        let mut list3 = ShoppingList::new();
        list3.add_item(4, 1); // Product 4 is not available in any store
        assert!(!list3.can_be_fulfilled_by(&stores));
    }

    #[test]
    fn test_find_relevant_stores() {
        let stores = create_test_stores();

        // List requiring products from both stores
        let mut list1 = ShoppingList::new();
        list1.add_item(1, 1);
        list1.add_item(3, 1);
        let relevant_stores = list1.find_relevant_stores(&stores);
        assert_eq!(relevant_stores.len(), 2);

        // List requiring products only from store 1
        let mut list2 = ShoppingList::new();
        list2.add_item(1, 1);
        let relevant_stores = list2.find_relevant_stores(&stores);
        assert_eq!(relevant_stores.len(), 1);
        assert_eq!(relevant_stores[0].id, 1);
    }

    #[test]
    fn test_create_fulfillable_list() {
        let stores = create_test_stores();

        // List with more quantity than available
        let mut list = ShoppingList::new();
        list.add_item(1, 10); // 10 apples (only 5 available)
        list.add_item(2, 5); // 5 bananas (11 available)
        list.add_item(4, 2); // 2 units of product 4 (not available)

        let fulfillable = list.create_fulfillable_list(&stores);

        assert_eq!(fulfillable.items.len(), 2);
        assert_eq!(fulfillable.items.get(&1), Some(&5)); // Reduced to available quantity
        assert_eq!(fulfillable.items.get(&2), Some(&5)); // Unchanged
        assert!(!fulfillable.items.contains_key(&4)); // Removed completely
    }
}
