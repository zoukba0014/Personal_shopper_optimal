// Integration test for analyzing extreme threshold values
// Demonstrates the effect of very low and very high thresholds on BSL-PSD algorithm
use personal_shopper::algorithms::bsl_psd::BSLPSD;
use personal_shopper::models::{Location, ShoppingList};
use personal_shopper::utils::init_map::init_map_with_road_network;
use plotters::prelude::*;
use std::collections::HashMap;
use std::error::Error;
use std::time::Instant;

#[test]
fn test_extreme_threshold_analysis() -> Result<(), Box<dyn Error>> {
    // Configuration parameters
    let city_code = "AMS"; // City code
    let total_product_supply = 5; // Product supply
    let output_path = "extreme_threshold_analysis.png"; // Main output image path
    let time_analysis_output_path = "extreme_threshold_time_analysis.png"; // Time analysis output path

    // Define extreme threshold values to test
    // Very low, low, medium, high, very high
    let thresholds = vec![10, 50, 1000, 20000, 50000];

    println!("Starting extreme threshold performance analysis test");

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

    // Initialize shopping list with available products
    let mut product_ids: Vec<u32> = available_products.keys().cloned().collect();
    product_ids.sort();

    if product_ids.len() >= 5 {
        shopping_list.add_item(product_ids[0], 2);
        shopping_list.add_item(product_ids[1], 4);
        shopping_list.add_item(product_ids[2], 4);
        shopping_list.add_item(product_ids[3], 3);
        shopping_list.add_item(product_ids[4], 4);
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

    // Define start and end points (shopper and customer locations)
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

    // Collect results for each threshold
    let mut results = Vec::new();
    let mut time_results = Vec::new();

    for &threshold in &thresholds {
        println!("\nTesting with extreme threshold: {}", threshold);

        let start_time = Instant::now();
        let (routes, best_route_search_time) = bsl_psd.solve_with_parallel(
            &shopping_list,
            shopper_location,
            customer_location,
            threshold,
        );
        let total_time = start_time.elapsed();
        let search_time = total_time - best_route_search_time;

        println!("Routes found: {}", routes.len());
        println!("Best route search time: {:.2?}", best_route_search_time);
        println!("Total algorithm time: {:.2?}", total_time);
        println!("Search time (excluding best route): {:.2?}", search_time);

        if !routes.is_empty() {
            // Store results for visualization
            results.push((threshold, routes.len()));
            time_results.push((
                threshold,
                best_route_search_time.as_secs_f64(),
                search_time.as_secs_f64(),
                total_time.as_secs_f64(),
            ));
        }
    }

    if results.is_empty() {
        println!("No feasible routes found for any threshold!");
        return Ok(());
    }

    // Create performance visualization with logarithmic scale
    visualize_route_counts(output_path, &results)?;
    println!("Performance visualization saved to: {}", output_path);

    // Create time analysis visualization with logarithmic scale
    visualize_time_analysis(time_analysis_output_path, &time_results)?;
    println!(
        "Time analysis visualization saved to: {}",
        time_analysis_output_path
    );

    // Calculate efficiency ratio (routes per second)
    println!("\nEfficiency Analysis (Routes per Second):");
    for &(threshold, routes) in &results {
        let time_data = time_results
            .iter()
            .find(|&&(t, _, _, _)| t == threshold)
            .unwrap();
        let total_time = time_data.3;
        let efficiency = routes as f64 / total_time;
        println!("Threshold {}: {:.2} routes/second", threshold, efficiency);
    }

    Ok(())
}

/// Visualize the number of routes found for each threshold with logarithmic scale
fn visualize_route_counts(
    output_path: &str,
    results: &[(i32, usize)],
) -> Result<(), Box<dyn Error>> {
    // Create root area
    let root = BitMapBackend::new(output_path, (800, 600)).into_drawing_area();
    root.fill(&WHITE)?;

    // Find min/max values for axis scaling
    let min_threshold = results.iter().map(|&(t, _)| t).min().unwrap_or(0);
    let max_threshold = results.iter().map(|&(t, _)| t).max().unwrap_or(0);

    let max_routes = results.iter().map(|&(_, r)| r).max().unwrap_or(0);
    let route_padding = max_routes as f64 * 0.1;

    // Create chart with logarithmic x-axis
    let mut chart = ChartBuilder::on(&root)
        .caption(
            "Effect of Extreme Threshold Values on Routes Found",
            ("sans-serif", 22).into_font(),
        )
        .margin(10)
        .x_label_area_size(40)
        .y_label_area_size(40)
        .build_cartesian_2d(
            (min_threshold as f64)..(max_threshold as f64 * 1.1),
            0.0..(max_routes as f64 + route_padding),
        )?;

    chart
        .configure_mesh()
        .x_desc("Threshold Value (note: not to scale)")
        .y_desc("Number of Routes")
        .x_labels(5)
        .draw()?;

    // Draw data points and connecting line
    chart.draw_series(LineSeries::new(
        results.iter().map(|&(t, r)| (t as f64, r as f64)),
        BLUE.mix(0.8).stroke_width(3),
    ))?;

    chart.draw_series(
        results
            .iter()
            .map(|&(t, r)| Circle::new((t as f64, r as f64), 5, RED.filled())),
    )?;

    // Draw data labels
    for &(threshold, routes) in results {
        // Add threshold value label
        chart.draw_series(std::iter::once(Text::new(
            format!("t={}", threshold),
            (threshold as f64, routes as f64 * 0.9),
            ("sans-serif", 12).into_font(),
        )))?;

        // Add route count label
        chart.draw_series(std::iter::once(Text::new(
            format!("{} routes", routes),
            (threshold as f64, routes as f64 + (route_padding * 0.2)),
            ("sans-serif", 15).into_font(),
        )))?;
    }

    root.present()?;
    Ok(())
}

/// Visualize the time analysis for each threshold with logarithmic scale
fn visualize_time_analysis(
    output_path: &str,
    results: &[(i32, f64, f64, f64)],
) -> Result<(), Box<dyn Error>> {
    // Create root area
    let root = BitMapBackend::new(output_path, (800, 600)).into_drawing_area();
    root.fill(&WHITE)?;

    // Find min/max values for axis scaling
    let min_threshold = results.iter().map(|&(t, _, _, _)| t).min().unwrap_or(0);
    let max_threshold = results.iter().map(|&(t, _, _, _)| t).max().unwrap_or(0);

    let max_time = results
        .iter()
        .map(|&(_, best, search, total)| best.max(search).max(total))
        .max_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal))
        .unwrap_or(0.0);
    let time_padding = max_time * 0.1;

    // Create chart with logarithmic x-axis
    let mut chart = ChartBuilder::on(&root)
        .caption(
            "Effect of Extreme Threshold Values on Performance Time",
            ("sans-serif", 22).into_font(),
        )
        .margin(10)
        .x_label_area_size(40)
        .y_label_area_size(60)
        .build_cartesian_2d(
            (min_threshold as f64)..(max_threshold as f64 * 1.1),
            0.0..(max_time + time_padding),
        )?;

    chart
        .configure_mesh()
        .x_desc("Threshold Value (note: not to scale)")
        .y_desc("Time (seconds)")
        .y_labels(10)
        .x_labels(5)
        .draw()?;

    // Draw best route search time
    chart
        .draw_series(LineSeries::new(
            results.iter().map(|&(t, best, _, _)| (t as f64, best)),
            GREEN.mix(0.8).stroke_width(3),
        ))?
        .label("Best Route Search Time")
        .legend(|(x, y)| {
            PathElement::new(vec![(x, y), (x + 20, y)], GREEN.mix(0.8).stroke_width(3))
        });

    // Draw search time (excluding best route)
    chart
        .draw_series(LineSeries::new(
            results.iter().map(|&(t, _, search, _)| (t as f64, search)),
            BLUE.mix(0.8).stroke_width(3),
        ))?
        .label("Search Time (excl. Best Route)")
        .legend(|(x, y)| {
            PathElement::new(vec![(x, y), (x + 20, y)], BLUE.mix(0.8).stroke_width(3))
        });

    // Draw total time
    chart
        .draw_series(LineSeries::new(
            results.iter().map(|&(t, _, _, total)| (t as f64, total)),
            RED.mix(0.8).stroke_width(3),
        ))?
        .label("Total Algorithm Time")
        .legend(|(x, y)| PathElement::new(vec![(x, y), (x + 20, y)], RED.mix(0.8).stroke_width(3)));

    // Draw data points
    chart.draw_series(
        results
            .iter()
            .map(|&(t, best, _, _)| Circle::new((t as f64, best), 4, GREEN.filled())),
    )?;

    chart.draw_series(
        results
            .iter()
            .map(|&(t, _, search, _)| Circle::new((t as f64, search), 4, BLUE.filled())),
    )?;

    chart.draw_series(
        results
            .iter()
            .map(|&(t, _, _, total)| Circle::new((t as f64, total), 4, RED.filled())),
    )?;

    // Add labels for thresholds
    for &(threshold, _, _, _) in results {
        chart.draw_series(std::iter::once(Text::new(
            format!("{}", threshold),
            (threshold as f64, max_time * 0.05),
            ("sans-serif", 12).into_font(),
        )))?;
    }

    // Add legend
    chart
        .configure_series_labels()
        .background_style(&WHITE.mix(0.8))
        .border_style(&BLACK)
        .position(SeriesLabelPosition::UpperRight)
        .draw()?;

    root.present()?;
    Ok(())
}
