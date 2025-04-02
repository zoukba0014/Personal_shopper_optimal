use std::cmp::Ordering;
use std::collections::{BinaryHeap, HashMap, HashSet};
use std::f64;
use std::sync::atomic::Ordering::Relaxed;
use std::sync::mpsc::{self};
use std::sync::RwLock;
use std::sync::{atomic::AtomicBool, Arc};
use std::thread;
use std::time::Duration;

use crate::algorithms::PSDSolver;
use crate::models::{
    Cost, Location, ProductId, RouteCandidate, ShoppingList, ShoppingRoute, Store, StoreId, Time,
};

// Custom wrapper to make f64 implement Eq
#[derive(PartialEq, Copy, Clone, Debug)]
struct F64Wrapper(f64);

impl Eq for F64Wrapper {}

impl PartialOrd for F64Wrapper {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        self.0.partial_cmp(&other.0)
    }
}

impl Ord for F64Wrapper {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.partial_cmp(other).unwrap_or(std::cmp::Ordering::Equal)
    }
}

// Custom ordered tuple for the priority queue
#[derive(PartialEq, Eq, Debug)]
struct QueueState {
    distance: F64Wrapper,
    store_id: StoreId,
    remaining_items: Vec<(ProductId, u32)>,
}

// Implement Ord for the custom state
impl Ord for QueueState {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        // Reverse order so we get a min-heap
        other.distance.cmp(&self.distance)
    }
}

impl PartialOrd for QueueState {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}
/// BSL-PSD solver for the Personal Shopper's Dilemma with inventory tracking
#[derive(Clone)]
pub struct BSLPSD {
    /// Map of stores by ID
    pub stores: HashMap<StoreId, Arc<RwLock<Store>>>,

    /// Precomputed travel times between stores
    travel_times: HashMap<(StoreId, StoreId), Time>,

    /// Inverted list of products to stores (sorted by cost)
    product_to_stores: HashMap<ProductId, Vec<(StoreId, Cost)>>,
}

impl BSLPSD {
    /// Creates a new BSL-PSD solver with the given stores
    pub fn new(stores: HashMap<StoreId, Store>) -> Self {
        // Convert to Arc<Mutex<Store>> for thread-safe interior mutability
        let arc_stores = stores
            .into_iter()
            .map(|(id, store)| (id, Arc::new(RwLock::new(store))))
            .collect();

        Self {
            stores: arc_stores,
            travel_times: HashMap::new(),
            product_to_stores: HashMap::new(),
        }
    }
    pub fn new_with_travel_times(
        stores: HashMap<StoreId, Store>,
        travel_times: HashMap<(StoreId, StoreId), f64>,
    ) -> Self {
        let arc_stores = stores
            .into_iter()
            .map(|(id, store)| (id, Arc::new(RwLock::new(store))))
            .collect();

        Self {
            stores: arc_stores,
            travel_times,
            product_to_stores: HashMap::new(),
        }
    }

    /// Precomputes necessary data structures
    pub fn precompute_data(&mut self) {
        self.build_inverted_list();
    }

