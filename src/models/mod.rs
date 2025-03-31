// Models module - exports all model types

mod location;
mod product;
mod route;
mod shopping_list;
mod store;

// Re-export model types
pub use self::location::Location;
pub use self::product::Product;
pub use self::route::{RouteCandidate, ShoppingRoute};
pub use self::shopping_list::ShoppingList;
pub use self::store::Store;

// Common type aliases for improved code readability
pub type ProductId = u32;
pub type StoreId = u32;
pub type Cost = f64;
pub type Time = f64;
