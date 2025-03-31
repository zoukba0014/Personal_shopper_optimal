// Integration test for comparing infinite vs. limited product supply
use personal_shopper::algorithms::bsl_psd::BSLPSD;
use personal_shopper::models::{Location, ShoppingList, ShoppingRoute, StoreId};
use personal_shopper::utils::init_map::init_map_with_road_network;
use plotters::prelude::*;
use std::collections::HashMap;
use std::error::Error;

#[test]
fn test_supply_comparison() -> Result<(), Box<dyn Error>> {
    // Configuration parameters
    let city_code = "AMS"; // City code
    let product_counts = [5, 10, 15]; // Test three different product counts
    let threshold = 10000;

    println!(
        "=== Testing BSL-PSD with Limited vs. Infinite Supply for Different Product Counts ==="
    );

    // Test for each product count
    for &product_count in &product_counts {
        println!("\n### Testing with {} products ###", product_count);

        // Set output filenames for current product count
        let limited_output_path = format!("bsl_psd_limited_supply_{}_products.png", product_count);
        let infinite_output_path =
            format!("bsl_psd_infinite_supply_{}_products.png", product_count);
        let comparison_output_path = format!("supply_comparison_{}_products.png", product_count);
        let performance_comparison_path =
            format!("performance_comparison_{}_products.png", product_count);

        // Initialize map data for BOTH limited and infinite supply
        println!(
            "Loading map data for limited supply with {} products...",
            product_count
        );
        let (limited_stores, limited_travel_times) =
            match init_map_with_road_network(city_code, false, product_count) {
                Ok(data) => data,
                Err(e) => {
                    eprintln!("Error loading map data: {}", e);
                    eprintln!(
                        "Ensure data files are in the 'data/' directory and have the correct format"
                    );
                    return Err(e.into());
                }
            };

        println!(
            "Loading map data for infinite supply with {} products...",
            product_count
        );
        let (infinite_stores, infinite_travel_times) =
            match init_map_with_road_network(city_code, true, product_count) {
                Ok(data) => data,
                Err(e) => {
                    eprintln!("Error loading map data: {}", e);
                    eprintln!(
                        "Ensure data files are in the 'data/' directory and have the correct format"
                    );
                    return Err(e.into());
                }
            };

        // Create shopping list (same for both tests)
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
            "\nAvailable products (from limited supply, {} products):",
            product_count
        );
        for (product_id, (name, total_supply)) in &available_products {
            println!(
                "  Product ID: {}, Name: {}, Total supply: {}",
                product_id, name, total_supply
            );
        }

        // Initialize shopping list
        let mut product_ids: Vec<u32> = available_products.keys().cloned().collect();
        product_ids.sort();

        // Create a demanding shopping list to test supply constraints
        // Make sure we don't exceed the available products
        let count_to_use = std::cmp::min(5, product_ids.len());

        println!("\nShopping List (using {} products):", count_to_use);
        if count_to_use >= 5 {
            shopping_list.add_item(product_ids[0], 4); // More demanding shopping list
            shopping_list.add_item(product_ids[1], 6);
            shopping_list.add_item(product_ids[2], 5);
            shopping_list.add_item(product_ids[3], 4);
            shopping_list.add_item(product_ids[4], 7);
        } else {
            // If available products are fewer than 5, add based on available count
            for i in 0..count_to_use {
                let quantity = 3 + (i % 3); // 3-5 units per product
                shopping_list.add_item(product_ids[i], quantity as u32);
            }
        }

        for (product_id, quantity) in &shopping_list.items {
            let product_info = available_products.get(product_id);
            if let Some((name, _)) = product_info {
                println!("  Product {} ({}): {} units", product_id, name, quantity);
            }
        }

        // Initialize BSLPSD algorithms for both supply types
        let mut limited_bsl_psd =
            BSLPSD::new_with_travel_times(limited_stores.clone(), limited_travel_times);
        let mut infinite_bsl_psd =
            BSLPSD::new_with_travel_times(infinite_stores.clone(), infinite_travel_times);

        limited_bsl_psd.precompute_data();
        infinite_bsl_psd.precompute_data();

        // Define start and end points (same for both tests)
        let shopper_location = Location::new(0.0, 0.0);
        let customer_location = Location::new(10.0, 10.0);

        println!(
            "Shopper starting location ({:.1}, {:.1})",
            shopper_location.x, shopper_location.y
        );
        println!(
            "Customer delivery location ({:.1}, {:.1})",
            customer_location.x, customer_location.y
        );

        // PART 1: Generate results for limited supply
        println!(
            "\nStarting route planning with {} products, LIMITED supply...",
            product_count
        );
        let start_time_limited = std::time::Instant::now();
        let (limited_results, best_search_time) = limited_bsl_psd.solve_with_parallel(
            &shopping_list,
            shopper_location,
            customer_location,
            threshold,
        );
        let elapsed_limited = start_time_limited.elapsed();

        println!(
            "Limited Supply Results with {} products (found in {:.2?}):",
            product_count, elapsed_limited
        );
        println!("------------------------------------------");

        if limited_results.is_empty() {
            println!(
                "No feasible routes found with limited supply for {} products!",
                product_count
            );
            println!(
                "This is expected if the shopping list demands exceed total available inventory."
            );
        } else {
            // Randomize store locations for limited supply
            let limited_store_locations = generate_store_locations(&limited_results);

            // Print each route's information for limited supply
            for (i, route) in limited_results.iter().enumerate() {
                println!("Limited Supply Route {}: {:?}", i + 1, route.stores);
                println!("  Shopping Time: {:.2} minutes", route.shopping_time);
                println!("  Shopping Cost: ${:.2}", route.shopping_cost);
                println!("  Store Count: {}", route.stores.len());
            }

            // Visualize routes for limited supply
            visualize_all_routes(
                &limited_output_path,
                &limited_results,
                &limited_store_locations,
                &shopper_location,
                &customer_location,
                &format!(
                    "BSL-PSD Shopping Routes ({} products, Limited Supply)",
                    product_count
                ),
            )?;

            println!(
                "Limited supply visualization complete. Output saved to: {}",
                limited_output_path
            );
        }

        // PART 2: Generate results for infinite supply
        println!(
            "\nStarting route planning with {} products, INFINITE supply...",
            product_count
        );
        let start_time_infinite = std::time::Instant::now();
        let (infinite_results, inf_best_search_time) = infinite_bsl_psd.solve_with_parallel(
            &shopping_list,
            shopper_location,
            customer_location,
            threshold,
        );
        let elapsed_infinite = start_time_infinite.elapsed();

        println!(
            "Infinite Supply Results with {} products (found in {:.2?}):",
            product_count, elapsed_infinite
        );
        println!("------------------------------------------");

        if infinite_results.is_empty() {
            println!(
                "No feasible routes found with infinite supply for {} products!",
                product_count
            );
            println!("This is unexpected as infinite supply should always find a solution.");
        } else {
            // Randomize store locations for infinite supply
            let infinite_store_locations = generate_store_locations(&infinite_results);

            // Print each route's information for infinite supply
            for (i, route) in infinite_results.iter().enumerate() {
                println!("Infinite Supply Route {}: {:?}", i + 1, route.stores);
                println!("  Shopping Time: {:.2} minutes", route.shopping_time);
                println!("  Shopping Cost: ${:.2}", route.shopping_cost);
                println!("  Store Count: {}", route.stores.len());
            }

            // Visualize routes for infinite supply
            visualize_all_routes(
                &infinite_output_path,
                &infinite_results,
                &infinite_store_locations,
                &shopper_location,
                &customer_location,
                &format!(
                    "BSL-PSD Shopping Routes ({} products, Infinite Supply)",
                    product_count
                ),
            )?;

            println!(
                "Infinite supply visualization complete. Output saved to: {}",
                infinite_output_path
            );
        }

        // PART 3: Comparison between limited and infinite supply
        if !limited_results.is_empty() && !infinite_results.is_empty() {
            println!(
                "\n=== Supply Comparison Analysis for {} products ===",
                product_count
            );

            // Calculate total search time and best route search time
            let limited_total_time_ms = elapsed_limited.as_millis() as f64;
            let infinite_total_time_ms = elapsed_infinite.as_millis() as f64;
            let limited_best_time_ms = best_search_time.as_millis() as f64;
            let infinite_best_time_ms = inf_best_search_time.as_millis() as f64;

            // Calculate additional search time
            let limited_search_time_ms = limited_total_time_ms - limited_best_time_ms;
            let infinite_search_time_ms = infinite_total_time_ms - infinite_best_time_ms;

            // Print information
            println!("\nPerformance Metrics ({} products):", product_count);
            println!("------------------------------------------");
            println!(
                "LIMITED SUPPLY   - Total Routes: {}, Total Time: {:.2}ms, Best Route Time: {:.2}ms, Search Time: {:.2}ms",
                limited_results.len(), limited_total_time_ms, limited_best_time_ms, limited_search_time_ms
            );
            println!(
                "INFINITE SUPPLY  - Total Routes: {}, Total Time: {:.2}ms, Best Route Time: {:.2}ms, Search Time: {:.2}ms",
                infinite_results.len(), infinite_total_time_ms, infinite_best_time_ms, infinite_search_time_ms
            );

            // Compare best route metrics
            let limited_fastest = &limited_results[0];
            let infinite_fastest = &infinite_results[0];

            println!("\nFastest Route Comparison:");
            println!(
                "  Limited Supply: {:.2} minutes, ${:.2}, {} stores",
                limited_fastest.shopping_time,
                limited_fastest.shopping_cost,
                limited_fastest.stores.len()
            );
            println!(
                "  Infinite Supply: {:.2} minutes, ${:.2}, {} stores",
                infinite_fastest.shopping_time,
                infinite_fastest.shopping_cost,
                infinite_fastest.stores.len()
            );

            // Time savings with infinite supply
            let time_savings = limited_fastest.shopping_time - infinite_fastest.shopping_time;
            let time_savings_percent = (time_savings / limited_fastest.shopping_time) * 100.0;

            // Cost difference with infinite supply
            let cost_diff = infinite_fastest.shopping_cost - limited_fastest.shopping_cost;
            let cost_diff_percent = (cost_diff / limited_fastest.shopping_cost) * 100.0;

            println!(
                "\nImpact of Infinite Supply for {} products:",
                product_count
            );
            if time_savings > 0.0 {
                println!(
                    "  Time Savings: {:.2} minutes ({:.1}%)",
                    time_savings, time_savings_percent
                );
            } else {
                println!(
                    "  Time Increase: {:.2} minutes ({:.1}%)",
                    -time_savings, -time_savings_percent
                );
            }

            if cost_diff > 0.0 {
                println!(
                    "  Cost Increase: ${:.2} ({:.1}%)",
                    cost_diff, cost_diff_percent
                );
            } else {
                println!(
                    "  Cost Savings: ${:.2} ({:.1}%)",
                    -cost_diff, -cost_diff_percent
                );
            }

            // Create comparison visualization
            create_supply_comparison_chart(
                &comparison_output_path,
                &limited_results,
                &infinite_results,
                product_count,
            )?;

            println!(
                "Supply comparison visualization saved to: {}",
                comparison_output_path
            );

            // Performance comparison chart
            create_performance_comparison_chart(
                &performance_comparison_path,
                &limited_results,
                &infinite_results,
                limited_total_time_ms,
                infinite_total_time_ms,
                limited_best_time_ms,
                infinite_best_time_ms,
                product_count,
            )?;
            println!(
                "Performance comparison visualization saved to: {}",
                performance_comparison_path
            );
        }

        println!("\nCompleted testing with {} products\n", product_count);
        println!("=======================================================");
    }

    Ok(())
}

