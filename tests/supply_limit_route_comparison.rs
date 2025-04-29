// Integration test for comparing minimum cost routes between limited and unlimited supply scenarios
use personal_shopper::algorithms::bsl_psd::BSLPSD;
use personal_shopper::algorithms::PSDSolver;
use personal_shopper::models::{Location, ShoppingList, ShoppingRoute};
use personal_shopper::utils::init_map::init_map_with_road_network;
use plotters::prelude::*;
use rand::Rng;
use std::collections::{HashMap, HashSet};
use std::error::Error;
use std::sync::Arc;
use std::time::Instant;

type StoreId = u32;

/// Generate all possible combinations of stores up to a certain size
fn generate_all_store_combinations(
    store_ids: &[StoreId],
    max_combination_size: usize,
) -> Vec<Vec<StoreId>> {
    let mut result = Vec::new();

    // Function to generate combinations recursively
    fn generate_combinations_helper(
        store_ids: &[StoreId],
        current: Vec<StoreId>,
        start: usize,
        max_size: usize,
        result: &mut Vec<Vec<StoreId>>,
    ) {
        // Base case: we've reached the maximum size for this combination
        if current.len() > 0 {
            result.push(current.clone());
        }

        // Stop if we've reached the maximum combination size
        if current.len() >= max_size {
            return;
        }

        // Try adding each remaining store to the current combination
        for i in start..store_ids.len() {
            let mut new_combination = current.clone();
            new_combination.push(store_ids[i]);
            generate_combinations_helper(store_ids, new_combination, i + 1, max_size, result);
        }
    }

    generate_combinations_helper(store_ids, Vec::new(), 0, max_combination_size, &mut result);
    result
}

/// Find all routes that achieve the minimum shopping cost
fn find_all_min_cost_routes(
    bsl_psd: &BSLPSD,
    shopping_list: &ShoppingList,
    shopper_location: &Location,
    customer_location: &Location,
    min_cost: f64,
    max_combination_size: usize,
) -> Vec<Vec<StoreId>> {
    println!("Finding all routes with minimum cost ${:.2}...", min_cost);

    // Get all store IDs
    let store_ids: Vec<StoreId> = bsl_psd.stores.keys().cloned().collect();
    println!("Total available stores: {}", store_ids.len());

    // Generate all possible store combinations up to max size
    println!(
        "Generating combinations (up to {} stores)...",
        max_combination_size
    );
    let combinations = generate_all_store_combinations(&store_ids, max_combination_size);
    println!("Generated {} store combinations", combinations.len());

    // Check each combination
    let mut min_cost_routes = Vec::new();
    let epsilon = 0.01; // Small tolerance for floating point comparison

    println!("Evaluating combinations for minimum cost routes...");
    for combination in combinations {
        // Skip empty combinations
        if combination.is_empty() {
            continue;
        }

        // Check if this combination can fulfill the shopping list
        if !bsl_psd.satisfies_list(&combination, shopping_list) {
            continue;
        }

        // Calculate shopping cost for this combination
        let cost = bsl_psd.calculate_shopping_cost(&combination, shopping_list);

        // If cost is close to the minimum cost, add to our results
        if (cost - min_cost).abs() < epsilon {
            min_cost_routes.push(combination);
        }
    }

    println!(
        "Found {} routes with minimum cost ${:.2}",
        min_cost_routes.len(),
        min_cost
    );
    min_cost_routes
}

