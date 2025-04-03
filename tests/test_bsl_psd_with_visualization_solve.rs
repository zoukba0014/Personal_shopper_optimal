// Integration test for BSL-PSD algorithm
use personal_shopper::algorithms::bsl_psd::BSLPSD;
use personal_shopper::models::{Location, ShoppingList, ShoppingRoute, Store, StoreId};
use personal_shopper::utils::init_map::init_map_with_road_network;
use plotters::prelude::*;
use rand::Rng;
use std::collections::HashMap;
use std::error::Error;

#[test]
fn test_bsl_psd_with_visualization() -> Result<(), Box<dyn Error>> {
    // Configuration parameters
    let city_code = "AMS"; // City code
    let total_product_supply = 30; // Product supply
    let parallel_output_path = "bsl_psd_routes_parallel.png"; // Output image path for parallel
    let debug_output_path = "bsl_psd_routes_debug.png"; // Output image path for debug
    let threshold = 50000;

    // Initialize map data
    println!("Loading map data...");
    let (stores, travel_times) =
        match init_map_with_road_network(city_code, false, total_product_supply) {
            Ok(data) => data,
            Err(e) => {
                eprintln!("Error loading map data: {}", e);
                eprintln!(
                    "Ensure data files are in the 'data/' directory and have the correct format"
                );
                return Err(e.into());
            }
        };

    // Create shopping list
    let mut shopping_list = ShoppingList::new();

    // Find available products
    let mut available_products = HashMap::new();
    for (_store_id, store) in &stores {
        for (product_id, product) in &store.products {
            let entry = available_products
                .entry(*product_id)
                .or_insert((product.name.clone(), 0));
            entry.1 += store.get_inventory_level(product_id);
        }
    }

    println!("\nAvailable products:");
    for (product_id, (name, total_supply)) in &available_products {
        println!(
            "  Product ID: {}, Name: {}, Total supply: {}",
            product_id, name, total_supply
        );
    }

    // Initialize shopping list
    let mut product_ids: Vec<u32> = available_products.keys().cloned().collect();
    product_ids.sort();

    if product_ids.len() >= 5 {
        let mut rng = rand::thread_rng();

        let quantity1 = rng.gen_range(5..=10);
        let quantity2 = rng.gen_range(5..=10);
        let quantity3 = rng.gen_range(5..=10);
        let quantity4 = rng.gen_range(5..=10);
        let quantity5 = rng.gen_range(5..=10);
        let quantity6 = rng.gen_range(5..=10);
        let quantity7 = rng.gen_range(5..=10);
        let quantity8 = rng.gen_range(5..=10);

        shopping_list.add_item(product_ids[0], quantity1);
        shopping_list.add_item(product_ids[1], quantity2);
        shopping_list.add_item(product_ids[2], quantity3);
        shopping_list.add_item(product_ids[3], quantity4);
        shopping_list.add_item(product_ids[4], quantity5);
        shopping_list.add_item(product_ids[5], quantity6);
        shopping_list.add_item(product_ids[6], quantity7);
        shopping_list.add_item(product_ids[7], quantity8);
    }

    println!("\nShopping List:");
    for (product_id, quantity) in &shopping_list.items {
        let product_info = available_products.get(product_id);
        if let Some((name, _)) = product_info {
            println!("  Product {} ({}): {} units", product_id, name, quantity);
        }
    }

    // Initialize BSLPSD algorithm
    let mut bsl_psd = BSLPSD::new_with_travel_times(stores.clone(), travel_times);
    bsl_psd.precompute_data();

    // Define start and end points (also make them more spread out)
    let shopper_location = Location::new(4.8950, 52.3664); // 阿姆斯特丹市中心餐厅密集区
    let customer_location = Location::new(4.8730, 52.3383); // 阿姆斯特丹市中心偏南住宅区

    println!(
        "Shopper starting location ({:.4}, {:.4})",
        shopper_location.x, shopper_location.y
    );
    println!(
        "Customer delivery location ({:.4}, {:.4})",
        customer_location.x, customer_location.y
    );

    // PART 1: Generate visualization using solve_with_parallel
    println!("Starting route planning with parallel solver...");
    let start_time_parallel = std::time::Instant::now();
    let (parallel_results, _best_searching_time) = bsl_psd.solve_with_parallel(
        &shopping_list,
        shopper_location,
        customer_location,
        threshold,
    );
    let elapsed_parallel = start_time_parallel.elapsed();

    println!(
        "Parallel Linear Skyline Results (found in {:.2?}):",
        elapsed_parallel
    );
    println!("------------------------------------------");

    if parallel_results.is_empty() {
        println!("No feasible routes found with parallel solver!");
    } else {
        // Use actual store locations for the parallel results
        let parallel_store_locations = generate_store_locations(&parallel_results, &stores);

        // Print each route's information for parallel
        for (i, route) in parallel_results.iter().enumerate() {
            println!("Parallel Route {}: {:?}", i + 1, route.stores);
            println!("  Shopping Time: {:.2} minutes", route.shopping_time);
            println!("  Shopping Cost: ${:.2}", route.shopping_cost);
        }

        // Visualize all routes from parallel solver
        visualize_all_routes(
            parallel_output_path,
            &parallel_results,
            &parallel_store_locations,
            &shopper_location,
            &customer_location,
            "BSL-PSD Shopping Routes (Parallel Solver)",
        )?;

        // Also visualize each individual route
        visualize_individual_routes(
            parallel_output_path,
            &parallel_results,
            &parallel_store_locations,
            &shopper_location,
            &customer_location,
            "BSL-PSD Shopping Route (Parallel Solver)",
            &stores,
            &shopping_list,
        )?;

        println!(
            "Parallel visualization complete. Output saved to: {}",
            parallel_output_path
        );
    }

    // PART 2: Generate visualization using solve_with_debug
    println!("\nStarting route planning with debug solver...");
    let start_time_debug = std::time::Instant::now();
    let debug_results = bsl_psd.solve_with_debug(
        &shopping_list,
        shopper_location,
        customer_location,
        threshold,
    );
    let elapsed_debug = start_time_debug.elapsed();

    println!(
        "Debug Linear Skyline Results (found in {:.2?}):",
        elapsed_debug
    );
    println!("------------------------------------------");

    if debug_results.is_empty() {
        println!("No feasible routes found with debug solver!");
    } else {
        // Use actual store locations for the debug results
        let debug_store_locations = generate_store_locations(&debug_results, &stores);

        // Print each route's information for debug
        for (i, route) in debug_results.iter().enumerate() {
            println!("Debug Route {}: {:?}", i + 1, route.stores);
            println!("  Shopping Time: {:.2} minutes", route.shopping_time);
            println!("  Shopping Cost: ${:.2}", route.shopping_cost);
        }

        // Visualize all routes from debug solver
        visualize_all_routes(
            debug_output_path,
            &debug_results,
            &debug_store_locations,
            &shopper_location,
            &customer_location,
            "BSL-PSD Shopping Routes (Debug Solver)",
        )?;

        // Also visualize each individual route
        visualize_individual_routes(
            debug_output_path,
            &debug_results,
            &debug_store_locations,
            &shopper_location,
            &customer_location,
            "BSL-PSD Shopping Route (Debug Solver)",
            &stores,
            &shopping_list,
        )?;

        println!(
            "Debug visualization complete. Output saved to: {}",
            debug_output_path
        );
    }

    // Compare results if both methods found solutions
    if !parallel_results.is_empty() && !debug_results.is_empty() {
        println!("\nComparison between Parallel and Debug solvers:");
        println!(
            "  Parallel solver found {} routes in {:.2?}",
            parallel_results.len(),
            elapsed_parallel
        );
        println!(
            "  Debug solver found {} routes in {:.2?}",
            debug_results.len(),
            elapsed_debug
        );

        if elapsed_parallel < elapsed_debug {
            println!(
                "  Parallel solver was {:.1}% faster",
                (elapsed_debug.as_secs_f64() / elapsed_parallel.as_secs_f64() - 1.0) * 100.0
            );
        } else {
            println!(
                "  Debug solver was {:.1}% faster",
                (elapsed_parallel.as_secs_f64() / elapsed_debug.as_secs_f64() - 1.0) * 100.0
            );
        }
    }

    Ok(())
}