    /// Builds the inverted list of products to stores
    fn build_inverted_list(&mut self) {
        // Clear existing data
        self.product_to_stores.clear();

        // Build the inverted list
        for (store_id, store_arc) in &self.stores {
            // Use lock() instead of borrow() for Arc<Mutex<T>>
            let store = store_arc.read().unwrap();
            for (product_id, product) in &store.products {
                // Only include products with available inventory
                if *store.inventory.get(product_id).unwrap_or(&0) > 0 {
                    self.product_to_stores
                        .entry(*product_id)
                        .or_insert_with(Vec::new)
                        .push((*store_id, product.cost));
                }
            }
        }

        // Sort each product's stores by cost (ascending)
        for stores in self.product_to_stores.values_mut() {
            stores.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(Ordering::Equal));
        }
    }

    /// Find the route with minimum shopping cost considering inventory
    pub fn find_min_cost_route(
        &self,
        shopping_list: &ShoppingList,
        shopper_location: Location,
        customer_location: Location,
        // ) -> Option<ShoppingRoute> {
    ) -> Option<f64> {
        // First verify if the shopping list can be fulfilled by all stores combined
        let mut total_available: HashMap<ProductId, u32> = HashMap::new();

        // Calculate available quantities across all stores
        for (product_id, qty_needed) in &shopping_list.items {
            let mut available = 0;
            if let Some(stores) = self.product_to_stores.get(product_id) {
                for &(store_id, _) in stores {
                    let store = self.stores[&store_id].read().unwrap();
                    available += store.get_inventory_level(product_id);
                }
            }
            total_available.insert(*product_id, available);

            // Check if enough quantity is available across all stores
            if available < *qty_needed {
                return None; // Cannot fulfill the shopping list
            }
        }

        // For each product, find the lowest cost stores
        let mut total_cost = 0.0;

        for (product_id, qty_needed) in &shopping_list.items {
            let mut remaining_qty = *qty_needed;
            let mut options = Vec::new();

            // Get all store options for this product
            if let Some(stores) = self.product_to_stores.get(product_id) {
                for &(store_id, _) in stores {
                    let store = self.stores[&store_id].read().unwrap();
                    let available_qty = store.get_inventory_level(product_id);

                    if available_qty > 0 {
                        let cost = store.get_product_cost(product_id).unwrap_or(f64::INFINITY);
                        options.push((store_id, cost, available_qty));
                    }
                }
            }

            // Sort options by cost (lowest first)
            options.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal));

            // Allocate products to lowest cost stores first
            for &(_, cost, available_qty) in &options {
                let purchase_qty = std::cmp::min(available_qty, remaining_qty);
                if purchase_qty > 0 {
                    total_cost += cost * purchase_qty as f64;
                    remaining_qty -= purchase_qty;
                }

                if remaining_qty == 0 {
                    break;
                }
            }
        }

        Some(total_cost)
    }
    /// Find the route with minimum shopping time using Dijkstra algorithm
    /// Allows purchasing products across multiple stores
    pub fn find_min_time_route_dijkstra(
        &self,
        shopping_list: &ShoppingList,
        shopper_location: Location,
        customer_location: Location,
    ) -> Option<ShoppingRoute> {
        // Create a distance mapping and a precursor node mapping
        let mut distances: HashMap<(StoreId, Vec<(ProductId, u32)>), f64> = HashMap::new();
        let mut predecessors: HashMap<
            (StoreId, Vec<(ProductId, u32)>),
            Option<(StoreId, Vec<(ProductId, u32)>)>,
        > = HashMap::new();

        // Create a priority queue
        let mut priority_queue = BinaryHeap::new();

        // Collect all items from the shopping list
        let shopping_list_items: Vec<(ProductId, u32)> =
            shopping_list.items.iter().map(|(k, v)| (*k, *v)).collect();

        // Find all stores that can satisfy at least one item from the shopping list
        let mut candidate_stores: HashSet<StoreId> = HashSet::new();
        for &(product_id, _) in &shopping_list_items {
            for (store_id, store_rc) in &self.stores {
                let store = store_rc.read().unwrap();
                if store.has_product(&product_id) {
                    candidate_stores.insert(*store_id);
                }
            }
        }

        // If no candidate stores found, return None as shopping list cannot be satisfied
        if candidate_stores.is_empty() {
            return None;
        }

        // First, verify if the shopping list can be fulfilled by all stores combined
        let mut can_be_fulfilled = true;
        {
            let mut total_available: HashMap<ProductId, u32> = HashMap::new();

            // Calculate available quantities across all stores
            for &(product_id, qty_needed) in &shopping_list_items {
                let mut available = 0;
                for &store_id in &candidate_stores {
                    let store = self.stores[&store_id].read().unwrap();
                    if store.has_product(&product_id) {
                        available += store.get_inventory_level(&product_id);
                    }
                }
                total_available.insert(product_id, available);

                // Check if enough quantity is available across all stores
                if available < qty_needed {
                    can_be_fulfilled = false;
                    break;
                }
            }

            if !can_be_fulfilled {
                return None; // Cannot fulfill the shopping list with all stores combined
            }
        }

        // Start from the shopper's location
        // Set initial distances for each potential starting store
        for &store_id in &candidate_stores {
            let store = self.stores[&store_id].read().unwrap();

            // Calculate distance from shopper's location to this store
            let distance = shopper_location.distance_to(&store.location);

            // Calculate which products and quantities this store can satisfy
            let mut remaining_quantities = shopping_list_items.clone();
            let mut any_product_purchased = false;

            // Use indexed iteration to avoid multiple mutable borrows
            for i in 0..remaining_quantities.len() {
                let (product_id, qty_needed) = &mut remaining_quantities[i];
                if store.has_product(product_id) {
                    let available_qty = store.get_inventory_level(product_id);
                    let purchase_qty = std::cmp::min(available_qty, *qty_needed);

                    if purchase_qty > 0 {
                        // Update the remaining quantity needed
                        *qty_needed -= purchase_qty;
                        any_product_purchased = true;
                    }
                }
            }

            if any_product_purchased {
                let sorted_remaining = sort_remaining_items(remaining_quantities.clone());
                distances.insert((store_id, sorted_remaining.clone()), distance);
                predecessors.insert((store_id, sorted_remaining.clone()), None);

                priority_queue.push(QueueState {
                    distance: F64Wrapper(distance),
                    store_id,
                    remaining_items: sorted_remaining,
                });
            }
        }

        // Track visited nodes to avoid processing them multiple times
        let mut visited: HashSet<(StoreId, Vec<(ProductId, u32)>)> = HashSet::new();

        // Variables to keep track of the best route found
        let mut best_time = f64::INFINITY;
        let mut best_state = None;

        // Main Dijkstra algorithm loop
        while let Some(QueueState {
            distance,
            store_id: current_store,
            remaining_items: current_remaining,
        }) = priority_queue.pop()
        {
            let current_dist = distance.0; // Unwrap the f64 value

            // Skip if this node has already been visited
            if visited.contains(&(current_store, current_remaining.clone())) {
                continue;
            }

            // Mark as visited
            visited.insert((current_store, current_remaining.clone()));

            // Check if all products have been purchased (all remaining quantities are 0)
            let all_purchased = current_remaining.iter().all(|(_, qty)| *qty == 0);

            if all_purchased {
                // Calculate the final distance to the customer location
                let store = self.stores[&current_store].read().unwrap();
                let final_distance = current_dist + store.location.distance_to(&customer_location);

                // Update the best result if this route is faster
                if final_distance < best_time {
                    best_time = final_distance;
                    best_state = Some((current_store, current_remaining.clone()));
                }

                // Continue searching for potentially better routes
                continue;
            }

            // Try the next store
            for &next_store in &candidate_stores {
                // Skip the current store (we've already purchased what we can there)
                if next_store == current_store {
                    continue;
                }

                // Get the travel time from current store to next store
                let edge_weight = self
                    .travel_times
                    .get(&(current_store, next_store))
                    .cloned()
                    .unwrap_or(f64::INFINITY);

                if edge_weight.is_infinite() {
                    continue;
                }

                // Calculate which products can be purchased at the next store
                let next_store_ref = self.stores[&next_store].read().unwrap();
                let mut new_remaining = current_remaining.clone();
                let mut any_new_purchases = false;

                // Use indexed iteration to avoid multiple mutable borrows
                for i in 0..new_remaining.len() {
                    let (product_id, qty_needed) = &mut new_remaining[i];
                    if *qty_needed > 0 && next_store_ref.has_product(product_id) {
                        let available_qty = next_store_ref.get_inventory_level(product_id);
                        let purchase_qty = std::cmp::min(available_qty, *qty_needed);

                        if purchase_qty > 0 {
                            // Update the remaining quantity needed
                            *qty_needed -= purchase_qty;
                            any_new_purchases = true;
                        }
                    }
                }

                // Skip if no new products can be purchased
                if !any_new_purchases {
                    continue;
                }

                // Sort the remaining items for consistent state comparison
                let sorted_new_remaining = sort_remaining_items(new_remaining);

                // Calculate the new distance
                let next_dist = current_dist + edge_weight;

                // Relaxation operation - update if this path is shorter
                if !distances.contains_key(&(next_store, sorted_new_remaining.clone()))
                    || next_dist < distances[&(next_store, sorted_new_remaining.clone())]
                {
                    distances.insert((next_store, sorted_new_remaining.clone()), next_dist);
                    predecessors.insert(
                        (next_store, sorted_new_remaining.clone()),
                        Some((current_store, current_remaining.clone())),
                    );
                    priority_queue.push(QueueState {
                        distance: F64Wrapper(next_dist),
                        store_id: next_store,
                        remaining_items: sorted_new_remaining,
                    });
                }
            }
        }

        // If no valid route was found, return None
        if best_state.is_none() {
            return None;
        }

        // Reconstruct the path by backtracking
        let mut current_state = best_state.unwrap();
        let mut path = Vec::new();

        while let Some(prev_state) = predecessors[&current_state].clone() {
            path.push(current_state.0);
            current_state = prev_state;
        }

        path.push(current_state.0);
        path.reverse();

        // Calculate the shopping cost for the path - this will optimize purchase decisions
        let shopping_cost = self.calculate_shopping_cost(&path, shopping_list);

        Some(ShoppingRoute {
            stores: path,
            shopping_time: best_time,
            shopping_cost,
        })
    }

    /// Check if the route can fulfill the shopping list
    fn can_fulfill_shopping_list(&self, route: &[StoreId], shopping_list: &ShoppingList) -> bool {
        let mut remaining_quantities = shopping_list.items.clone();

        for &store_id in route {
            let store = self.stores[&store_id].read().unwrap();

            for (product_id, remaining_qty) in remaining_quantities.iter_mut() {
                if *remaining_qty > 0 && store.has_product(product_id) {
                    let available_qty = store.get_inventory_level(product_id);
                    let purchase_qty = std::cmp::min(available_qty, *remaining_qty);
                    *remaining_qty -= purchase_qty;
                }
            }
        }

        // Check if all items are fulfilled
        remaining_quantities.values().all(|&qty| qty == 0)
    }

    /// Checks if a route satisfies a shopping list considering inventory
    pub fn satisfies_list_with_inventory(
        &self,
        route: &[StoreId],
        shopping_list: &ShoppingList,
    ) -> bool {
        // Create a map to track the total available inventory in the route's stores
        let mut available_inventory: HashMap<ProductId, u32> = HashMap::new();

        // Collect available inventory from stores in the route
        for &store_id in route {
            let store = self.stores[&store_id].read().unwrap();
            for (product_id, _) in &shopping_list.items {
                if store.has_product(product_id) {
                    let inventory = store.get_inventory_level(product_id);
                    *available_inventory.entry(*product_id).or_insert(0) += inventory;
                }
            }
        }

        // Check if all products have sufficient inventory
        for (product_id, required_qty) in &shopping_list.items {
            if let Some(available_qty) = available_inventory.get(product_id) {
                if available_qty < required_qty {
                    return false;
                }
            } else {
                return false;
            }
        }

        true
    }

    /// Find the minimum detour store to add to a route
    /// This corresponds to the MinDetour function in the paper
    fn find_min_detour_store(
        &self,
        from_store: Option<StoreId>,
        visited_stores: &HashSet<StoreId>,
    ) -> Option<StoreId> {
        let mut min_detour = f64::INFINITY;
        let mut min_detour_store = None;

        for (store_id, _) in &self.stores {
            // Skip if already visited
            if visited_stores.contains(store_id) {
                continue;
            }

            // Calculate detour
            let detour = match from_store {
                Some(from) => self
                    .travel_times
                    .get(&(from, *store_id))
                    .cloned()
                    .unwrap_or(f64::INFINITY),
                None => 0.0, // First store in route
            };

            if detour < min_detour {
                min_detour = detour;
                min_detour_store = Some(*store_id);
            }
        }

        min_detour_store
    }

    /// Find the next minimum detour store after the kth one
    /// This corresponds to the NextMinDetour function in the paper
    fn find_next_min_detour_store(
        &self,
        from_store: Option<StoreId>,
        visited_stores: &HashSet<StoreId>,
        current_min_detour_store: StoreId,
    ) -> Option<StoreId> {
        let mut next_min_detour = f64::INFINITY;
        let mut next_min_detour_store = None;

        // Get the detour of the current minimum
        let current_detour = match from_store {
            Some(from) => self
                .travel_times
                .get(&(from, current_min_detour_store))
                .cloned()
                .unwrap_or(f64::INFINITY),
            None => 0.0, // First store in route
        };

        for (store_id, _) in &self.stores {
            // Skip if already visited or if it's the current minimum
            if visited_stores.contains(store_id) || *store_id == current_min_detour_store {
                continue;
            }

            // Calculate detour
            let detour = match from_store {
                Some(from) => self
                    .travel_times
                    .get(&(from, *store_id))
                    .cloned()
                    .unwrap_or(f64::INFINITY),
                None => 0.0, // First store in route
            };

            // Find the next minimum (greater than current, but less than any other)
            if detour > current_detour && detour < next_min_detour {
                next_min_detour = detour;
                next_min_detour_store = Some(*store_id);
            }
        }

        next_min_detour_store
    }
    /// Generate next routes according to the generation scheme in the paper
    fn generate_next_routes(&self, route: &RouteCandidate) -> Vec<RouteCandidate> {
        let mut next_routes = Vec::new();
        let mut visited_stores: HashSet<StoreId> = route.stores.iter().cloned().collect();

        // Case 1: Add a new store at the end (θs in the paper)
        if let Some(last_store) = route.stores.last().cloned() {
            if let Some(min_detour_store) =
                self.find_min_detour_store(Some(last_store), &visited_stores)
            {
                let mut new_route = route.stores.clone();
                new_route.push(min_detour_store);

                // Calculate new shopping time
                let detour = self
                    .travel_times
                    .get(&(last_store, min_detour_store))
                    .cloned()
                    .unwrap_or(f64::INFINITY);

                let new_time = route.shopping_time + detour;

                next_routes.push(RouteCandidate {
                    stores: new_route,
                    shopping_time: new_time,
                });
            }
        } else {
            // First store in the route
            if let Some(min_detour_store) = self.find_min_detour_store(None, &visited_stores) {
                let new_route = vec![min_detour_store];

                // For initial route, calculate from shopper location (handled in solve() method)
                next_routes.push(RouteCandidate {
                    stores: new_route,
                    shopping_time: 0.0,
                });
            }
        }

        // Case 2: Replace the last store (θp in the paper)
        if route.stores.len() >= 1 {
            let last_store = route.stores.last().unwrap();

            // Remove last store from visited set for consideration of replacements
            visited_stores.remove(last_store);

            if let Some(second_last_store) = route
                .stores
                .get(route.stores.len().saturating_sub(2))
                .cloned()
            {
                if let Some(next_min_detour_store) = self.find_next_min_detour_store(
                    Some(second_last_store),
                    &visited_stores,
                    *last_store,
                ) {
                    let mut new_route = route.stores.clone();
                    new_route.pop(); // Remove last store
                    new_route.push(next_min_detour_store);

                    // Calculate new shopping time by removing last detour and adding new one
                    let old_detour = self
                        .travel_times
                        .get(&(second_last_store, *last_store))
                        .cloned()
                        .unwrap_or(0.0);

                    let new_detour = self
                        .travel_times
                        .get(&(second_last_store, next_min_detour_store))
                        .cloned()
                        .unwrap_or(f64::INFINITY);

                    let new_time = route.shopping_time - old_detour + new_detour;

                    next_routes.push(RouteCandidate {
                        stores: new_route,
                        shopping_time: new_time,
                    });
                }
            }
        }

        next_routes
    }
    /// Generate next routes according to the original strategy but with path optimization
    /// Find the shortest path that visits all stores in the given set
    /// Ensures no duplicate stores in the result
    fn find_shortest_path(
        &self,
        stores: &Vec<StoreId>,
        shopper_location: &Location,
        customer_location: &Location,
    ) -> Vec<StoreId> {
        if stores.is_empty() {
            return Vec::new();
        }

        if stores.len() == 1 {
            return stores.clone();
        }

        // Remove any duplicates from the store set first
        let mut unique_stores = Vec::new();
        let mut seen = HashSet::new();

        for &store_id in stores {
            if !seen.contains(&store_id) {
                unique_stores.push(store_id);
                seen.insert(store_id);
            }
        }

        // For small sets of stores, we can try all permutations
        let permutations = self.generate_permutations(unique_stores);
        let mut best_path = Vec::new();
        let mut min_time = f64::INFINITY;

        for perm in permutations {
            let time = self.calculate_total_time(&perm, shopper_location, customer_location);

            if time < min_time {
                min_time = time;
                best_path = perm;
            }
        }

        best_path
    }
    /// Generate all permutations of a vector of StoreId
    fn generate_permutations(&self, stores: Vec<StoreId>) -> Vec<Vec<StoreId>> {
        if stores.is_empty() {
            return vec![vec![]];
        }

        let mut result = Vec::new();

        for (i, &store) in stores.iter().enumerate() {
            let mut remaining = stores.clone();
            remaining.remove(i);

            for mut perm in self.generate_permutations(remaining) {
                perm.insert(0, store);
                result.push(perm);
            }
        }

        result
    }
    /// Calculate the total time of a path from shopper to stores to customer
    fn calculate_total_time(
        &self,
        path: &Vec<StoreId>,
        shopper_location: &Location,
        customer_location: &Location,
    ) -> f64 {
        if path.is_empty() {
            // Direct path from shopper to customer
            return shopper_location.distance_to(customer_location);
        }

        let mut total_time = 0.0;

        // Time from shopper to first store
        let first_store = self.stores[&path[0]].read().unwrap();
        total_time += shopper_location.distance_to(&first_store.location);

        // Time between consecutive stores
        for i in 0..path.len() - 1 {
            if let Some(&time) = self.travel_times.get(&(path[i], path[i + 1])) {
                total_time += time;
            } else {
                // If we don't have travel time data, use distance between locations
                let from_store = self.stores[&path[i]].read().unwrap();
                let to_store = self.stores[&path[i + 1]].read().unwrap();
                total_time += from_store.location.distance_to(&to_store.location);
            }
        }

        // Time from last store to customer
        let last_store = self.stores[&path[path.len() - 1]].read().unwrap();
        total_time += last_store.location.distance_to(customer_location);

        total_time
    }

    /// Generate next routes according to the original strategy but with path optimization
    /// Ensures no duplicate stores in routes
    fn generate_next_routes_shuffle(
        &self,
        route: &RouteCandidate,
        shopper_location: &Location,
        customer_location: &Location,
    ) -> Vec<RouteCandidate> {
        let mut next_routes = Vec::new();
        let visited_stores: HashSet<StoreId> = route.stores.iter().cloned().collect();

        // First, check if the current route has duplicates and fix if needed
        let current_route = route.stores.clone();
        let mut has_duplicates = false;
        let mut unique_current = Vec::new();
        let mut seen = HashSet::new();

        for &store_id in &current_route {
            if !seen.contains(&store_id) {
                unique_current.push(store_id);
                seen.insert(store_id);
            } else {
                has_duplicates = true;
            }
        }

        // If the input route had duplicates, optimize it first
        if has_duplicates {
            let optimized_current =
                self.find_shortest_path(&unique_current, shopper_location, customer_location);

            let shopping_time =
                self.calculate_total_time(&optimized_current, shopper_location, customer_location);

            if !shopping_time.is_infinite() {
                next_routes.push(RouteCandidate {
                    stores: optimized_current,
                    shopping_time,
                });
            }

            // Return early - we need to fix the current route before generating more
            return next_routes;
        }

        // Case 1: Add a new store at the end (θs in the paper)
        if !route.stores.is_empty() {
            let last_store = route.stores.last().unwrap();

            if let Some(min_detour_store) =
                self.find_min_detour_store(Some(*last_store), &visited_stores)
            {
                // Make sure we're not adding a duplicate
                if !visited_stores.contains(&min_detour_store) {
                    // Generate a new route with the additional store
                    let mut new_route = route.stores.clone();
                    new_route.push(min_detour_store);

                    // Now optimize the entire route order to find the shortest path
                    let optimized_route =
                        self.find_shortest_path(&new_route, shopper_location, customer_location);

                    // Calculate shopping time for the optimized route
                    let shopping_time = self.calculate_total_time(
                        &optimized_route,
                        shopper_location,
                        customer_location,
                    );

                    if !shopping_time.is_infinite() {
                        next_routes.push(RouteCandidate {
                            stores: optimized_route,
                            shopping_time,
                        });
                    }
                }
            }
        } else {
            // First store in the route
            if let Some(min_detour_store) = self.find_min_detour_store(None, &visited_stores) {
                let new_route = vec![min_detour_store];

                // Calculate shopping time from shopper location to this store to customer location
                let shopping_time =
                    self.calculate_total_time(&new_route, shopper_location, customer_location);

                if !shopping_time.is_infinite() {
                    next_routes.push(RouteCandidate {
                        stores: new_route,
                        shopping_time,
                    });
                }
            }
        }

        // Case 2: Replace the last store (θp in the paper)
        if route.stores.len() >= 1 {
            let last_store = route.stores.last().unwrap();

            // Create a modified visited set without the last store
            let mut visited_without_last = visited_stores.clone();
            visited_without_last.remove(last_store);

            if let Some(second_last_store) = route
                .stores
                .get(route.stores.len().saturating_sub(2))
                .cloned()
            {
                if let Some(next_min_detour_store) = self.find_next_min_detour_store(
                    Some(second_last_store),
                    &visited_without_last,
                    *last_store,
                ) {
                    // Make sure the replacement isn't already in the route
                    if !visited_without_last.contains(&next_min_detour_store) {
                        // Create a new route with the replacement
                        let mut new_route = route.stores.clone();
                        new_route.pop(); // Remove last store
                        new_route.push(next_min_detour_store);

                        // Now optimize the entire route order
                        let optimized_route = self.find_shortest_path(
                            &new_route,
                            shopper_location,
                            customer_location,
                        );

                        // Calculate shopping time for the optimized route
                        let shopping_time = self.calculate_total_time(
                            &optimized_route,
                            shopper_location,
                            customer_location,
                        );

                        if !shopping_time.is_infinite() {
                            next_routes.push(RouteCandidate {
                                stores: optimized_route,
                                shopping_time,
                            });
                        }
                    }
                }
            }
        }

        next_routes
    }

    /// Create a snapshot of the current inventory state
    pub fn snapshot_inventory(&self) -> HashMap<StoreId, HashMap<ProductId, u32>> {
        let mut snapshot = HashMap::new();

        for (&store_id, store_arc) in &self.stores {
            let store = store_arc.read().unwrap();
            snapshot.insert(store_id, store.inventory.clone());
        }

        snapshot
    }

    /// Update the skyline with a new route
    pub fn update_skyline(&self, skyline: &mut Vec<ShoppingRoute>, route: ShoppingRoute) -> bool {
        // Check if the route is dominated by any route in the skyline
        for existing_route in skyline.iter() {
            if existing_route.conventionally_dominates(&route) || existing_route == &route {
                return false;
            }
        }

        // Remove routes that are dominated by the new route
        skyline.retain(|existing_route| !route.conventionally_dominates(existing_route));

        // Add the new route
        skyline.push(route);

        true
    }
    pub fn solve_with_parallel(
        &self,
        shopping_list: &ShoppingList,
        shopper_location: Location,
        customer_location: Location,
        threshold: i32,
    ) -> (Vec<ShoppingRoute>, Duration) {
        println!("Starting parallel BSL-PSD algorithm with channels...");
        let start_time_find_best_route = std::time::Instant::now();
        // Step 1: Find route with minimum shopping cost
        // let min_cost_route =
        let min_cost =
            match self.find_min_cost_route(shopping_list, shopper_location, customer_location) {
                Some(route) => route,
                None => {
                    println!("No minimum cost route found, aborting.");
                    return (Vec::new(), Duration::default());
                }
            };
        println!("Found_min_cost: {:?}", min_cost);

        // Find route with minimum time cost
        let min_time_route = match self.find_min_time_route_dijkstra(
            shopping_list,
            shopper_location,
            customer_location,
        ) {
            Some(route) => route,
            None => {
                println!("No minimum time route found, aborting.");
                return (Vec::new(), Duration::default());
            }
        };
        let elapsed_limited = start_time_find_best_route.elapsed();
        println!("Found_min_time: {:?}", min_time_route);

        // let sc_upper_bound = min_cost_route.shopping_cost;
        let sc_upper_bound = min_cost;
        // println!(
        //     "Found min cost route with cost: ${:.2}, sc_upper_bound: ${:.2}",
        //     min_cost_route.shopping_cost, sc_upper_bound
        // );

        // Create termination signal
        let found_upper_bound = Arc::new(AtomicBool::new(false));

        // Create communication channel for sending found skyline routes
        let (tx1, rx) = mpsc::channel();
        let tx2 = tx1.clone();

        // Clone data needed for use between threads
        let min_time_route_clone = min_time_route.clone();
        let shopping_list_clone = shopping_list.clone();
        let shopper_location_clone = shopper_location.clone();
        let customer_location_clone = customer_location.clone();
        let self_clone = self.clone(); // Need to implement Clone trait
        let found_upper_bound_clone = Arc::clone(&found_upper_bound);

        // Start normal route generation thread
        let _handle1 = thread::spawn(move || {
            let mut visited_route = HashSet::new();
            println!("Start thread 1");
            let mut queue = BinaryHeap::new();
            // Initialize with min time route
            queue.push(RouteCandidate {
                stores: min_time_route_clone.stores.clone(),
                shopping_time: min_time_route_clone.shopping_time,
            });

            // // Mark initial route as visited
            // {
            //     let mut visited = visited_routes1.lock().unwrap();
            //     visited.insert(min_time_route_clone.stores.clone());
            // }

            while let Some(route_candidate) = queue.pop() {
                // Check if upper bound route has been found
                if found_upper_bound_clone.load(Relaxed) {
                    break;
                }

                // Check if route satisfies shopping list
                let satisfies = self_clone
                    .satisfies_list_with_inventory(&route_candidate.stores, &shopping_list_clone);

                if satisfies {
                    // Calculate shopping cost
                    let shopping_cost = self_clone
                        .calculate_shopping_cost(&route_candidate.stores, &shopping_list_clone);

                    // Create complete shopping route
                    let shopping_route = ShoppingRoute {
                        stores: route_candidate.stores.clone(),
                        shopping_time: route_candidate.shopping_time,
                        shopping_cost,
                    };

                    // Send found route
                    tx1.send(shopping_route.clone()).unwrap();

                    // println!(
                    //     "Thread 1 found route: {:?}, cost: ${:.2}, time: {:.2}",
                    //     route_candidate.stores, shopping_cost, route_candidate.shopping_time
                    // );

                    // Check if upper bound has been reached
                    if shopping_cost == sc_upper_bound {
                        println!("Thread 1 found the sc_upper_bound skyline route!");
                        found_upper_bound_clone.store(true, Relaxed);
                        break;
                    }
                }

                // Generate next batch of routes
                let next_routes = self_clone.generate_next_routes(&route_candidate);

                // Filter already visited routes and add to queue
                for next_route in next_routes {
                    let is_new_route;
                    {
                        // let visited = visited_routes1.read().unwrap();
                        is_new_route = !visited_route.contains(&next_route.stores);
                        if is_new_route {
                            // visited.insert(next_route.stores.clone());
                            // let mut visited = visited_routes1.write().unwrap();
                            visited_route.insert(next_route.stores.clone());
                        }
                    }
                    if is_new_route {
                        queue.push(next_route);
                    }
                }
            }
        });

        // Clone data needed for second thread
        let min_time_route_clone2 = min_time_route.clone();
        let shopping_list_clone2 = shopping_list.clone();
        let self_clone2 = self.clone();
        let found_upper_bound_clone2 = Arc::clone(&found_upper_bound);

        // Start shuffle route generation thread
        let _handle2 = thread::spawn(move || {
            let mut visited_route = HashSet::new();
            println!("Start thread 2");
            let mut queue = BinaryHeap::new();
            queue.push(RouteCandidate {
                stores: min_time_route_clone2.stores.clone(),
                shopping_time: min_time_route_clone2.shopping_time,
            });

            // No need to mark initial route as visited again since thread1 did it

            while let Some(route_candidate) = queue.pop() {
                // Check if upper bound route has been found
                if found_upper_bound_clone2.load(Relaxed) {
                    break;
                }

                // Check if route satisfies shopping list
                let satisfies = self_clone2
                    .satisfies_list_with_inventory(&route_candidate.stores, &shopping_list_clone2);

                if satisfies {
                    // Calculate shopping cost
                    let shopping_cost = self_clone2
                        .calculate_shopping_cost(&route_candidate.stores, &shopping_list_clone2);

                    // Create complete shopping route
                    let shopping_route = ShoppingRoute {
                        stores: route_candidate.stores.clone(),
                        shopping_time: route_candidate.shopping_time,
                        shopping_cost,
                    };

                    // Send found route
                    tx2.send(shopping_route.clone()).unwrap();

                    // println!(
                    //     "Thread 2 found route: {:?}, cost: ${:.2}, time: {:.2}",
                    //     route_candidate.stores, shopping_cost, route_candidate.shopping_time
                    // );

                    // Check if upper bound has been reached
                    if shopping_cost == sc_upper_bound {
                        println!("Thread 2 found the sc_upper_bound skyline route!");
                        found_upper_bound_clone2.store(true, Relaxed);
                        break;
                    }
                }

                // Generate next batch of routes (using shuffle version)
                let next_routes = self_clone2.generate_next_routes_shuffle(
                    &route_candidate,
                    &shopper_location_clone,
                    &customer_location_clone,
                );

                // Filter already visited routes and add to queue
                for next_route in next_routes {
                    let is_new_route;
                    {
                        // let visited = visited_routes2.read().unwrap();
                        is_new_route = !visited_route.contains(&next_route.stores);
                        if is_new_route {
                            // visited.insert(next_route.stores.clone());

                            visited_route.insert(next_route.stores.clone());
                        }
                    }
                    if is_new_route {
                        queue.push(next_route);
                    }
                }
            }
        });

        // Main thread processes received skyline routes
        let mut linear_skyline = Vec::new();
        // let mut last_size = 0;
        let mut unchanged_count = 0;
        // let max_unchanged = 10000; // Set a threshold for how many unchanged iterations before exiting
        // Process all received routes
        while let Ok(route) = rx.recv() {
            let old_size = linear_skyline.len();
            let update = self.update_skyline(&mut linear_skyline, route);
            // self.update_skyline(&mut linear_skyline, route);
            // println!("routes: {}", linear_skyline.len());
            if linear_skyline.len() == old_size && !update {
                unchanged_count += 1;
                // println!("Skyline unchanged for {} iterations", unchanged_count);

                // If unchanged for multiple iterations, consider exhaustive search complete
                if unchanged_count >= threshold {
                    println!(
                        "Skyline size remained at {} for {} iterations, breaking",
                        linear_skyline.len(),
                        threshold
                    );
                    found_upper_bound.store(true, Relaxed);
                    // break;
                }
            } else {
                unchanged_count = 0
            }
            // if linear_skyline.len() >= 15 {
            //     println!("Already got 15 routes in optimal route set, start break");
            //     found_upper_bound.store(true, Relaxed);
            // }
        }

        // Sort skyline
        linear_skyline.sort_by(|a, b| {
            a.shopping_time
                .partial_cmp(&b.shopping_time)
                .unwrap_or(Ordering::Equal)
        });

        println!("Final skyline size: {}", linear_skyline.len());
        (linear_skyline, elapsed_limited)
    }

    /// Debug version of solve function with logging and timeout
    pub fn solve_with_debug(
        &self,
        shopping_list: &ShoppingList,
        shopper_location: Location,
        customer_location: Location,
        threshold: i32,
    ) -> Vec<ShoppingRoute> {
        println!("Starting BSL-PSD algorithm with debug mode...");

        // Step 1: Find route with minimum shopping cost
        let min_cost_route =
            match self.find_min_cost_route(shopping_list, shopper_location, customer_location) {
                Some(route) => route,
                None => {
                    println!("No minimum cost route found, aborting.");
                    return Vec::new(); // No solution available
                }
            };
        println!("min cost route: {:?}", min_cost_route);

        // Find route with minimum time cost
        let min_time_route = match self.find_min_time_route_dijkstra(
            shopping_list,
            shopper_location,
            customer_location,
        ) {
            Some(route) => route,
            None => {
                println!("No minimum time route found, aborting.");
                return Vec::new();
            }
        };
        println!("min time route: {:?}", min_time_route);

        // println!(
        //     "Found min cost route: {:?} with cost: {}, sc_upper_bound: {}",
        //     min_time_route.stores, min_time_route.shopping_cost, min_cost_route.shopping_cost
        // );

        // let sc_upper_bound = min_cost_route.shopping_cost;
        let sc_upper_bound = min_cost_route;

        // Step 2: Initialize priority queue and linear skyline
        let mut queue = BinaryHeap::new();
        let mut linear_skyline = Vec::new();

        let mut visited_routes = HashSet::new();

        queue.push(RouteCandidate {
            stores: min_time_route.stores.clone(),
            shopping_time: min_time_route.shopping_time,
        });

        println!("Initial queue size: {}", queue.len());
        // let mut last_size = 0;
        let mut unchanged_count = 0;
        // let max_unchanged = 10000;

        while let Some(route_candidate) = queue.pop() {
            // Only consider the route if it satisfies the shopping list
            let satisfies =
                self.satisfies_list_with_inventory(&route_candidate.stores, shopping_list);

            if satisfies {
                // Calculate actual shopping cost now that we know the route satisfies the list
                let shopping_cost =
                    self.calculate_shopping_cost(&route_candidate.stores, shopping_list);

                // Create a complete ShoppingRoute with shopping cost
                let shopping_route = ShoppingRoute {
                    stores: route_candidate.stores.clone(),
                    shopping_time: route_candidate.shopping_time,
                    shopping_cost,
                };
                // println!("shopping route: {:?}", shopping_route);

                // Update linear skyline with the new route
                let old_size = linear_skyline.len();
                let update = self.update_skyline(&mut linear_skyline, shopping_route);
                if linear_skyline.len() == old_size && !update {
                    unchanged_count += 1;
                    // println!("Skyline unchanged for {} iterations", unchanged_count);

                    // If unchanged for multiple iterations, consider exhaustive search complete
                    if unchanged_count >= threshold {
                        println!(
                            "Skyline size remained at {} for {} iterations, breaking",
                            linear_skyline.len(),
                            threshold
                        );
                        break;
                    }
                } else {
                    unchanged_count = 0
                }
                // if self.update_skyline(&mut linear_skyline, shopping_route) {
                //     println!(
                //         "Found skyline route: {:?}, cost: ${:.2}, time: {:.2}",
                //         route_candidate.stores, shopping_cost, route_candidate.shopping_time
                //     );
                // }
                if shopping_cost == sc_upper_bound {
                    println!("Found the sc_upper_bound skyline routes! Exist");
                    break;
                }
            }
            // Always generate next routes and add to queue
            let mut next_routes = self.generate_next_routes(&route_candidate);
            // let mut next_routes = self.generate_next_routes(
            //     &route_candidate,
            //     &shopper_location,
            //     &customer_location,
            // );

            next_routes.retain(|route| {
                let is_new_route = !visited_routes.contains(&route.stores);
                if is_new_route {
                    visited_routes.insert(route.stores.clone());
                }
                is_new_route
            });
            for next_route in next_routes {
                // println!("next route: {:?}", next_route);
                queue.push(next_route);
            }
        }

        println!("Final skyline size: {}", linear_skyline.len());

        // Sort skyline by shopping time (ascending)
        linear_skyline.sort_by(|a, b| {
            a.shopping_time
                .partial_cmp(&b.shopping_time)
                .unwrap_or(Ordering::Equal)
        });

        linear_skyline
    }

    /// Verify that travel times are correctly precomputed for all store pairs
    pub fn verify_travel_times(&self) -> bool {
        let store_ids: Vec<StoreId> = self.stores.keys().cloned().collect();
        let mut missing_pairs = Vec::new();
        let mut infinity_pairs = Vec::new();

        for i in 0..store_ids.len() {
            for j in 0..store_ids.len() {
                if i != j {
                    let from = store_ids[i];
                    let to = store_ids[j];

                    match self.travel_times.get(&(from, to)) {
                        None => {
                            missing_pairs.push((from, to));
                        }
                        Some(&time) => {
                            if time.is_infinite() || time.is_nan() {
                                infinity_pairs.push((from, to, time));
                            }
                        }
                    }
                }
            }
        }

        if !missing_pairs.is_empty() {
            println!(
                "WARNING: Missing travel times for {} store pairs:",
                missing_pairs.len()
            );
            for (from, to) in missing_pairs.iter().take(10) {
                println!("  - Travel time missing: {} -> {}", from, to);
            }
            if missing_pairs.len() > 10 {
                println!("  - ... and {} more", missing_pairs.len() - 10);
            }
        }

        if !infinity_pairs.is_empty() {
            println!(
                "WARNING: Infinite/NaN travel times for {} store pairs:",
                infinity_pairs.len()
            );
            for (from, to, time) in infinity_pairs.iter().take(10) {
                println!("  - Problematic travel time: {} -> {} = {}", from, to, time);
            }
            if infinity_pairs.len() > 10 {
                println!("  - ... and {} more", infinity_pairs.len() - 10);
            }
        }

        missing_pairs.is_empty() && infinity_pairs.is_empty()
    }
}
// Helper function to sort remaining items for consistent state representation
fn sort_remaining_items(items: Vec<(ProductId, u32)>) -> Vec<(ProductId, u32)> {
    let mut sorted_items = items;
    sorted_items.sort_by_key(|(product_id, _)| *product_id);
    sorted_items
}