#[test]
fn test_supply_limit_route_comparison() -> Result<(), Box<dyn Error>> {
    // Configuration parameters
    let city_code = "AMS"; // City code for Amsterdam
    let output_path = "min_cost_route_comparison.png"; // Output path for comparison chart
    let product_counts = [5, 10, 15]; // Different product counts to test
    let total_product_count = 30; // Total product types available
    let max_combination_size = 5; // Maximum number of stores in combinations to test

    println!("=== Testing Minimum Cost Route Comparison: Limited vs Unlimited Supply ===");

    // Data structures to store results
    let mut limited_min_costs = Vec::new();
    let mut unlimited_min_costs = Vec::new();
    let mut limited_min_cost_routes = Vec::new();
    let mut unlimited_min_cost_routes = Vec::new();
    let mut product_count_labels = Vec::new();

    // Define fixed locations for shopper and customer (to ensure consistency)
    let shopper_location = Location::new(4.8950, 52.3664); // Amsterdam city center restaurant district
    let customer_location = Location::new(4.8730, 52.3383); // Amsterdam city center southern residential area

    // Iterate through each product count
    for &product_count in &product_counts {
        println!("\n=== Testing with product count: {} ===", product_count);

        // PART 1: Initialize map with LIMITED supply
        println!("Loading map data with LIMITED supply...");
        let (limited_stores, limited_travel_times) =
            match init_map_with_road_network(city_code, false, total_product_count) {
                Ok(data) => data,
                Err(e) => {
                    eprintln!("Error loading limited supply map data: {}", e);
                    return Err(e.into());
                }
            };

        // PART 2: Initialize map with UNLIMITED supply
        println!("Loading map data with UNLIMITED supply...");
        let (unlimited_stores, unlimited_travel_times) =
            match init_map_with_road_network(city_code, true, total_product_count) {
                Ok(data) => data,
                Err(e) => {
                    eprintln!("Error loading unlimited supply map data: {}", e);
                    return Err(e.into());
                }
            };

        // Create shopping list
        let mut shopping_list = ShoppingList::new();

        // Find available products from limited supply stores (as baseline)
        let mut available_products = HashMap::new();
        for (_store_id, store) in &limited_stores {
            for (product_id, product) in &store.products {
                let entry = available_products
                    .entry(*product_id)
                    .or_insert((product.name.clone(), 0));
                entry.1 += store.get_inventory_level(product_id);
            }
        }

        println!(
            "\nAvailable products (from limited supply, {} total products):",
            available_products.len()
        );

        // Initialize shopping list with products
        let mut product_ids: Vec<u32> = available_products.keys().cloned().collect();
        product_ids.sort();

        // Make sure we don't exceed the available products
        let count_to_use = std::cmp::min(product_count as usize, product_ids.len());

        println!("\nShopping List (using {} products):", count_to_use);

        // Add specified number of products to the shopping list
        for i in 0..count_to_use {
            if i < product_ids.len() {
                let mut rng = rand::thread_rng();
                let quantity = rng.gen_range(2..=5); // Randomly generate quantity between 2-5
                shopping_list.add_item(product_ids[i], quantity);

                let product_info = available_products.get(&product_ids[i]);
                if let Some((name, _)) = product_info {
                    println!(
                        "  Added product {} ({}): {} units",
                        product_ids[i], name, quantity
                    );
                }
            }
        }

        // Initialize BSLPSD algorithms for both supply types
        let mut limited_bsl_psd =
            BSLPSD::new_with_travel_times(limited_stores.clone(), limited_travel_times);
        let mut unlimited_bsl_psd =
            BSLPSD::new_with_travel_times(unlimited_stores.clone(), unlimited_travel_times);

        limited_bsl_psd.precompute_data();
        unlimited_bsl_psd.precompute_data();

        // PART 3: Find minimum cost with LIMITED supply
        println!("\nFinding minimum cost with LIMITED supply...");
        let start_time_limited = Instant::now();

        // First, get minimum cost value
        let limited_min_cost = limited_bsl_psd.find_min_cost_route(
            &shopping_list,
            shopper_location,
            customer_location,
        );

        let elapsed_limited_min_cost = start_time_limited.elapsed();

        // Find all routes with this minimum cost
        let min_cost_limited_routes = if let Some(min_cost) = limited_min_cost {
            println!(
                "LIMITED SUPPLY: Minimum cost = ${:.2} (found in {:.2?})",
                min_cost, elapsed_limited_min_cost
            );

            // Find all routes that achieve this minimum cost
            let routes = find_all_min_cost_routes(
                &limited_bsl_psd,
                &shopping_list,
                &shopper_location,
                &customer_location,
                min_cost,
                max_combination_size,
            );

            let elapsed_limited = start_time_limited.elapsed();
            println!(
                "LIMITED SUPPLY: Found {} routes with minimum cost ${:.2} (in {:.2?})",
                routes.len(),
                min_cost,
                elapsed_limited
            );

            routes.len()
        } else {
            println!("LIMITED SUPPLY: No feasible routes found!");
            0
        };

        // PART 4: Find minimum cost with UNLIMITED supply
        println!("\nFinding minimum cost with UNLIMITED supply...");
        let start_time_unlimited = Instant::now();

        // First, get minimum cost value
        let unlimited_min_cost = unlimited_bsl_psd.find_min_cost_route(
            &shopping_list,
            shopper_location,
            customer_location,
        );

        let elapsed_unlimited_min_cost = start_time_unlimited.elapsed();

        // Find all routes with this minimum cost
        let min_cost_unlimited_routes = if let Some(min_cost) = unlimited_min_cost {
            println!(
                "UNLIMITED SUPPLY: Minimum cost = ${:.2} (found in {:.2?})",
                min_cost, elapsed_unlimited_min_cost
            );

            // Find all routes that achieve this minimum cost
            let routes = find_all_min_cost_routes(
                &unlimited_bsl_psd,
                &shopping_list,
                &shopper_location,
                &customer_location,
                min_cost,
                max_combination_size,
            );

            let elapsed_unlimited = start_time_unlimited.elapsed();
            println!(
                "UNLIMITED SUPPLY: Found {} routes with minimum cost ${:.2} (in {:.2?})",
                routes.len(),
                min_cost,
                elapsed_unlimited
            );

            routes.len()
        } else {
            println!("UNLIMITED SUPPLY: No feasible routes found!");
            0
        };

        // Store results for visualization
        if let Some(min_cost) = limited_min_cost {
            limited_min_costs.push(min_cost);
        } else {
            limited_min_costs.push(0.0);
        }

        if let Some(min_cost) = unlimited_min_cost {
            unlimited_min_costs.push(min_cost);
        } else {
            unlimited_min_costs.push(0.0);
        }

        limited_min_cost_routes.push(min_cost_limited_routes);
        unlimited_min_cost_routes.push(min_cost_unlimited_routes);
        product_count_labels.push(product_count);

        // Print detailed comparison
        println!(
            "\n=== Minimum Cost Route Comparison for {} Products ===",
            product_count
        );
        println!(
            "Limited Supply: {} routes with minimum cost",
            min_cost_limited_routes
        );
        println!(
            "Unlimited Supply: {} routes with minimum cost",
            min_cost_unlimited_routes
        );

        let ratio = if min_cost_limited_routes > 0 {
            min_cost_unlimited_routes as f64 / min_cost_limited_routes as f64
        } else if min_cost_unlimited_routes > 0 {
            f64::INFINITY
        } else {
            1.0
        };

        println!("Ratio (Unlimited/Limited): {:.2}x", ratio);
    }

    // Create comparison visualization
    create_comparison_chart(
        output_path,
        &product_count_labels,
        &limited_min_costs,
        &unlimited_min_costs,
        &limited_min_cost_routes,
        &unlimited_min_cost_routes,
    )?;

    println!(
        "\nMinimum cost route comparison visualization saved to: {}",
        output_path
    );

    Ok(())
}