/// Generate random store locations for routes
fn generate_store_locations(routes: &[ShoppingRoute]) -> HashMap<StoreId, (f64, f64)> {
    let mut store_locations: HashMap<StoreId, (f64, f64)> = HashMap::new();

    // Define the display area bounds with more space between stores
    let display_min_x = -70.0;
    let display_max_x = 70.0;
    let display_min_y = -70.0;
    let display_max_y = 70.0;

    // First, identify all stores used in any route
    let mut used_store_ids = std::collections::HashSet::new();
    for route in routes {
        for store_id in &route.stores {
            used_store_ids.insert(*store_id);
        }
    }

    // Only generate positions for stores that are used in routes
    for &store_id in &used_store_ids {
        // Generate random position for each store
        let x = display_min_x + (display_max_x - display_min_x) * rand::random::<f64>();
        let y = display_min_y + (display_max_y - display_min_y) * rand::random::<f64>();
        store_locations.insert(store_id, (x, y));
    }

    store_locations
}

/// Visualize all shopping routes
fn visualize_all_routes(
    output_path: &str,
    routes: &[ShoppingRoute],
    store_locations: &HashMap<StoreId, (f64, f64)>,
    shopper_start: &Location,
    customer_location: &Location,
    chart_title: &str,
) -> Result<(), Box<dyn Error>> {
    // Determine chart boundaries
    let (min_x, max_x, min_y, max_y) =
        determine_bounds(store_locations, shopper_start, customer_location);

    // Create chart
    let root = BitMapBackend::new(output_path, (1000, 800)).into_drawing_area();
    root.fill(&WHITE)?;

    // Set up coordinate system
    let mut chart = ChartBuilder::on(&root)
        .caption(
            format!("{} ({} routes)", chart_title, routes.len()),
            ("sans-serif", 20).into_font(),
        )
        .margin(10)
        .x_label_area_size(30)
        .y_label_area_size(30)
        .build_cartesian_2d(min_x..max_x, min_y..max_y)?;

    chart.configure_mesh().draw()?;

    // Draw all store locations
    let mut all_route_stores = Vec::new();
    for route in routes {
        for store_id in &route.stores {
            if !all_route_stores.contains(store_id) {
                all_route_stores.push(*store_id);
            }
        }
    }

    for (store_id, (x, y)) in store_locations {
        // All stores in store_locations are used in routes, so we always use GREEN
        let style = ShapeStyle::from(&GREEN).filled();

        chart
            .draw_series(std::iter::once(Circle::new((*x, *y), 8, style)))?
            .label(format!("Store {}", store_id))
            .legend(move |(x, y)| Circle::new((x, y), 8, style));
    }

    // Draw shopper starting point
    chart
        .draw_series(std::iter::once(Circle::new(
            (shopper_start.x, shopper_start.y),
            10,
            ShapeStyle::from(&BLUE).filled(),
        )))?
        .label("Shopper Start")
        .legend(|(x, y)| Circle::new((x, y), 10, ShapeStyle::from(&BLUE).filled()));

    // Draw customer location
    chart
        .draw_series(std::iter::once(Circle::new(
            (customer_location.x, customer_location.y),
            10,
            ShapeStyle::from(&RED).filled(),
        )))?
        .label("Customer Location")
        .legend(|(x, y)| Circle::new((x, y), 10, ShapeStyle::from(&RED).filled()));

    // Draw all routes with different colors
    let colors = [
        &RED,
        &BLUE,
        &GREEN,
        &MAGENTA,
        &CYAN,
        &RGBColor(255, 165, 0),  // Orange
        &RGBColor(128, 0, 128),  // Purple
        &RGBColor(0, 128, 128),  // Teal
        &RGBColor(128, 128, 0),  // Olive
        &RGBColor(70, 130, 180), // Steel blue
    ];

    for (i, route) in routes.iter().enumerate() {
        if !route.stores.is_empty() {
            let mut path_points = Vec::new();
            path_points.push((shopper_start.x, shopper_start.y));

            for store_id in &route.stores {
                if let Some(&(x, y)) = store_locations.get(store_id) {
                    path_points.push((x, y));
                }
            }

            path_points.push((customer_location.x, customer_location.y));

            let color = colors[i % colors.len()];
            chart
                .draw_series(LineSeries::new(path_points, color.mix(0.7).stroke_width(2)))?
                .label(format!(
                    "Route {} (Time: {:.1} min, Cost: ${:.2})",
                    i + 1,
                    route.shopping_time,
                    route.shopping_cost
                ))
                .legend(|(x, y)| {
                    PathElement::new(vec![(x, y), (x + 20, y)], color.mix(0.7).stroke_width(2))
                });
        }
    }

    // Calculate and draw the time-cost curve
    if routes.len() >= 2 {
        let time_cost_path = format!(
            "time_cost_analysis_{}.png",
            output_path.strip_suffix(".png").unwrap_or(output_path)
        );
        create_time_cost_chart(&time_cost_path, routes, chart_title)?;
        println!("Time-cost analysis saved to: {}", time_cost_path);
    }

    chart
        .configure_series_labels()
        .background_style(&WHITE.mix(0.8))
        .border_style(&BLACK)
        .position(SeriesLabelPosition::UpperLeft)
        .draw()?;

    root.present()?;

    Ok(())
}