// Implementation of the PSDSolver trait for BSLPSD
impl PSDSolver for BSLPSD {
    /// Main BSL-PSD algorithm to solve the Personal Shopper's Dilemma
    /// with inventory constraints
    fn solve(
        &self,
        shopping_list: &ShoppingList,
        shopper_location: Location,
        customer_location: Location,
    ) -> Vec<ShoppingRoute> {
        // Use a version with safety checks, limiting max iterations to 10000
        self.solve_with_debug(shopping_list, shopper_location, customer_location, 10000)
    }

    /// Checks if a route satisfies a shopping list
    fn satisfies_list(&self, route: &[StoreId], shopping_list: &ShoppingList) -> bool {
        self.satisfies_list_with_inventory(route, shopping_list)
    }

    /// Calculate shopping time for a route
    fn calculate_shopping_time(
        &self,
        route: &[StoreId],
        shopper_location: Location,
        customer_location: Location,
    ) -> f64 {
        if route.is_empty() {
            return 0.0;
        }

        let mut total_time = 0.0;

        // Time from shopper to first store
        if let Some(first_store_id) = route.first() {
            let first_store = self.stores[first_store_id].read().unwrap();
            total_time += shopper_location.distance_to(&first_store.location);
        }

        // Time between stores
        for i in 0..route.len() - 1 {
            let time = self
                .travel_times
                .get(&(route[i], route[i + 1]))
                .cloned()
                .unwrap_or(f64::INFINITY);
            total_time += time;
        }

        // Time from last store to customer
        if let Some(last_store_id) = route.last() {
            let last_store = self.stores[last_store_id].read().unwrap();
            total_time += last_store.location.distance_to(&customer_location);
        }

        total_time
    }

