// Product model representing items that can be purchased

use crate::models::Cost;

/// Represents a product that can be sold in a store
#[derive(Debug, Clone)]
pub struct Product {
    /// Name of the product
    pub name: String,

    /// Cost of the product
    pub cost: Cost,
}

impl Product {
    /// Creates a new product with the given name and cost
    pub fn new<S: Into<String>>(name: S, cost: Cost) -> Self {
        Self {
            name: name.into(),
            cost,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_product_creation() {
        let product = Product::new("Test Product", 10.5);
        assert_eq!(product.name, "Test Product");
        assert_eq!(product.cost, 10.5);
    }

    #[test]
    fn test_product_clone() {
        let product = Product::new("Test Product", 10.5);
        let cloned = product.clone();
        assert_eq!(cloned.name, product.name);
        assert_eq!(cloned.cost, product.cost);
    }
}
