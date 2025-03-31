// Integration test for analyzing how different product counts affect algorithm performance
// with a constant threshold value of 10000
use personal_shopper::algorithms::bsl_psd::BSLPSD;
use personal_shopper::models::{Location, ShoppingList};
use personal_shopper::utils::init_map::init_map_with_road_network;
use plotters::prelude::*;
use std::collections::HashMap;
use std::error::Error;
use std::time::Instant;

#[test]
fn test_product_count_threshold_analysis() -> Result<(), Box<dyn Error>> {
    // Configuration parameters
    let city_code = "AMS"; // City code
    let total_product_supply = 15; // Maximum product supply (must be >= max product count to test)
    let output_path = "product_count_threshold_analysis.png"; // Main output image path
    let time_analysis_output_path = "product_count_time_analysis.png"; // Time analysis output path

    // Fixed threshold value
    let threshold = 10000;

    // Different product counts to test
    let product_counts = vec![3, 5, 10];

    println!(
        "Starting product count analysis test with fixed threshold: {}",
        threshold
    );

    // Initialize map data
    println!("Loading map data...");
    let (stores, travel_times) =
        match init_map_with_road_network(city_code, false, total_product_supply) {
            Ok(data) => data,
            Err(e) => {
                eprintln!("Error loading map data: {}", e);
                return Err(e.into());
            }
        };

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

    // Sort product IDs
    let mut product_ids: Vec<u32> = available_products.keys().cloned().collect();
    product_ids.sort();

    println!(
        "\nAvailable products (total {}): {:?}",
        product_ids.len(),
        product_ids
    );

    // Initialize BSLPSD algorithm
    let mut bsl_psd = BSLPSD::new_with_travel_times(stores.clone(), travel_times);
    bsl_psd.precompute_data();

    // Define standard locations
    let shopper_location = Location::new(-80.0, -80.0);
    let customer_location = Location::new(80.0, 80.0);

    println!(
        "Shopper location: ({:.1}, {:.1})",
        shopper_location.x, shopper_location.y
    );
    println!(
        "Customer location: ({:.1}, {:.1})",
        customer_location.x, customer_location.y
    );

    // Collect results for each product count
    let mut results = Vec::new();
    let mut time_results = Vec::new();

    // Ensure we have enough products available for the largest count
    if product_ids.len() < *product_counts.iter().max().unwrap_or(&3) as usize {
        eprintln!(
            "Not enough available products for testing. Need at least {} products.",
            product_counts.iter().max().unwrap_or(&3)
        );
        return Ok(());
    }

    for &count in &product_counts {
        println!("\nTesting with {} products:", count);

        // Create shopping list with specified number of products
        let mut shopping_list = ShoppingList::new();

        // Add specified number of items
        for i in 0..count as usize {
            if i < product_ids.len() {
                let quantity = if i % 3 == 0 { 2 } else { 4 }; // Mix of quantities
                shopping_list.add_item(product_ids[i], quantity);
                println!("  Added product {}: {} units", product_ids[i], quantity);
            }
        }

        // Run the algorithm
        let start_time = Instant::now();
        let (routes, best_route_search_time) = bsl_psd.solve_with_parallel(
            &shopping_list,
            shopper_location,
            customer_location,
            threshold,
        );
        let total_time = start_time.elapsed();
        let search_time = total_time - best_route_search_time;

        println!("Results for {} products:", count);
        println!("  Routes found: {}", routes.len());
        println!("  Best route search time: {:.2?}", best_route_search_time);
        println!("  Total algorithm time: {:.2?}", total_time);
        println!("  Search time (excluding best route): {:.2?}", search_time);

        if !routes.is_empty() {
            // Store results for visualization
            results.push((count, routes.len()));
            time_results.push((
                count,
                best_route_search_time.as_secs_f64(),
                search_time.as_secs_f64(),
                total_time.as_secs_f64(),
            ));
        }
    }

    if results.is_empty() {
        println!("No feasible routes found for any product count!");
        return Ok(());
    }

    // Create visualizations
    visualize_product_count_routes(output_path, &results)?;
    println!("Routes visualization saved to: {}", output_path);

    visualize_product_count_time(time_analysis_output_path, &time_results)?;
    println!(
        "Time analysis visualization saved to: {}",
        time_analysis_output_path
    );

    // Calculate and print route density (routes per product)
    println!("\nRoute Density Analysis (Routes per Product):");
    for &(count, routes) in &results {
        let density = routes as f64 / count as f64;
        println!("  {} products: {:.2} routes/product", count, density);
    }

    // Calculate and print time efficiency
    println!("\nTime Efficiency Analysis (Processing Time per Product):");
    for &(count, _, _, total_time) in &time_results {
        let time_per_product = total_time / count as f64;
        println!(
            "  {} products: {:.2} seconds/product",
            count, time_per_product
        );
    }

    // Calculate and print route discovery rate
    println!("\nRoute Discovery Rate Analysis (Routes per Second):");
    for i in 0..results.len() {
        let (count, routes) = results[i];
        let (_, _, _, total_time) = time_results[i];
        let discovery_rate = routes as f64 / total_time;
        println!("  {} products: {:.2} routes/second", count, discovery_rate);
    }

    Ok(())
}