    /// Calculate shopping cost for a route
    fn calculate_shopping_cost(&self, route: &[StoreId], shopping_list: &ShoppingList) -> f64 {
        // First, verify the route can satisfy the shopping list
        let can_fulfill = self.can_fulfill_shopping_list(route, shopping_list);
        if !can_fulfill {
            return f64::INFINITY;
        }

        // For each product, record available stores, prices, and quantities
        let mut product_options: HashMap<ProductId, Vec<(StoreId, f64, u32)>> = HashMap::new();

        // Gather all options for each product from stores in the route
        for &store_id in route {
            let store = self.stores[&store_id].read().unwrap();

            for (product_id, _qty_needed) in &shopping_list.items {
                if store.has_product(product_id) {
                    let available_qty = store.get_inventory_level(product_id);
                    if available_qty > 0 {
                        let cost = store.get_product_cost(product_id).unwrap_or(f64::INFINITY);
                        product_options
                            .entry(*product_id)
                            .or_insert_with(Vec::new)
                            .push((store_id, cost, available_qty));
                    }
                }
            }
        }

        // Sort options for each product by price (cheapest first)
        for options in product_options.values_mut() {
            options.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal));
        }

        // Now allocate purchases optimally
        let mut total_cost = 0.0;
        for (product_id, qty_needed) in &shopping_list.items {
            let mut remaining_qty = *qty_needed;

            if let Some(options) = product_options.get(product_id) {
                // Buy from cheapest store first
                for &(_, cost, available_qty) in options {
                    let purchase_qty = std::cmp::min(available_qty, remaining_qty);
                    if purchase_qty > 0 {
                        total_cost += cost * purchase_qty as f64;
                        remaining_qty -= purchase_qty;

                        if remaining_qty == 0 {
                            break;
                        }
                    }
                }
            }

            // If we couldn't buy all needed quantity
            if remaining_qty > 0 {
                return f64::INFINITY; // Route cannot fulfill shopping list
            }
        }

        total_cost
    }
}