/// Create a time-cost trade-off analysis chart
fn create_time_cost_chart(
    output_path: &str,
    routes: &[ShoppingRoute],
    chart_title: &str,
) -> Result<(), Box<dyn Error>> {
    // Get time and cost ranges
    let mut min_time = f64::MAX;
    let mut max_time = f64::MIN;
    let mut min_cost = f64::MAX;
    let mut max_cost = f64::MIN;

    for route in routes {
        min_time = min_time.min(route.shopping_time);
        max_time = max_time.max(route.shopping_time);
        min_cost = min_cost.min(route.shopping_cost);
        max_cost = max_cost.max(route.shopping_cost);
    }

    // Add padding
    let time_padding = (max_time - min_time) * 0.1;
    let cost_padding = (max_cost - min_cost) * 0.1;

    // Create root area
    let root = BitMapBackend::new(output_path, (800, 600)).into_drawing_area();
    root.fill(&WHITE)?;

    // Create chart
    let mut chart = ChartBuilder::on(&root)
        .caption(
            format!("Time-Cost Trade-off Analysis - {}", chart_title),
            ("sans-serif", 20).into_font(),
        )
        .margin(10)
        .x_label_area_size(30)
        .y_label_area_size(30)
        .build_cartesian_2d(
            (min_time - time_padding)..(max_time + time_padding),
            (min_cost - cost_padding)..(max_cost + cost_padding),
        )?;

    chart
        .configure_mesh()
        .x_desc("Shopping Time (minutes)")
        .y_desc("Shopping Cost ($)")
        .draw()?;

    // Draw time-cost points
    let colors = [
        &RED,
        &BLUE,
        &GREEN,
        &MAGENTA,
        &CYAN,
        &RGBColor(255, 165, 0),  // orange
        &RGBColor(128, 0, 128),  // purple
        &RGBColor(0, 128, 128),  // teal
        &RGBColor(128, 128, 0),  // olive
        &RGBColor(70, 130, 180), // steel blue
    ];

    for (i, route) in routes.iter().enumerate() {
        let color = colors[i % colors.len()];

        chart
            .draw_series(std::iter::once(Circle::new(
                (route.shopping_time, route.shopping_cost),
                5,
                color.filled(),
            )))?
            .label(format!("Route {}", i + 1))
            .legend(move |(x, y)| Circle::new((x, y), 5, color.filled()));
    }

    // Draw Pareto frontier
    let mut points: Vec<(f64, f64)> = routes
        .iter()
        .map(|r| (r.shopping_time, r.shopping_cost))
        .collect();

    // Sort points by time for proper line drawing
    points.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap_or(std::cmp::Ordering::Equal));

    if points.len() >= 2 {
        chart
            .draw_series(LineSeries::new(points, BLACK.mix(0.5).stroke_width(2)))?
            .label("Pareto frontier")
            .legend(|(x, y)| {
                PathElement::new(vec![(x, y), (x + 20, y)], BLACK.mix(0.5).stroke_width(2))
            });
    }

    chart
        .configure_series_labels()
        .background_style(&WHITE.mix(0.8))
        .border_style(&BLACK)
        .position(SeriesLabelPosition::UpperRight)
        .draw()?;

    root.present()?;

    Ok(())
}

