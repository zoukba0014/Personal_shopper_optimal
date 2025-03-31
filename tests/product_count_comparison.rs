// Integration test for comparing different product counts: 5, 10, and 15 products
use personal_shopper::algorithms::bsl_psd::BSLPSD;
use personal_shopper::models::{Location, ShoppingList, ShoppingRoute};
use personal_shopper::utils::init_map::init_map_with_road_network;
use plotters::prelude::*;
use std::collections::HashMap;
use std::error::Error;
use std::time::{Duration, Instant};

#[test]
fn test_product_count_comparison() -> Result<(), Box<dyn Error>> {
    // Configuration parameters
    let city_code = "AMS"; // City code for Amsterdam
    let output_path_5 = "product_count_5.png"; // Output path for 5 products
    let output_path_10 = "product_count_10.png"; // Output path for 10 products
    let output_path_15 = "product_count_15.png"; // Output path for 15 products
    let output_summary = "product_count_summary.png"; // Output path for summary comparison
    let threshold = 10000;

    println!("=== Testing BSL-PSD with Different Product Counts ===");

    // Define test parameters
    let product_counts = [5u32, 10u32, 15u32]; // Different product counts to test, explicitly typed as u32

    // Initialize data structures to store results
    let mut all_results = Vec::new();
    let mut all_best_times = Vec::new();
    let mut all_total_times = Vec::new();
    let mut all_route_counts = Vec::new();

    // Define fixed locations for shopper and customer (to ensure consistency)
    let shopper_location = Location::new(0.0, 0.0);
    let customer_location = Location::new(10.0, 10.0);

    // Run tests for each product count
    for &product_count in &product_counts {
        println!("\n--- Testing with {} products ---", product_count);

        // Initialize map data for limited supply scenario
        println!("Loading map data for {} products...", product_count);
        let (stores, travel_times) =
            match init_map_with_road_network(city_code, false, product_count) {
                Ok(data) => data,
                Err(e) => {
                    eprintln!("Error loading map data: {}", e);
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

        println!(
            "\nAvailable products (count: {}):",
            available_products.len()
        );
        for (product_id, (name, total_supply)) in &available_products {
            println!(
                "  Product ID: {}, Name: {}, Total supply: {}",
                product_id, name, total_supply
            );
        }

        // Initialize shopping list with all available products
        // Use between 3-5 units of each product
        let mut product_ids: Vec<u32> = available_products.keys().cloned().collect();
        product_ids.sort();

        // Use minimum of available products or requested count
        let count_to_use = std::cmp::min(product_count, product_ids.len() as u32) as usize;

        println!("\nShopping List (using {} products):", count_to_use);
        for i in 0..count_to_use {
            if i < product_ids.len() {
                let product_id = product_ids[i];
                let quantity = (i % 3) as u32 + 3; // 3-5 units per product
                shopping_list.add_item(product_id, quantity);

                let product_info = available_products.get(&product_id);
                if let Some((name, _)) = product_info {
                    println!("  Product {} ({}): {} units", product_id, name, quantity);
                }
            }
        }

        // Initialize BSLPSD algorithm
        let mut bsl_psd = BSLPSD::new_with_travel_times(stores.clone(), travel_times);
        bsl_psd.precompute_data();

        println!(
            "Shopper starting location ({:.1}, {:.1})",
            shopper_location.x, shopper_location.y
        );
        println!(
            "Customer delivery location ({:.1}, {:.1})",
            customer_location.x, customer_location.y
        );

        // Run the algorithm and measure times
        println!(
            "\nStarting route planning with {} products...",
            product_count
        );
        let start_time = Instant::now();
        let (results, best_search_time) = bsl_psd.solve_with_parallel(
            &shopping_list,
            shopper_location,
            customer_location,
            threshold,
        );
        let elapsed = start_time.elapsed();

        println!(
            "Results for {} products (found in {:.2?}, best route in {:.2?}):",
            product_count, elapsed, best_search_time
        );
        println!("------------------------------------------");

        if results.is_empty() {
            println!("No feasible routes found with {} products!", product_count);
        } else {
            // Print summary of routes
            println!("Found {} routes:", results.len());
            for (i, route) in results.iter().enumerate().take(5) {
                // Show only top 5
                println!("Route {}: {} stores", i + 1, route.stores.len());
                println!("  Shopping Time: {:.2} minutes", route.shopping_time);
                println!("  Shopping Cost: ${:.2}", route.shopping_cost);
            }

            if results.len() > 5 {
                println!("... and {} more routes", results.len() - 5);
            }

            // Store results for this product count
            all_results.push(results.clone());
            all_best_times.push(best_search_time);
            all_total_times.push(elapsed);
            all_route_counts.push(results.len());

            // Create individual performance chart for this product count
            let output_path = match product_count {
                5 => output_path_5,
                10 => output_path_10,
                15 => output_path_15,
                _ => "product_count_other.png", // Fallback name
            };

            create_performance_chart(
                output_path,
                &results,
                elapsed.as_millis() as f64,
                best_search_time.as_millis() as f64,
                product_count as usize,
            )?;

            println!("Performance chart saved to: {}", output_path);
        }
    }

    // Create summary comparison chart if we have results for all product counts
    if all_results.len() == product_counts.len() {
        create_summary_chart(
            output_summary,
            &product_counts
                .iter()
                .map(|&p| p as usize)
                .collect::<Vec<usize>>(),
            &all_route_counts,
            &all_total_times,
            &all_best_times,
        )?;

        println!("\nSummary comparison chart saved to: {}", output_summary);

        // Print final comparison
        println!("\n=== Product Count Comparison Summary ===");
        for i in 0..product_counts.len() {
            println!(
                "{} Products: {} routes in {:.2?} (best route in {:.2?})",
                product_counts[i], all_route_counts[i], all_total_times[i], all_best_times[i]
            );
        }
    }

    Ok(())
}

/// Create a performance chart for a specific product count
fn create_performance_chart(
    output_path: &str,
    routes: &[ShoppingRoute],
    total_time_ms: f64,
    best_time_ms: f64,
    product_count: usize,
) -> Result<(), Box<dyn Error>> {
    // Create root area
    let root = BitMapBackend::new(output_path, (1000, 800)).into_drawing_area();
    root.fill(&WHITE)?;

    // Add title
    let title = format!("Performance with {} Products", product_count);
    let (title_area, content_area) = root.split_vertically(60);

    title_area.fill(&WHITE)?;
    title_area.draw_text(
        &title,
        &TextStyle::from(("sans-serif", 30).into_font()).color(&BLACK),
        (500, 30),
    )?;

    // Split content area into 2x2 grid
    let areas = content_area.split_evenly((2, 2));

    // Calculate extra time
    let extra_time_ms = total_time_ms - best_time_ms;

    // Part 1: Search Times Chart (Best vs. Extra)
    let max_time = total_time_ms * 1.1;

    let mut chart_time = ChartBuilder::on(&areas[0])
        .caption("Search Time Breakdown", ("sans-serif", 22).into_font())
        .margin(10)
        .x_label_area_size(40)
        .y_label_area_size(60)
        .build_cartesian_2d(0f64..3f64, 0.0..max_time)?;

    chart_time
        .configure_mesh()
        .disable_x_mesh()
        .y_desc("Time (ms)")
        .axis_desc_style(("sans-serif", 16))
        .label_style(("sans-serif", 14))
        .draw()?;

    // Draw stacked bar chart
    chart_time.draw_series(std::iter::once(Rectangle::new(
        [(1f64, 0.0), (2f64, best_time_ms)],
        CYAN.mix(0.7).filled(),
    )))?;

    chart_time.draw_series(std::iter::once(Rectangle::new(
        [(1f64, best_time_ms), (2f64, total_time_ms)],
        MAGENTA.mix(0.7).filled(),
    )))?;

    // Add labels
    chart_time.draw_series(std::iter::once(Text::new(
        format!("Best: {:.0} ms", best_time_ms),
        (1.5f64, best_time_ms / 2.0),
        ("sans-serif", 15).into_font(),
    )))?;

    chart_time.draw_series(std::iter::once(Text::new(
        format!("Extra: {:.0} ms", extra_time_ms),
        (1.5f64, best_time_ms + extra_time_ms / 2.0),
        ("sans-serif", 15).into_font(),
    )))?;

    chart_time.draw_series(std::iter::once(Text::new(
        format!("Total: {:.0} ms", total_time_ms),
        (1.5f64, total_time_ms + 10.0),
        ("sans-serif", 16).into_font(),
    )))?;

    // Part 2: Routes Count Chart
    let route_count = routes.len() as f64;
    let mut chart_routes = ChartBuilder::on(&areas[1])
        .caption("Routes Found", ("sans-serif", 22).into_font())
        .margin(10)
        .x_label_area_size(40)
        .y_label_area_size(60)
        .build_cartesian_2d(0f64..3f64, 0.0..route_count * 1.1)?;

    chart_routes
        .configure_mesh()
        .disable_x_mesh()
        .y_desc("Number of Routes")
        .axis_desc_style(("sans-serif", 16))
        .label_style(("sans-serif", 14))
        .draw()?;

    // Draw route count bar
    chart_routes.draw_series(std::iter::once(Rectangle::new(
        [(1f64, 0.0), (2f64, route_count)],
        RED.mix(0.6).filled(),
    )))?;

    // Add label
    chart_routes.draw_series(std::iter::once(Text::new(
        format!("{} routes", route_count as i32),
        (1.5f64, route_count / 2.0),
        ("sans-serif", 18).into_font(),
    )))?;

    // Part 3: Routes Shopping Time Distribution
    let mut times: Vec<f64> = routes.iter().map(|r| r.shopping_time).collect();
    times.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));

    let min_time = *times.first().unwrap_or(&0.0);
    let max_time = *times.last().unwrap_or(&0.0);
    // let time_range = max_time - min_time;

    let mut chart_dist = ChartBuilder::on(&areas[2])
        .caption("Shopping Time Distribution", ("sans-serif", 22).into_font())
        .margin(10)
        .x_label_area_size(40)
        .y_label_area_size(60)
        .build_cartesian_2d(min_time..max_time * 1.05, 0.0..routes.len() as f64 * 1.1)?;

    chart_dist
        .configure_mesh()
        .x_desc("Shopping Time (minutes)")
        .y_desc("Cumulative Routes")
        .axis_desc_style(("sans-serif", 16))
        .label_style(("sans-serif", 14))
        .draw()?;

    // Draw cumulative distribution
    let points: Vec<(f64, f64)> = times
        .iter()
        .enumerate()
        .map(|(i, &t)| (t, (i + 1) as f64))
        .collect();

    chart_dist.draw_series(LineSeries::new(points, BLUE.stroke_width(3)))?;

    // Part 4: Shopping Cost vs Time Scatter Plot
    let min_cost = routes
        .iter()
        .map(|r| r.shopping_cost)
        .fold(f64::MAX, |a, b| a.min(b));
    let max_cost = routes
        .iter()
        .map(|r| r.shopping_cost)
        .fold(f64::MIN, |a, b| a.max(b));

    let mut chart_scatter = ChartBuilder::on(&areas[3])
        .caption("Cost vs Time Trade-off", ("sans-serif", 22).into_font())
        .margin(10)
        .x_label_area_size(40)
        .y_label_area_size(60)
        .build_cartesian_2d(
            min_time..(max_time * 1.05),
            (min_cost - 5.0)..(max_cost + 5.0),
        )?;

    chart_scatter
        .configure_mesh()
        .x_desc("Shopping Time (minutes)")
        .y_desc("Shopping Cost ($)")
        .axis_desc_style(("sans-serif", 16))
        .label_style(("sans-serif", 14))
        .draw()?;

    // Draw scatter points
    chart_scatter.draw_series(routes.iter().map(|route| {
        Circle::new(
            (route.shopping_time, route.shopping_cost),
            5,
            GREEN.mix(0.5).filled(),
        )
    }))?;

    // Draw Pareto frontier
    let mut points: Vec<(f64, f64)> = routes
        .iter()
        .map(|r| (r.shopping_time, r.shopping_cost))
        .collect();

    points.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap_or(std::cmp::Ordering::Equal));

    if points.len() >= 2 {
        chart_scatter.draw_series(LineSeries::new(points, BLACK.mix(0.7).stroke_width(2)))?;
    }

    root.present()?;
    Ok(())
}