/// Create a comparison chart for minimum cost routes between limited and unlimited supply
fn create_comparison_chart(
    output_path: &str,
    product_counts: &[i32],
    limited_costs: &[f64],
    unlimited_costs: &[f64],
    limited_routes: &[usize],
    unlimited_routes: &[usize],
) -> Result<(), Box<dyn Error>> {
    // Create root area with space for two charts
    let root = BitMapBackend::new(output_path, (1000, 800)).into_drawing_area();
    root.fill(&WHITE)?;

    // Add title
    let title = "Minimum Cost Route Comparison: Limited vs Unlimited Supply";
    let (title_area, charts_area) = root.split_vertically(60);

    title_area.fill(&WHITE)?;
    title_area.draw_text(
        title,
        &TextStyle::from(("sans-serif", 30).into_font()).color(&BLACK),
        (500, 30),
    )?;

    // Split main area into two charts
    let (cost_chart_area, route_chart_area) = charts_area.split_vertically(370);

    // CHART 1: Cost Comparison
    let max_cost = limited_costs
        .iter()
        .chain(unlimited_costs.iter())
        .fold(0.0, |a, b| match a.partial_cmp(b) {
            Some(std::cmp::Ordering::Greater) => a,
            _ => *b,
        });

    let mut cost_chart = ChartBuilder::on(&cost_chart_area)
        .caption(
            "Minimum Shopping Cost Comparison",
            ("sans-serif", 22).into_font(),
        )
        .margin(10)
        .x_label_area_size(40)
        .y_label_area_size(60)
        .build_cartesian_2d(
            0..(*product_counts.last().unwrap_or(&0) + 5),
            0.0..(max_cost * 1.2),
        )?;

    cost_chart
        .configure_mesh()
        .x_desc("Number of Products")
        .y_desc("Minimum Cost ($)")
        .axis_desc_style(("sans-serif", 18))
        .label_style(("sans-serif", 15))
        .draw()?;

    // Plot limited supply costs
    cost_chart
        .draw_series(LineSeries::new(
            product_counts
                .iter()
                .zip(limited_costs.iter())
                .map(|(x, y)| (*x, *y)),
            BLUE.stroke_width(3),
        ))?
        .label("Limited Supply")
        .legend(|(x, y)| PathElement::new(vec![(x, y), (x + 20, y)], BLUE.stroke_width(3)));

    cost_chart.draw_series(
        product_counts
            .iter()
            .zip(limited_costs.iter())
            .map(|(x, y)| Circle::new((*x, *y), 5, BLUE.filled())),
    )?;

    // Plot unlimited supply costs
    cost_chart
        .draw_series(LineSeries::new(
            product_counts
                .iter()
                .zip(unlimited_costs.iter())
                .map(|(x, y)| (*x, *y)),
            GREEN.stroke_width(3),
        ))?
        .label("Unlimited Supply")
        .legend(|(x, y)| PathElement::new(vec![(x, y), (x + 20, y)], GREEN.stroke_width(3)));

    cost_chart.draw_series(
        product_counts
            .iter()
            .zip(unlimited_costs.iter())
            .map(|(x, y)| Circle::new((*x, *y), 5, GREEN.filled())),
    )?;

    // Add cost value labels
    for (i, (&x, &y)) in product_counts.iter().zip(limited_costs.iter()).enumerate() {
        cost_chart.draw_series(std::iter::once(Text::new(
            format!("${:.2}", y),
            (x, y + (max_cost * 0.03)),
            ("sans-serif", 12).into_font(),
        )))?;

        let unlimited_y = unlimited_costs[i];
        cost_chart.draw_series(std::iter::once(Text::new(
            format!("${:.2}", unlimited_y),
            (x, unlimited_y + (max_cost * 0.03)),
            ("sans-serif", 12).into_font(),
        )))?;
    }

    // Add legend for cost chart
    cost_chart
        .configure_series_labels()
        .background_style(&WHITE.mix(0.8))
        .border_style(&BLACK)
        .position(SeriesLabelPosition::UpperRight)
        .draw()?;

    // CHART 2: Route Count Comparison
    let max_routes = limited_routes
        .iter()
        .chain(unlimited_routes.iter())
        .copied()
        .max()
        .unwrap_or(1);

    // Calculate x values for positioning bars
    let x_values: Vec<f64> = (0..product_counts.len())
        .map(|i| i as f64 * 3.0) // Group bars with spacing
        .collect();

    let mut route_chart = ChartBuilder::on(&route_chart_area)
        .caption(
            "Number of Routes with Minimum Cost",
            ("sans-serif", 22).into_font(),
        )
        .margin(10)
        .x_label_area_size(40)
        .y_label_area_size(60)
        .build_cartesian_2d(
            -0.5f64..((x_values.len() as f64) * 3.0 - 0.5),
            0.0..((max_routes as f64) * 1.2),
        )?;

    route_chart
        .configure_mesh()
        .disable_x_mesh()
        .x_labels(product_counts.len())
        .x_label_formatter(&|x| {
            let idx = (*x / 3.0).round() as usize;
            if idx < product_counts.len() {
                format!("{} Products", product_counts[idx])
            } else {
                String::new()
            }
        })
        .y_desc("Number of Routes")
        .axis_desc_style(("sans-serif", 18))
        .label_style(("sans-serif", 15))
        .draw()?;

    // Draw limited supply bars
    for (i, &count) in limited_routes.iter().enumerate() {
        let x = x_values[i];

        // Draw bar
        route_chart.draw_series(std::iter::once(Rectangle::new(
            [(x, 0.0), (x + 1.0, count as f64)],
            BLUE.mix(0.7).filled(),
        )))?;

        // Add label
        route_chart.draw_series(std::iter::once(Text::new(
            format!("{}", count),
            (x + 0.5, count as f64 + (max_routes as f64 * 0.05)),
            ("sans-serif", 15).into_font(),
        )))?;
    }

    // Draw unlimited supply bars
    for (i, &count) in unlimited_routes.iter().enumerate() {
        let x = x_values[i] + 1.2; // Position next to limited bar

        // Draw bar
        route_chart.draw_series(std::iter::once(Rectangle::new(
            [(x, 0.0), (x + 1.0, count as f64)],
            GREEN.mix(0.7).filled(),
        )))?;

        // Add label
        route_chart.draw_series(std::iter::once(Text::new(
            format!("{}", count),
            (x + 0.5, count as f64 + (max_routes as f64 * 0.05)),
            ("sans-serif", 15).into_font(),
        )))?;
    }

    // Add legend for route count chart
    route_chart.draw_series(std::iter::once(Rectangle::new(
        [
            (x_values[0], max_routes as f64 * 1.1),
            (x_values[0] + 1.0, max_routes as f64 * 1.15),
        ],
        BLUE.mix(0.7).filled(),
    )))?;

    route_chart.draw_series(std::iter::once(Text::new(
        "Limited Supply",
        (x_values[0] + 1.2, max_routes as f64 * 1.125),
        ("sans-serif", 15).into_font(),
    )))?;

    route_chart.draw_series(std::iter::once(Rectangle::new(
        [
            (x_values[0] + 8.0, max_routes as f64 * 1.1),
            (x_values[0] + 9.0, max_routes as f64 * 1.15),
        ],
        GREEN.mix(0.7).filled(),
    )))?;

    route_chart.draw_series(std::iter::once(Text::new(
        "Unlimited Supply",
        (x_values[0] + 9.2, max_routes as f64 * 1.125),
        ("sans-serif", 15).into_font(),
    )))?;

    root.present()?;
    Ok(())
}