/// Create a direct comparison chart between limited and infinite supply
fn create_supply_comparison_chart(
    output_path: &str,
    limited_routes: &[ShoppingRoute],
    infinite_routes: &[ShoppingRoute],
    product_count: u32,
) -> Result<(), Box<dyn Error>> {
    // Get time and cost ranges for both sets
    let mut min_time = f64::MAX;
    let mut max_time = f64::MIN;
    let mut min_cost = f64::MAX;
    let mut max_cost = f64::MIN;

    for route in limited_routes.iter().chain(infinite_routes.iter()) {
        min_time = min_time.min(route.shopping_time);
        max_time = max_time.max(route.shopping_time);
        min_cost = min_cost.min(route.shopping_cost);
        max_cost = max_cost.max(route.shopping_cost);
    }

    // Add padding
    let time_padding = (max_time - min_time) * 0.1;
    let cost_padding = (max_cost - min_cost) * 0.1;

    // Create root area
    let root = BitMapBackend::new(output_path, (800, 600)).into_drawing_area();
    root.fill(&WHITE)?;

    // Create chart
    let mut chart = ChartBuilder::on(&root)
        .caption(
            format!(
                "Limited vs. Infinite Supply Comparison ({} Products)",
                product_count
            ),
            ("sans-serif", 20).into_font(),
        )
        .margin(10)
        .x_label_area_size(30)
        .y_label_area_size(30)
        .build_cartesian_2d(
            (min_time - time_padding)..(max_time + time_padding),
            (min_cost - cost_padding)..(max_cost + cost_padding),
        )?;

    chart
        .configure_mesh()
        .x_desc("Shopping Time (minutes)")
        .y_desc("Shopping Cost ($)")
        .draw()?;

    // Draw limited supply points
    for (i, route) in limited_routes.iter().enumerate() {
        chart
            .draw_series(std::iter::once(Circle::new(
                (route.shopping_time, route.shopping_cost),
                6,
                RGBColor(220, 50, 50).filled(), // Red for limited
            )))?
            .label(if i == 0 { "Limited Supply Routes" } else { "" });
    }

    // Draw infinite supply points
    for (i, route) in infinite_routes.iter().enumerate() {
        chart
            .draw_series(std::iter::once(Circle::new(
                (route.shopping_time, route.shopping_cost),
                6,
                RGBColor(50, 50, 220).filled(), // Blue for infinite
            )))?
            .label(if i == 0 { "Infinite Supply Routes" } else { "" });
    }

    // Draw limited supply Pareto frontier
    let mut limited_points: Vec<(f64, f64)> = limited_routes
        .iter()
        .map(|r| (r.shopping_time, r.shopping_cost))
        .collect();

    limited_points.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap_or(std::cmp::Ordering::Equal));

    if limited_points.len() >= 2 {
        chart
            .draw_series(LineSeries::new(
                limited_points,
                RGBColor(220, 50, 50).mix(0.7).stroke_width(2),
            ))?
            .label("Limited Supply Frontier");
    }

    // Draw infinite supply Pareto frontier
    let mut infinite_points: Vec<(f64, f64)> = infinite_routes
        .iter()
        .map(|r| (r.shopping_time, r.shopping_cost))
        .collect();

    infinite_points.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap_or(std::cmp::Ordering::Equal));

    if infinite_points.len() >= 2 {
        chart
            .draw_series(LineSeries::new(
                infinite_points,
                RGBColor(50, 50, 220).mix(0.7).stroke_width(2),
            ))?
            .label("Infinite Supply Frontier");
    }

    // Highlight best routes from each category
    if !limited_routes.is_empty() {
        let best_limited = &limited_routes[0]; // Fastest route
                                               // Draw circle with border
        chart.draw_series(std::iter::once(Circle::new(
            (best_limited.shopping_time, best_limited.shopping_cost),
            8,
            RGBColor(220, 50, 50).filled(),
        )))?;
        // Draw border circle
        chart
            .draw_series(std::iter::once(Circle::new(
                (best_limited.shopping_time, best_limited.shopping_cost),
                10,
                RGBColor(0, 0, 0).stroke_width(2),
            )))?
            .label("Best Limited Route");
    }

    if !infinite_routes.is_empty() {
        let best_infinite = &infinite_routes[0]; // Fastest route
                                                 // Draw circle with border
        chart.draw_series(std::iter::once(Circle::new(
            (best_infinite.shopping_time, best_infinite.shopping_cost),
            8,
            RGBColor(50, 50, 220).filled(),
        )))?;
        // Draw border circle
        chart
            .draw_series(std::iter::once(Circle::new(
                (best_infinite.shopping_time, best_infinite.shopping_cost),
                10,
                RGBColor(0, 0, 0).stroke_width(2),
            )))?
            .label("Best Infinite Route");
    }

    chart
        .configure_series_labels()
        .background_style(&WHITE.mix(0.8))
        .border_style(&BLACK)
        .position(SeriesLabelPosition::UpperRight)
        .draw()?;

    root.present()?;

    Ok(())
}