/// Generate store locations from actual store coordinates instead of random positions
fn generate_store_locations(
    routes: &[ShoppingRoute],
    stores_map: &HashMap<StoreId, Store>,
) -> HashMap<StoreId, (f64, f64)> {
    let mut store_locations: HashMap<StoreId, (f64, f64)> = HashMap::new();

    // First, identify all stores used in any route
    let mut used_store_ids = std::collections::HashSet::new();
    for route in routes {
        for store_id in &route.stores {
            used_store_ids.insert(*store_id);
        }
    }

    // Use actual store locations
    for &store_id in &used_store_ids {
        if let Some(store) = stores_map.get(&store_id) {
            // Use actual coordinates from the store
            store_locations.insert(store_id, (store.location.x, store.location.y));
        }
    }

    store_locations
}

// Visualize all shopping routes
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

/// Visualize each route individually in a separate image
fn visualize_individual_routes(
    base_output_path: &str,
    routes: &[ShoppingRoute],
    store_locations: &HashMap<StoreId, (f64, f64)>,
    shopper_start: &Location,
    customer_location: &Location,
    chart_title: &str,
    stores_map: &HashMap<StoreId, Store>,
    shopping_list: &ShoppingList,
) -> Result<(), Box<dyn Error>> {
    // Determine chart boundaries (use the same bounds for all routes for consistency)
    let (min_x, max_x, min_y, max_y) =
        determine_bounds(store_locations, shopper_start, customer_location);

    // Create a directory to store individual route images
    let output_dir = format!(
        "{}_routes",
        base_output_path
            .strip_suffix(".png")
            .unwrap_or(base_output_path)
    );
    std::fs::create_dir_all(&output_dir)?;

    // Define colors
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

    // Generate individual images for each route
    for (i, route) in routes.iter().enumerate() {
        if route.stores.is_empty() {
            continue;
        }

        // Determine the output path for this route
        let route_output_path = format!("{}/route_{}.png", output_dir, i + 1);

        // Create chart for this route
        let root = BitMapBackend::new(&route_output_path, (1000, 800)).into_drawing_area();
        root.fill(&WHITE)?;

        // Set up coordinate system
        let mut chart = ChartBuilder::on(&root)
            .caption(
                format!(
                    "{} - Route {} (Time: {:.1} min, Cost: ${:.2})",
                    chart_title,
                    i + 1,
                    route.shopping_time,
                    route.shopping_cost
                ),
                ("sans-serif", 20).into_font(),
            )
            .margin(10)
            .x_label_area_size(30)
            .y_label_area_size(30)
            .build_cartesian_2d(min_x..max_x, min_y..max_y)?;

        chart.configure_mesh().draw()?;

        // Draw all stores (highlight stores in this route)
        for (store_id, (x, y)) in store_locations {
            // Use a different style for stores in this route vs. other stores
            let in_route = route.stores.contains(store_id);
            let style = if in_route {
                ShapeStyle::from(&RGBColor(0, 100, 0)).filled() // Dark green for stores in route
            } else {
                ShapeStyle::from(&RGBColor(200, 200, 200)).filled() // Light gray for other stores
            };

            let size = if in_route { 10 } else { 6 };

            // Get store name if available
            let store_name = if let Some(store) = stores_map.get(store_id) {
                format!("Store {}", store_id)
            } else {
                format!("Store {}", store_id)
            };

            chart
                .draw_series(std::iter::once(Circle::new((*x, *y), size, style)))?
                .label(format!(
                    "{}{}",
                    store_name,
                    if in_route { " (visited)" } else { "" }
                ))
                .legend(move |(x, y)| Circle::new((x, y), size, style));
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

        // Draw this route
        let mut path_points = Vec::new();
        path_points.push((shopper_start.x, shopper_start.y));

        // Add labels for route sequence
        for (j, store_id) in route.stores.iter().enumerate() {
            if let Some(&(x, y)) = store_locations.get(store_id) {
                path_points.push((x, y));

                // Create label with store info and products available there
                let store_label = if let Some(store) = stores_map.get(store_id) {
                    let mut products_info = String::new();
                    for (product_id, qty) in &shopping_list.items {
                        if store.has_product(product_id) {
                            let product_name = match &store.products.get(product_id) {
                                Some(product) => &product.name,
                                None => "Unknown",
                            };
                            products_info.push_str(&format!("\n{}: {}", product_name, qty));
                        }
                    }
                    format!("{}. Store {}{}", j + 1, store_id, products_info)
                } else {
                    format!("{}. Store {}", j + 1, store_id)
                };

                chart.draw_series(std::iter::once(Text::new(
                    format!("{}", j + 1),
                    (x, y - 8.0),
                    ("sans-serif", 15).into_font().color(&BLACK),
                )))?;

                // Add tooltip-like annotation with store details
                if let Some(store) = stores_map.get(store_id) {
                    let products_in_store = shopping_list
                        .items
                        .iter()
                        .filter(|(product_id, _)| store.has_product(product_id))
                        .count();

                    if products_in_store > 0 {
                        chart.draw_series(std::iter::once(Text::new(
                            format!("Store {} ({})", store_id, products_in_store),
                            (x, y + 12.0),
                            ("sans-serif", 10).into_font().color(&BLACK),
                        )))?;
                    }
                }
            }
        }

        path_points.push((customer_location.x, customer_location.y));

        let color = colors[i % colors.len()];
        chart
            .draw_series(LineSeries::new(
                path_points.clone(),
                color.mix(0.7).stroke_width(3),
            ))?
            .label(format!(
                "Route {} (Time: {:.1} min, Cost: ${:.2})",
                i + 1,
                route.shopping_time,
                route.shopping_cost
            ))
            .legend(|(x, y)| {
                PathElement::new(vec![(x, y), (x + 20, y)], color.mix(0.7).stroke_width(3))
            });

        // Add arrows to indicate direction
        if path_points.len() >= 2 {
            for i in 0..path_points.len() - 1 {
                let (x1, y1) = path_points[i];
                let (x2, y2) = path_points[i + 1];

                // Calculate the midpoint with a slight offset
                let mid_x = (x1 + x2) / 2.0;
                let mid_y = (y1 + y2) / 2.0;

                // Draw an arrow symbol at the midpoint
                chart.draw_series(std::iter::once(Circle::new(
                    (mid_x, mid_y),
                    5,
                    color.mix(0.9).filled(),
                )))?;
            }
        }

        // Add a summary of the route information
        let route_summary = format!(
            "Route Summary:\n- Stores visited: {}\n- Shopping time: {:.2} min\n- Shopping cost: ${:.2}",
            route.stores.len(),
            route.shopping_time,
            route.shopping_cost
        );

        // Draw route summary as text
        chart.draw_series(std::iter::once(Text::new(
            route_summary,
            (
                min_x + (max_x - min_x) * 0.05,
                max_y - (max_y - min_y) * 0.1,
            ),
            ("sans-serif", 14).into_font(),
        )))?;

        chart
            .configure_series_labels()
            .background_style(&WHITE.mix(0.8))
            .border_style(&BLACK)
            .position(SeriesLabelPosition::UpperLeft)
            .draw()?;

        root.present()?;

        println!(
            "Route {} visualization saved to: {}",
            i + 1,
            route_output_path
        );
    }

    println!(
        "All individual route visualizations saved to directory: {}",
        output_dir
    );
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