/// Visualize the number of routes found for each product count
fn visualize_product_count_routes(
    output_path: &str,
    results: &[(i32, usize)],
) -> Result<(), Box<dyn Error>> {
    // Create root area
    let root = BitMapBackend::new(output_path, (800, 600)).into_drawing_area();
    root.fill(&WHITE)?;

    // Find max values for axis scaling
    let max_product_count = results.iter().map(|&(c, _)| c).max().unwrap_or(0);
    let max_routes = results.iter().map(|&(_, r)| r).max().unwrap_or(0);
    let route_padding = max_routes as f64 * 0.1;

    // Create chart with float ranges
    let mut chart = ChartBuilder::on(&root)
        .caption(
            "Effect of Product Count on Routes Found (Threshold: 10000)",
            ("sans-serif", 22).into_font(),
        )
        .margin(10)
        .x_label_area_size(40)
        .y_label_area_size(60)
        .build_cartesian_2d(
            0.0..((max_product_count + 1) as f64),
            0.0..(max_routes as f64 + route_padding),
        )?;

    chart
        .configure_mesh()
        .x_desc("Number of Products in Shopping List")
        .y_desc("Number of Routes Found")
        .x_labels(results.len() + 1)
        .draw()?;

    // Draw bar chart
    chart.draw_series(results.iter().map(|&(count, routes)| {
        let bar_width = 0.6; // Width of the bar
        let x_start = count as f64 - bar_width / 2.0;
        let x_end = count as f64 + bar_width / 2.0;

        Rectangle::new(
            [(x_start, 0.0), (x_end, routes as f64)],
            BLUE.mix(0.6).filled(),
        )
    }))?;

    // Draw data points for clarity
    chart.draw_series(
        results
            .iter()
            .map(|&(count, routes)| Circle::new((count as f64, routes as f64), 5, RED.filled())),
    )?;

    // Add data labels
    for &(count, routes) in results {
        chart.draw_series(std::iter::once(Text::new(
            format!("{}", routes),
            (count as f64, routes as f64 + (route_padding * 0.2)),
            ("sans-serif", 15).into_font(),
        )))?;
    }

    root.present()?;
    Ok(())
}

/// Visualize the time analysis for each product count
fn visualize_product_count_time(
    output_path: &str,
    results: &[(i32, f64, f64, f64)],
) -> Result<(), Box<dyn Error>> {
    // Create root area
    let root = BitMapBackend::new(output_path, (800, 600)).into_drawing_area();
    root.fill(&WHITE)?;

    // Find max values for axis scaling
    let max_product_count = results.iter().map(|&(c, _, _, _)| c).max().unwrap_or(0);

    let max_time = results
        .iter()
        .map(|&(_, best, search, total)| best.max(search).max(total))
        .max_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal))
        .unwrap_or(0.0);
    let time_padding = max_time * 0.1;

    // Create chart with float ranges
    let mut chart = ChartBuilder::on(&root)
        .caption(
            "Effect of Product Count on Algorithm Time (Threshold: 10000)",
            ("sans-serif", 22).into_font(),
        )
        .margin(10)
        .x_label_area_size(40)
        .y_label_area_size(60)
        .build_cartesian_2d(
            0.0..((max_product_count + 1) as f64),
            0.0..(max_time + time_padding),
        )?;

    chart
        .configure_mesh()
        .x_desc("Number of Products in Shopping List")
        .y_desc("Time (seconds)")
        .x_labels(results.len() + 1)
        .draw()?;

    // Draw lines connecting points
    // Best route search time
    chart
        .draw_series(LineSeries::new(
            results
                .iter()
                .map(|&(count, best, _, _)| (count as f64, best)),
            GREEN.mix(0.8).stroke_width(3),
        ))?
        .label("Best Route Search Time")
        .legend(|(x, y)| {
            PathElement::new(vec![(x, y), (x + 20, y)], GREEN.mix(0.8).stroke_width(3))
        });

    // Search time (excluding best route)
    chart
        .draw_series(LineSeries::new(
            results
                .iter()
                .map(|&(count, _, search, _)| (count as f64, search)),
            BLUE.mix(0.8).stroke_width(3),
        ))?
        .label("Search Time (excl. Best Route)")
        .legend(|(x, y)| {
            PathElement::new(vec![(x, y), (x + 20, y)], BLUE.mix(0.8).stroke_width(3))
        });

    // Total time
    chart
        .draw_series(LineSeries::new(
            results
                .iter()
                .map(|&(count, _, _, total)| (count as f64, total)),
            RED.mix(0.8).stroke_width(3),
        ))?
        .label("Total Algorithm Time")
        .legend(|(x, y)| PathElement::new(vec![(x, y), (x + 20, y)], RED.mix(0.8).stroke_width(3)));

    // Draw data points
    chart.draw_series(
        results
            .iter()
            .map(|&(count, best, _, _)| Circle::new((count as f64, best), 4, GREEN.filled())),
    )?;

    chart.draw_series(
        results
            .iter()
            .map(|&(count, _, search, _)| Circle::new((count as f64, search), 4, BLUE.filled())),
    )?;

    chart.draw_series(
        results
            .iter()
            .map(|&(count, _, _, total)| Circle::new((count as f64, total), 4, RED.filled())),
    )?;

    // Add legend
    chart
        .configure_series_labels()
        .background_style(&WHITE.mix(0.8))
        .border_style(&BLACK)
        .position(SeriesLabelPosition::UpperLeft)
        .draw()?;

    root.present()?;
    Ok(())
}