/// Create a performance comparison chart showing search time and route count differences
fn create_performance_comparison_chart(
    output_path: &str,
    limited_routes: &[ShoppingRoute],
    infinite_routes: &[ShoppingRoute],
    limited_total_time_ms: f64,
    infinite_total_time_ms: f64,
    limited_best_time_ms: f64,
    infinite_best_time_ms: f64,
    product_count: u32,
) -> Result<(), Box<dyn Error>> {
    // Create root area
    let root = BitMapBackend::new(output_path, (1200, 800)).into_drawing_area();
    root.fill(&WHITE)?;

    // Add title
    let ratio = limited_total_time_ms / infinite_total_time_ms;
    let title = format!(
        "Supply Comparison - {} Products (Limited is {:.1}x slower than Infinite)",
        product_count, ratio
    );

    // Create title area and content area with the title
    let (title_area, content_area) = root.split_vertically(60);
    title_area.fill(&WHITE)?;
    title_area.draw_text(
        &title,
        &TextStyle::from(("sans-serif", 30).into_font()).color(&BLACK),
        (600, 30),
    )?;

    // Split the content area into a 2x2 grid
    let areas = content_area.split_evenly((2, 2));

    // Part 1: Best Search Time Comparison
    let mut chart_best_time = ChartBuilder::on(&areas[0])
        .caption("Best Route Search Time", ("sans-serif", 22).into_font())
        .margin(10)
        .x_label_area_size(40)
        .y_label_area_size(60)
        .build_cartesian_2d(0f64..2f64, 0.0..limited_best_time_ms * 1.5)?;

    chart_best_time
        .configure_mesh()
        .disable_x_mesh()
        .x_desc("Supply Type")
        .y_desc("Time to Find Best Route (ms)")
        .axis_desc_style(("sans-serif", 16))
        .label_style(("sans-serif", 14))
        .draw()?;

    // Draw best time comparison bars
    chart_best_time.draw_series(
        vec![(0f64, limited_best_time_ms), (1f64, infinite_best_time_ms)]
            .iter()
            .map(|&(x, y)| {
                let color = if x == 0f64 { RED } else { BLUE };
                Rectangle::new([(x, 0.0), (x + 0.8, y)], color.mix(0.6).filled())
            }),
    )?;

    // Add best time text labels
    chart_best_time.draw_series(
        vec![
            (
                0f64,
                limited_best_time_ms,
                format!("{} ms", limited_best_time_ms as i32),
            ),
            (
                1f64,
                infinite_best_time_ms,
                format!("{} ms", infinite_best_time_ms as i32),
            ),
        ]
        .iter()
        .map(|&(x, y, ref label)| {
            Text::new(
                label.clone(),
                (x + 0.4, y + 5.0),
                ("sans-serif", 16).into_font(),
            )
        }),
    )?;

    // Add supply type labels
    chart_best_time.draw_series(vec![(0f64, "Limited"), (1f64, "Infinite")].iter().map(
        |&(x, label)| {
            Text::new(
                label,
                (x + 0.4, -limited_best_time_ms * 0.1),
                ("sans-serif", 16).into_font(),
            )
        },
    ))?;

    // Part 2: Extra Search Time Comparison
    let limited_extra_time = limited_total_time_ms - limited_best_time_ms;
    let infinite_extra_time = infinite_total_time_ms - infinite_best_time_ms;
    let max_extra_time = limited_extra_time.max(infinite_extra_time) * 1.2;

    let mut chart_extra_time = ChartBuilder::on(&areas[1])
        .caption("Additional Search Time", ("sans-serif", 22).into_font())
        .margin(10)
        .x_label_area_size(40)
        .y_label_area_size(60)
        .build_cartesian_2d(0f64..2f64, 0.0..max_extra_time)?;

    chart_extra_time
        .configure_mesh()
        .disable_x_mesh()
        .x_desc("Supply Type")
        .y_desc("Time After Best Route (ms)")
        .axis_desc_style(("sans-serif", 16))
        .label_style(("sans-serif", 14))
        .draw()?;

    // Draw extra time comparison bars
    chart_extra_time.draw_series(
        vec![(0f64, limited_extra_time), (1f64, infinite_extra_time)]
            .iter()
            .map(|&(x, y)| {
                let color = if x == 0f64 { RED } else { BLUE };
                Rectangle::new([(x, 0.0), (x + 0.8, y)], color.mix(0.6).filled())
            }),
    )?;

    // Add extra time text labels
    chart_extra_time.draw_series(
        vec![
            (
                0f64,
                limited_extra_time,
                format!("{} ms", limited_extra_time as i32),
            ),
            (
                1f64,
                infinite_extra_time,
                format!("{} ms", infinite_extra_time as i32),
            ),
        ]
        .iter()
        .map(|&(x, y, ref label)| {
            Text::new(
                label.clone(),
                (x + 0.4, y + 5.0),
                ("sans-serif", 16).into_font(),
            )
        }),
    )?;

    // Add supply type labels
    chart_extra_time.draw_series(vec![(0f64, "Limited"), (1f64, "Infinite")].iter().map(
        |&(x, label)| {
            Text::new(
                label,
                (x + 0.4, -max_extra_time * 0.1),
                ("sans-serif", 16).into_font(),
            )
        },
    ))?;

    // Part 3: Total Time Comparison (stacked bar)
    let max_total_time = limited_total_time_ms.max(infinite_total_time_ms) * 1.2;

    let mut chart_total_time = ChartBuilder::on(&areas[2])
        .caption("Total Search Time", ("sans-serif", 22).into_font())
        .margin(10)
        .x_label_area_size(40)
        .y_label_area_size(60)
        .build_cartesian_2d(0f64..2f64, 0.0..max_total_time)?;

    chart_total_time
        .configure_mesh()
        .disable_x_mesh()
        .x_desc("Supply Type")
        .y_desc("Total Search Time (ms)")
        .axis_desc_style(("sans-serif", 16))
        .label_style(("sans-serif", 14))
        .draw()?;

    // Draw stacked bars for best time
    chart_total_time.draw_series(
        vec![(0f64, limited_best_time_ms), (1f64, infinite_best_time_ms)]
            .iter()
            .map(|&(x, y)| Rectangle::new([(x, 0.0), (x + 0.8, y)], CYAN.mix(0.7).filled())),
    )?;

    // Draw stacked bars for extra time
    chart_total_time.draw_series(
        vec![
            (0f64, limited_best_time_ms, limited_extra_time),
            (1f64, infinite_best_time_ms, infinite_extra_time),
        ]
        .iter()
        .map(|&(x, base_y, height)| {
            Rectangle::new(
                [(x, base_y), (x + 0.8, base_y + height)],
                MAGENTA.mix(0.7).filled(),
            )
        }),
    )?;

    // Add total time text labels
    chart_total_time.draw_series(
        vec![
            (
                0f64,
                limited_total_time_ms,
                format!("Total: {} ms", limited_total_time_ms as i32),
            ),
            (
                1f64,
                infinite_total_time_ms,
                format!("Total: {} ms", infinite_total_time_ms as i32),
            ),
        ]
        .iter()
        .map(|&(x, y, ref label)| {
            Text::new(
                label.clone(),
                (x + 0.4, y + 5.0),
                ("sans-serif", 16).into_font(),
            )
        }),
    )?;

    // Add supply type labels
    chart_total_time.draw_series(vec![(0f64, "Limited"), (1f64, "Infinite")].iter().map(
        |&(x, label)| {
            Text::new(
                label,
                (x + 0.4, -max_total_time * 0.1),
                ("sans-serif", 16).into_font(),
            )
        },
    ))?;

    // Legend for stacked bars
    chart_total_time.draw_series(std::iter::once(Rectangle::new(
        [(0.2, max_total_time * 0.8), (0.4, max_total_time * 0.85)],
        CYAN.mix(0.7).filled(),
    )))?;
    chart_total_time.draw_series(std::iter::once(Text::new(
        "Best Route Search Time",
        (0.45, max_total_time * 0.825),
        ("sans-serif", 14).into_font(),
    )))?;

    chart_total_time.draw_series(std::iter::once(Rectangle::new(
        [(0.2, max_total_time * 0.7), (0.4, max_total_time * 0.75)],
        MAGENTA.mix(0.7).filled(),
    )))?;
    chart_total_time.draw_series(std::iter::once(Text::new(
        "Additional Search Time",
        (0.45, max_total_time * 0.725),
        ("sans-serif", 14).into_font(),
    )))?;

    // Part 4: Route Count Comparison
    let limited_count = limited_routes.len();
    let infinite_count = infinite_routes.len();
    let max_count = std::cmp::max(limited_count, infinite_count) as f64 * 1.2;

    let mut chart_count = ChartBuilder::on(&areas[3])
        .caption("Found Routes Comparison", ("sans-serif", 22).into_font())
        .margin(10)
        .x_label_area_size(40)
        .y_label_area_size(50)
        .build_cartesian_2d(0f64..2f64, 0.0..max_count)?;

    chart_count
        .configure_mesh()
        .disable_x_mesh()
        .x_desc("Supply Type")
        .y_desc("Number of Routes")
        .axis_desc_style(("sans-serif", 16))
        .label_style(("sans-serif", 14))
        .draw()?;

    // Draw route count comparison bars
    chart_count.draw_series(
        vec![(0f64, limited_count as f64), (1f64, infinite_count as f64)]
            .iter()
            .map(|&(x, y)| {
                let color = if x == 0f64 { RED } else { BLUE };
                Rectangle::new([(x, 0.0), (x + 0.8, y)], color.mix(0.6).filled())
            }),
    )?;

    // Add labels for the bars
    chart_count.draw_series(
        vec![
            (
                0f64,
                limited_count as f64,
                format!("{} routes", limited_count),
            ),
            (
                1f64,
                infinite_count as f64,
                format!("{} routes", infinite_count),
            ),
        ]
        .iter()
        .map(|&(x, y, ref label)| {
            Text::new(
                label.clone(),
                (x + 0.4, y + 0.5),
                ("sans-serif", 16).into_font(),
            )
        }),
    )?;

    // Add supply type labels
    chart_count.draw_series(vec![(0f64, "Limited"), (1f64, "Infinite")].iter().map(
        |&(x, label)| {
            Text::new(
                label,
                (x + 0.4, -max_count * 0.1),
                ("sans-serif", 16).into_font(),
            )
        },
    ))?;

    root.present()?;
    Ok(())
}

/// Determine the visualization chart boundaries
fn determine_bounds(
    store_locations: &HashMap<StoreId, (f64, f64)>,
    shopper_start: &Location,
    customer_location: &Location,
) -> (f64, f64, f64, f64) {
    let mut min_x = shopper_start.x;
    let mut max_x = shopper_start.x;
    let mut min_y = shopper_start.y;
    let mut max_y = shopper_start.y;

    // Consider customer location
    min_x = min_x.min(customer_location.x);
    max_x = max_x.max(customer_location.x);
    min_y = min_y.min(customer_location.y);
    max_y = max_y.max(customer_location.y);

    // Consider all store locations
    for &(x, y) in store_locations.values() {
        min_x = min_x.min(x);
        max_x = max_x.max(x);
        min_y = min_y.min(y);
        max_y = max_y.max(y);
    }

    // Add padding
    let padding_x = (max_x - min_x) * 0.1;
    let padding_y = (max_y - min_y) * 0.1;

    (
        min_x - padding_x,
        max_x + padding_x,
        min_y - padding_y,
        max_y + padding_y,
    )
}