#[cfg(test)]
mod tests {
    // use super::*;
    // use crate::Product;

    // // Create test data with inventory constraints
    // fn create_test_data() -> (HashMap<StoreId, Store>, ShoppingList) {
    //     let mut stores = HashMap::new();

    //     // Store 1
    //     let mut store1_products = HashMap::new();
    //     store1_products.insert(1, Product::new("A", 7.0));
    //     store1_products.insert(2, Product::new("B", 8.0));
    //     store1_products.insert(6, Product::new("F", 10.0));

    //     let mut inventory1 = HashMap::new();
    //     inventory1.insert(1, 5); // 5 units of Product A
    //     inventory1.insert(2, 3); // 3 units of Product B
    //     inventory1.insert(6, 8); // 8 units of Product F

    //     stores.insert(
    //         1,
    //         Store::new_with_inventory(1, Location::new(10.0, 6.0), store1_products, inventory1),
    //     );

    //     // Store 2
    //     let mut store2_products = HashMap::new();
    //     store2_products.insert(3, Product::new("C", 10.0));
    //     store2_products.insert(4, Product::new("D", 8.0));
    //     store2_products.insert(5, Product::new("E", 10.0));

    //     let mut inventory2 = HashMap::new();
    //     inventory2.insert(3, 4); // 4 units of Product C
    //     inventory2.insert(4, 6); // 6 units of Product D
    //     inventory2.insert(5, 2); // 2 units of Product E

