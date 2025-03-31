use personal_shopper::models::{ProductId, StoreId};
use personal_shopper::utils::init_map::init_map_with_road_network;
use personal_shopper::{
    algorithms::bsl_psd::BSLPSD,
    models::{Location, ShoppingList},
};
use std::collections::HashMap;

fn main() {
    let city_code = "AMS";
    let total_product_supply = 10;

    let threshold = 10000;

    // init searching map
    let (stores, travel_times) =
        match init_map_with_road_network(city_code, false, total_product_supply) {
            Ok(data) => data,
            Err(e) => {
                eprintln!("Error loading map data: {}", e);
                eprintln!(
                    "Ensure data files are in the 'data/' directory and have the correct format"
                );
                return;
            }
        };
    let (test_stores, test_travel_times) =
        match init_map_with_road_network(city_code, true, total_product_supply) {
            Ok(data) => data,
            Err(e) => {
                eprintln!("Error loading map data: {}", e);
                eprintln!(
                    "Ensure data files are in the 'data/' directory and have the correct format"
                );
                return;
            }
        };

    // create shopping lists
    let mut shopping_list = ShoppingList::new();

    // find avalible products
    let mut available_products = HashMap::new();
    for (_store_id, store) in &stores {
        let store = store;
        for (product_id, product) in &store.products {
            let entry = available_products
                .entry(*product_id)
                .or_insert((product.name.clone(), 0));
            entry.1 += store.get_inventory_level(product_id);
        }
    }
    println!("\navalible products:");
    for (product_id, (name, store_id)) in &available_products {
        println!(
            "  productID: {}, name: {}, total supply: {}",
            product_id, name, store_id
        );
    }

    // Init the shopping lists
    let mut product_ids: Vec<u32> = available_products.keys().cloned().collect();
    product_ids.sort();

    if product_ids.len() >= 5 {
        shopping_list.add_item(product_ids[0], 2);
        shopping_list.add_item(product_ids[1], 4);
        shopping_list.add_item(product_ids[2], 4);
        shopping_list.add_item(product_ids[3], 3);
        shopping_list.add_item(product_ids[4], 4);
        // shopping_list.add_item(product_ids[5], 3);
    }
    println!("\nShopping List:");
    for (product_id, quantity) in &shopping_list.items {
        let product_info = available_products.get(product_id);
        if let Some((name, _)) = product_info {
            println!("  Product {} ({}): {} units", product_id, name, quantity);
        }
    }
    let mut bsl_psd = BSLPSD::new_with_travel_times(stores, travel_times);
    let mut test_bsl_psd = BSLPSD::new_with_travel_times(test_stores, test_travel_times);
    bsl_psd.precompute_data();
    test_bsl_psd.precompute_data();

    // Solve PSD query
    let shopper_location = Location::new(0.0, 0.0);
    let customer_location = Location::new(20.0, 20.0);

    println!(
        "Shopper starting at location ({:.1}, {:.1})",
        shopper_location.x, shopper_location.y
    );
    println!(
        "Customer delivery location at ({:.1}, {:.1})",
        customer_location.x, customer_location.y
    );

    let start_time = std::time::Instant::now();
    // let results = bsl_psd.solve_with_debug(&shopping_list, shopper_location, customer_location);
    let (results, _best_time) = bsl_psd.solve_with_parallel(
        &shopping_list,
        shopper_location,
        customer_location,
        threshold,
    );
    let elapsed = start_time.elapsed();

    // Print results
    println!("Linear Skyline Results (found in {:.2?}):", elapsed);
    println!("------------------------------------------");

    if results.is_empty() {
        println!("No feasible routes found with current inventory constraints!");
    } else {
        for (i, route) in results.iter().enumerate() {
            println!("Route {}: {:?}", i + 1, route.stores);
            println!("  Shopping Time: {:.2} minutes", route.shopping_time);
            println!("  Shopping Cost: ${:.2}", route.shopping_cost);

            // Show optimized product allocation across stores
            println!("  Product Allocation:");

            // For each product, find optimal allocation across stores in the route
            let mut product_allocations: HashMap<ProductId, Vec<(StoreId, u32, f64)>> =
                HashMap::new();

            // First, collect all options for each product from all stores in the route
            for &store_id in &route.stores {
                let store = bsl_psd.stores[&store_id].read().unwrap();

                for (product_id, _qty_needed) in &shopping_list.items {
                    if store.has_product(product_id) {
                        let available_qty = store.get_inventory_level(product_id);
                        if available_qty > 0 {
                            let cost = store.get_product_cost(product_id).unwrap_or(f64::INFINITY);
                            product_allocations
                                .entry(*product_id)
                                .or_insert_with(Vec::new)
                                .push((store_id, available_qty, cost));
                        }
                    }
                }
            }

            // Then, for each product, sort options by cost and allocate optimally
            for (product_id, qty_needed) in &shopping_list.items {
                if let Some(options) = product_allocations.get_mut(product_id) {
                    // Sort by cost (lowest first)
                    options
                        .sort_by(|a, b| a.2.partial_cmp(&b.2).unwrap_or(std::cmp::Ordering::Equal));

                    // Calculate allocations
                    let mut remaining = *qty_needed;
                    let mut allocations = Vec::new();

                    for &(store_id, available, cost) in options.iter() {
                        if remaining == 0 {
                            break;
                        }

                        let purchase_qty = std::cmp::min(available, remaining);
                        if purchase_qty > 0 {
                            allocations.push((store_id, purchase_qty, cost));
                            remaining -= purchase_qty;
                        }
                    }

                    // Print allocations for this product
                    println!("    Product {}:", product_id);
                    for (store_id, qty, cost) in allocations {
                        println!(
                            "      Store {}: Buy {} units at ${:.2} each (${:.2} total)",
                            store_id,
                            qty,
                            cost,
                            qty as f64 * cost
                        );
                    }

                    if remaining > 0 {
                        println!("      WARNING: Could not allocate {} units", remaining);
                    }
                } else {
                    println!("    Product {}: No allocation found!", product_id);
                }
            }
            println!();
        }

        println!("Trade-off analysis:");
        if results.len() >= 2 {
            let fastest = &results.first().unwrap();
            let cheapest = &results.last().unwrap();

            println!("  Fastest route is {:.1}% faster but {:.1}% more expensive than the cheapest route.",
                100.0 * (cheapest.shopping_time - fastest.shopping_time) / cheapest.shopping_time,
                100.0 * (fastest.shopping_cost - cheapest.shopping_cost) / cheapest.shopping_cost);
        }
    }
    println!("Start searching with infinity product amout:");
    let start_time = std::time::Instant::now();
    // let results = bsl_psd.solve_with_debug(&shopping_list, shopper_location, customer_location);
    let (results, _best_time) = test_bsl_psd.solve_with_parallel(
        &shopping_list,
        shopper_location,
        customer_location,
        threshold,
    );
    let elapsed = start_time.elapsed();

    // Print results
    println!("Linear Skyline Results (found in {:.2?}):", elapsed);
    println!("------------------------------------------");

    if results.is_empty() {
        println!("No feasible routes found with current inventory constraints!");
    } else {
        for (i, route) in results.iter().enumerate() {
            println!("Route {}: {:?}", i + 1, route.stores);
            println!("  Shopping Time: {:.2} minutes", route.shopping_time);
            println!("  Shopping Cost: ${:.2}", route.shopping_cost);

            // Show optimized product allocation across stores
            println!("  Product Allocation:");

            // For each product, find optimal allocation across stores in the route
            let mut product_allocations: HashMap<ProductId, Vec<(StoreId, u32, f64)>> =
                HashMap::new();

            // First, collect all options for each product from all stores in the route
            for &store_id in &route.stores {
                let store = test_bsl_psd.stores[&store_id].read().unwrap();

                for (product_id, _qty_needed) in &shopping_list.items {
                    if store.has_product(product_id) {
                        let available_qty = store.get_inventory_level(product_id);
                        if available_qty > 0 {
                            let cost = store.get_product_cost(product_id).unwrap_or(f64::INFINITY);
                            product_allocations
                                .entry(*product_id)
                                .or_insert_with(Vec::new)
                                .push((store_id, available_qty, cost));
                        }
                    }
                }
            }

            // Then, for each product, sort options by cost and allocate optimally
            for (product_id, qty_needed) in &shopping_list.items {
                if let Some(options) = product_allocations.get_mut(product_id) {
                    // Sort by cost (lowest first)
                    options
                        .sort_by(|a, b| a.2.partial_cmp(&b.2).unwrap_or(std::cmp::Ordering::Equal));

                    // Calculate allocations
                    let mut remaining = *qty_needed;
                    let mut allocations = Vec::new();

                    for &(store_id, available, cost) in options.iter() {
                        if remaining == 0 {
                            break;
                        }

                        let purchase_qty = std::cmp::min(available, remaining);
                        if purchase_qty > 0 {
                            allocations.push((store_id, purchase_qty, cost));
                            remaining -= purchase_qty;
                        }
                    }

                    // Print allocations for this product
                    println!("    Product {}:", product_id);
                    for (store_id, qty, cost) in allocations {
                        println!(
                            "      Store {}: Buy {} units at ${:.2} each (${:.2} total)",
                            store_id,
                            qty,
                            cost,
                            qty as f64 * cost
                        );
                    }

                    if remaining > 0 {
                        println!("      WARNING: Could not allocate {} units", remaining);
                    }
                } else {
                    println!("    Product {}: No allocation found!", product_id);
                }
            }
            println!();
        }

        println!("Trade-off analysis:");
        if results.len() >= 2 {
            let fastest = &results.first().unwrap();
            let cheapest = &results.last().unwrap();

            println!("  Fastest route is {:.1}% faster but {:.1}% more expensive than the cheapest route.",
                100.0 * (cheapest.shopping_time - fastest.shopping_time) / cheapest.shopping_time,
                100.0 * (fastest.shopping_cost - cheapest.shopping_cost) / cheapest.shopping_cost);
        }
    }
}