/// Create a summary chart comparing all product counts
fn create_summary_chart(
    output_path: &str,
    product_counts: &[usize],
    route_counts: &[usize],
    total_times: &[Duration],
    best_times: &[Duration],
) -> Result<(), Box<dyn Error>> {
    // Create root area
    let root = BitMapBackend::new(output_path, (1200, 800)).into_drawing_area();
    root.fill(&WHITE)?;

    // Add title
    let title = "Product Count Comparison Summary";
    let (title_area, content_area) = root.split_vertically(60);

    title_area.fill(&WHITE)?;
    title_area.draw_text(
        title,
        &TextStyle::from(("sans-serif", 30).into_font()).color(&BLACK),
        (600, 30),
    )?;

    // Split content area into 2x1 grid
    let areas = content_area.split_evenly((2, 1));

    // Calculate max values for scaling
    let max_time_ms = total_times
        .iter()
        .map(|t| t.as_millis() as f64)
        .fold(0.0, |a, b| f64::max(a, b));

    let max_route_count = *route_counts.iter().max().unwrap_or(&1) as f64;

    // Part 1: Search Time Comparison
    let mut chart_time = ChartBuilder::on(&areas[0])
        .caption(
            "Search Time by Product Count",
            ("sans-serif", 25).into_font(),
        )
        .margin(10)
        .x_label_area_size(40)
        .y_label_area_size(60)
        .build_cartesian_2d(
            0f64..(product_counts.len() as f64),
            0.0..(max_time_ms * 1.1),
        )?;

    chart_time
        .configure_mesh()
        .disable_x_mesh()
        .x_labels(product_counts.len())
        .x_label_formatter(&|x| {
            let idx = *x as usize;
            if idx < product_counts.len() {
                format!("{} Products", product_counts[idx])
            } else {
                String::new()
            }
        })
        .y_desc("Time (ms)")
        .axis_desc_style(("sans-serif", 18))
        .label_style(("sans-serif", 15))
        .draw()?;

    // Draw stacked bars for best time
    for i in 0..product_counts.len() {
        let idx = i as f64;
        let best_time_ms = best_times[i].as_millis() as f64;
        let extra_time_ms = total_times[i].as_millis() as f64 - best_time_ms;

        // Best time portion
        chart_time.draw_series(std::iter::once(Rectangle::new(
            [(idx, 0.0), (idx + 0.8, best_time_ms)],
            CYAN.mix(0.7).filled(),
        )))?;

        // Extra time portion
        chart_time.draw_series(std::iter::once(Rectangle::new(
            [
                (idx, best_time_ms),
                (idx + 0.8, best_time_ms + extra_time_ms),
            ],
            MAGENTA.mix(0.7).filled(),
        )))?;

        // Add labels
        chart_time.draw_series(std::iter::once(Text::new(
            format!("Best: {:.0}ms", best_time_ms),
            (idx + 0.4, best_time_ms / 2.0),
            ("sans-serif", 15).into_font(),
        )))?;

        if extra_time_ms > max_time_ms * 0.05 {
            chart_time.draw_series(std::iter::once(Text::new(
                format!("Extra: {:.0}ms", extra_time_ms),
                (idx + 0.4, best_time_ms + extra_time_ms / 2.0),
                ("sans-serif", 15).into_font(),
            )))?;
        }

        chart_time.draw_series(std::iter::once(Text::new(
            format!("Total: {:.0}ms", best_time_ms + extra_time_ms),
            (idx + 0.4, best_time_ms + extra_time_ms + 50.0),
            ("sans-serif", 14).into_font(),
        )))?;
    }

    // Add legend
    chart_time.draw_series(std::iter::once(Rectangle::new(
        [
            (product_counts.len() as f64 * 0.6, max_time_ms * 0.8),
            (product_counts.len() as f64 * 0.7, max_time_ms * 0.85),
        ],
        CYAN.mix(0.7).filled(),
    )))?;

    chart_time.draw_series(std::iter::once(Text::new(
        "Best Route Time",
        (product_counts.len() as f64 * 0.73, max_time_ms * 0.825),
        ("sans-serif", 14).into_font(),
    )))?;

    chart_time.draw_series(std::iter::once(Rectangle::new(
        [
            (product_counts.len() as f64 * 0.6, max_time_ms * 0.7),
            (product_counts.len() as f64 * 0.7, max_time_ms * 0.75),
        ],
        MAGENTA.mix(0.7).filled(),
    )))?;

    chart_time.draw_series(std::iter::once(Text::new(
        "Additional Time",
        (product_counts.len() as f64 * 0.73, max_time_ms * 0.725),
        ("sans-serif", 14).into_font(),
    )))?;

    // Part 2: Route Count Comparison
    let mut chart_routes = ChartBuilder::on(&areas[1])
        .caption(
            "Routes Found by Product Count",
            ("sans-serif", 25).into_font(),
        )
        .margin(10)
        .x_label_area_size(40)
        .y_label_area_size(50)
        .build_cartesian_2d(
            0f64..(product_counts.len() as f64),
            0.0..(max_route_count * 1.1),
        )?;

    chart_routes
        .configure_mesh()
        .disable_x_mesh()
        .x_labels(product_counts.len())
        .x_label_formatter(&|x| {
            let idx = *x as usize;
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

    // Draw route count bars
    for i in 0..product_counts.len() {
        let idx = i as f64;
        let count = route_counts[i] as f64;

        chart_routes.draw_series(std::iter::once(Rectangle::new(
            [(idx, 0.0), (idx + 0.8, count)],
            RED.mix(0.6).filled(),
        )))?;

        // Add label
        chart_routes.draw_series(std::iter::once(Text::new(
            format!("{}", route_counts[i]),
            (idx + 0.4, count / 2.0),
            ("sans-serif", 18).into_font(),
        )))?;
    }

    root.present()?;
    Ok(())
}