    //     stores.insert(
    //         2,
    //         Store::new_with_inventory(2, Location::new(12.0, 20.0), store2_products, inventory2),
    //     );

    //     // Store 3
    //     let mut store3_products = HashMap::new();
    //     store3_products.insert(3, Product::new("C", 5.0));
    //     store3_products.insert(4, Product::new("D", 4.0));
    //     store3_products.insert(6, Product::new("F", 6.0));

    //     let mut inventory3 = HashMap::new();
    //     inventory3.insert(3, 3); // 3 units of Product C
    //     inventory3.insert(4, 2); // 2 units of Product D
    //     inventory3.insert(6, 5); // 5 units of Product F

    //     stores.insert(
    //         3,
    //         Store::new_with_inventory(3, Location::new(20.0, 18.0), store3_products, inventory3),
    //     );

    //     // Store 4
    //     let mut store4_products = HashMap::new();
    //     store4_products.insert(3, Product::new("C", 8.0));
    //     store4_products.insert(4, Product::new("D", 7.0));
    //     store4_products.insert(6, Product::new("F", 12.0));

    //     let mut inventory4 = HashMap::new();
    //     inventory4.insert(3, 7); // 7 units of Product C
    //     inventory4.insert(4, 4); // 4 units of Product D
    //     inventory4.insert(6, 3); // 3 units of Product F

    //     stores.insert(
    //         4,
    //         Store::new_with_inventory(4, Location::new(15.0, 22.0), store4_products, inventory4),
    //     );

    //     // Store 5
    //     let mut store5_products = HashMap::new();
    //     store5_products.insert(1, Product::new("A", 6.0));
    //     store5_products.insert(2, Product::new("B", 7.0));
    //     store5_products.insert(5, Product::new("E", 8.0));

    //     let mut inventory5 = HashMap::new();
    //     inventory5.insert(1, 3); // 3 units of Product A
    //     inventory5.insert(2, 5); // 5 units of Product B
    //     inventory5.insert(5, 4); // 4 units of Product E

    //     stores.insert(
    //         5,
    //         Store::new_with_inventory(5, Location::new(10.0, 15.0), store5_products, inventory5),
    //     );

    //     // Create shopping list with quantities
    //     let mut shopping_list = ShoppingList::new();
    //     shopping_list.add_item(1, 2); // 2 units of A
    //     shopping_list.add_item(2, 1); // 1 unit of B
    //     shopping_list.add_item(3, 3); // 3 units of C
    //     shopping_list.add_item(4, 2); // 2 units of D

    //     (stores, shopping_list)
    // }

    // #[test]
    // fn test_inventory_tracking() {
    //     let (stores, shopping_list) = create_test_data();
    //     let mut bsl_psd = BSLPSD::new(stores);
    //     bsl_psd.precompute_data();

    //     // Test finding routes with inventory constraints
    //     let route = vec![1, 2, 3]; // Contains stores with all needed products
    //     assert!(bsl_psd.satisfies_list_with_inventory(&route, &shopping_list));

    //     // Test when inventory is insufficient
    //     let mut limited_list = ShoppingList::new();
    //     limited_list.add_item(1, 10); // More A than available in any store
    //     assert!(!bsl_psd.satisfies_list_with_inventory(&route, &limited_list));

    //     // Test inventory deduction
    //     assert!(bsl_psd.reserve_inventory(&route, &shopping_list));

    //     // Test cost calculation with inventory
    //     let (cost, fulfilled) =
    //         bsl_psd.calculate_shopping_cost_with_inventory(&route, &shopping_list);
    //     assert!(fulfilled);
    //     assert!(cost < f64::INFINITY);

    //     // Release inventory for subsequent tests
    //     bsl_psd.release_inventory(&route, &shopping_list);
    // }

    // #[test]
    // fn test_solve_with_inventory() {
    //     let (stores, shopping_list) = create_test_data();
    //     let mut bsl_psd = BSLPSD::new(stores);
    //     bsl_psd.precompute_data();

    //     let shopper_location = Location::new(0.0, 0.0);
    //     let customer_location = Location::new(20.0, 20.0);

    //     let skyline = bsl_psd.solve(&shopping_list, shopper_location, customer_location);

    //     // Verify that we have solutions
    //     assert!(!skyline.is_empty());

    //     // Verify skyline properties
    //     for i in 1..skyline.len() {
    //         // Shopping time should increase
    //         assert!(skyline[i - 1].shopping_time <= skyline[i].shopping_time);

    //         // Shopping cost should decrease (or at least not increase)
    //         assert!(skyline[i - 1].shopping_cost >= skyline[i].shopping_cost);
    //     }

    //     // Verify that each route in the skyline satisfies inventory constraints
    //     for route in &skyline {
    //         let (_, fulfilled) =
    //             bsl_psd.calculate_shopping_cost_with_inventory(&route.stores, &shopping_list);
    //         assert!(
    //             fulfilled,
    //             "Route {:?} does not satisfy inventory constraints",
    //             route.stores
    //         );
    //     }
    // }

    // #[test]
    // fn test_inventory_conflict() {
    //     let (stores, shopping_list) = create_test_data();
    //     let mut bsl_psd = BSLPSD::new(stores);
    //     bsl_psd.precompute_data();

    //     // Create a second shopping list that competes for the same inventory
    //     let mut competing_list = ShoppingList::new();
    //     competing_list.add_item(1, 4); // 4 units of A
    //     competing_list.add_item(3, 6); // 6 units of C

    //     // First, apply the first shopping list to reduce inventory
    //     let shopper_location = Location::new(0.0, 0.0);
    //     let customer_location = Location::new(20.0, 20.0);

    //     let skyline1 = bsl_psd.solve(&shopping_list, shopper_location, customer_location);
    //     assert!(!skyline1.is_empty());

    //     // Apply the first route's inventory reduction
    //     let first_route = &skyline1[0];
    //     assert!(bsl_psd.apply_route(first_route, &shopping_list));

    //     // Now try to solve with the competing list
    //     let skyline2 = bsl_psd.solve(&competing_list, shopper_location, customer_location);

    //     // We expect limited or no solutions due to inventory constraints
    //     if !skyline2.is_empty() {
    //         // If we have solutions, verify they use different stores or have adjusted for inventory
    //         for route in &skyline2 {
    //             let (_, fulfilled) =
    //                 bsl_psd.calculate_shopping_cost_with_inventory(&route.stores, &competing_list);
    //             assert!(
    //                 fulfilled,
    //                 "Route {:?} does not satisfy inventory constraints",
    //                 route.stores
    //             );
    //         }
    //     }
    // }
    // #[test]
    // fn test_snapshot_and_restore() {
    //     let (stores, shopping_list) = create_test_data();
    //     let mut bsl_psd = BSLPSD::new(stores);
    //     bsl_psd.precompute_data();

    //     // Take a snapshot of initial inventory
    //     let initial_snapshot = bsl_psd.snapshot_inventory();

    //     // Apply a route to reduce inventory
    //     let route = vec![1, 2, 3];
    //     assert!(bsl_psd.reserve_inventory(&route, &shopping_list));

    //     // Verify inventory was reduced
    //     let store1 = bsl_psd.stores[&1].lock().unwrap();
    //     assert!(store1.get_inventory_level(&1) < initial_snapshot[&1][&1]);

    //     // Restore inventory
    //     bsl_psd.restore_inventory(initial_snapshot.clone());

    //     // Verify inventory was restored
    //     let store1_after = bsl_psd.stores[&1].lock().unwrap();
    //     assert_eq!(
    //         store1_after.get_inventory_level(&1),
    //         initial_snapshot[&1][&1]
    //     );
    // }
}
