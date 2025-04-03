// Integration test for analyzing how different product counts affect algorithm performance
// with a constant threshold value of 10000
use personal_shopper::algorithms::bsl_psd::BSLPSD;
use personal_shopper::models::{Location, ShoppingList};
use personal_shopper::utils::init_map::init_map_with_road_network;
use plotters::prelude::*;
use rand::Rng;
use std::collections::HashMap;
use std::error::Error;
use std::time::Instant;

#[test]
fn test_product_count_threshold_analysis() -> Result<(), Box<dyn Error>> {
    // Configuration parameters
    let city_code = "AMS"; // City code
    let total_product_supply = 30; // Maximum product supply (must be >= max product count to test)
    let output_path = "product_count_threshold_analysis.png"; // Main output image path
    let time_analysis_output_path = "product_count_time_analysis.png"; // Time analysis output path

    // Fixed threshold value
    let threshold = 50000;
    // let total_product_count = 30u32;

    // Different product counts to test
    let product_counts = vec![5, 10, 15];

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
    let shopper_location = Location::new(4.8950, 52.3664); // 阿姆斯特丹市中心餐厅密集区
    let customer_location = Location::new(4.8730, 52.3383); // 阿姆斯特丹市中心偏南住宅区

    println!(
        "Shopper location: ({:.4}, {:.4})",
        shopper_location.x, shopper_location.y
    );
    println!(
        "Customer location: ({:.4}, {:.4})",
        customer_location.x, customer_location.y
    );

    // Collect results for each product count
    let mut results = Vec::new();
    let mut time_results = Vec::new();
    let mut trade_off_metrics = HashMap::new();

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
                let mut rng = rand::thread_rng();
                let quantity = rng.gen_range(5..=10);
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

            // Add route quality trade-off analysis
            println!("\nRoute Quality Trade-off Analysis for {} products:", count);
            if routes.len() >= 2 {
                let fastest = &routes.first().unwrap();
                let cheapest = &routes.last().unwrap();

                println!(
                    "  Fastest route: {:.2} minutes, ${:.2}, {} stores",
                    fastest.shopping_time,
                    fastest.shopping_cost,
                    fastest.stores.len()
                );
                println!(
                    "  Cheapest route: {:.2} minutes, ${:.2}, {} stores",
                    cheapest.shopping_time,
                    cheapest.shopping_cost,
                    cheapest.stores.len()
                );

                // Calculate and display trade-off percentages
                let time_diff_percent = 100.0 * (cheapest.shopping_time - fastest.shopping_time)
                    / cheapest.shopping_time;
                let cost_diff_percent = 100.0 * (fastest.shopping_cost - cheapest.shopping_cost)
                    / cheapest.shopping_cost;

                println!("  Trade-off: Fastest route is {:.1}% faster but {:.1}% more expensive than the cheapest route.",
                         time_diff_percent, cost_diff_percent);

                // Calculate trade-off efficiency (time saved per extra dollar spent)
                let time_saved = cheapest.shopping_time - fastest.shopping_time;
                let extra_cost = fastest.shopping_cost - cheapest.shopping_cost;
                let efficiency = if extra_cost > 0.0 {
                    time_saved / extra_cost
                } else {
                    0.0
                };

                println!(
                    "  Trade-off Efficiency: {:.2} minutes saved per extra dollar spent",
                    efficiency
                );

                // Calculate route diversity metrics for this product count
                let route_count = routes.len();
                let time_range = cheapest.shopping_time - fastest.shopping_time;
                let cost_range = fastest.shopping_cost - cheapest.shopping_cost;

                println!("  Route Diversity Metrics:");
                println!("    - Number of Pareto-optimal routes: {}", route_count);
                println!("    - Time range covered: {:.2} minutes", time_range);
                println!("    - Cost range covered: ${:.2}", cost_range);
                println!(
                    "    - Average time step between routes: {:.2} minutes",
                    if route_count > 1 {
                        time_range / (route_count - 1) as f64
                    } else {
                        0.0
                    }
                );

                // Store trade-off metrics for comparison across product counts
                trade_off_metrics.entry(count).or_insert((
                    time_diff_percent,
                    cost_diff_percent,
                    efficiency,
                    route_count,
                ));
            } else {
                println!("  Only one route found, no trade-off analysis possible.");
            }
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

    // Create trade-off visualization
    if !trade_off_metrics.is_empty() {
        let trade_off_output_path = "product_count_trade_off_analysis.png";

        // Convert trade-off metrics to visualization format
        let mut trade_off_results: Vec<(i32, f64, f64, f64, usize)> = Vec::new();
        let mut sorted_counts: Vec<&i32> = trade_off_metrics.keys().collect();
        sorted_counts.sort();

        for &count in &sorted_counts {
            let (time_diff, cost_diff, efficiency, routes) = trade_off_metrics[count];
            trade_off_results.push((*count, efficiency, time_diff, cost_diff, routes));
        }

        visualize_product_count_trade_off(&trade_off_results, trade_off_output_path)?;
        println!(
            "Trade-off efficiency visualization saved to: {}",
            trade_off_output_path
        );
    }

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

    // Compare trade-off metrics across different product counts
    if trade_off_metrics.len() >= 2 {
        println!("\nTrade-off Metrics Comparison Across Product Counts:");
        println!("--------------------------------------------------------");
        println!("| Product Count | Time-Cost Trade-off | Efficiency (min/$) | Route Count |");
        println!("--------------------------------------------------------");

        // Sort product counts for ordered display
        let mut product_counts: Vec<i32> = trade_off_metrics.keys().cloned().collect();
        product_counts.sort();

        // Print comparison table
        for &product_count in &product_counts {
            let (time_diff, cost_diff, efficiency, route_count) = trade_off_metrics[&product_count];
            println!(
                "| {:<13} | {:.1}% faster, {:.1}% costlier | {:.2} min/$ | {:<11} |",
                product_count, time_diff, cost_diff, efficiency, route_count
            );
        }
        println!("--------------------------------------------------------");

        // Analyze trend in trade-off efficiency
        if product_counts.len() >= 2 {
            let min_count = *product_counts.first().unwrap();
            let max_count = *product_counts.last().unwrap();

            let min_efficiency = trade_off_metrics[&min_count].2;
            let max_efficiency = trade_off_metrics[&max_count].2;

            println!("\nTrade-off Efficiency Trend Analysis:");
            if max_efficiency > min_efficiency {
                let increase = (max_efficiency - min_efficiency) / min_efficiency * 100.0;
                println!("  As product count increases from {} to {}, trade-off efficiency increases by {:.1}%",
                         min_count, max_count, increase);
                println!("  This suggests that with more products, time savings relative to cost increase becomes more favorable");
            } else if min_efficiency > max_efficiency {
                let decrease = (min_efficiency - max_efficiency) / min_efficiency * 100.0;
                println!("  As product count increases from {} to {}, trade-off efficiency decreases by {:.1}%",
                         min_count, max_count, decrease);
                println!("  This suggests that with more products, time savings relative to cost increase becomes less favorable");
            } else {
                println!(
                    "  Trade-off efficiency remains consistent across different product counts"
                );
            }
        }
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

/// Visualize trade-off metrics for different product counts
fn visualize_product_count_trade_off(
    results: &[(i32, f64, f64, f64, usize)],
    output_path: &str,
) -> Result<(), Box<dyn Error>> {
    // Create root area
    let root = BitMapBackend::new(output_path, (1000, 700)).into_drawing_area();
    root.fill(&WHITE)?;

    // Split into areas for different metrics
    let (top_area, bottom_area) = root.split_vertically(350);
    let (efficiency_area, percent_area) = top_area.split_horizontally(500);

    // Find min/max values for axis scaling
    let min_count = results.iter().map(|&(c, _, _, _, _)| c).min().unwrap_or(0);
    let max_count = results.iter().map(|&(c, _, _, _, _)| c).max().unwrap_or(0);
    let count_padding = 1; // Add padding for x-axis

    // Calculate max values for other metrics
    let max_efficiency = results
        .iter()
        .map(|&(_, e, _, _, _)| e)
        .fold(0.0, |a, b| f64::max(a, b));
    let max_percent = results
        .iter()
        .map(|&(_, _, time_diff, cost_diff, _)| f64::max(time_diff, cost_diff))
        .fold(0.0, |a, b| f64::max(a, b));
    let max_routes = results.iter().map(|&(_, _, _, _, r)| r).max().unwrap_or(0);

    // 1. Draw efficiency chart
    let mut efficiency_chart = ChartBuilder::on(&efficiency_area)
        .caption(
            "Trade-off Efficiency by Product Count",
            ("sans-serif", 22).into_font(),
        )
        .margin(10)
        .x_label_area_size(30)
        .y_label_area_size(50)
        .build_cartesian_2d(
            (min_count as f64 - count_padding as f64)..(max_count as f64 + count_padding as f64),
            0.0..(max_efficiency * 1.2),
        )?;

    efficiency_chart
        .configure_mesh()
        .x_desc("Number of Products")
        .y_desc("Efficiency (minutes saved per dollar)")
        .x_labels(results.len())
        .draw()?;

    // Draw efficiency line and points
    efficiency_chart.draw_series(LineSeries::new(
        results.iter().map(|&(c, e, _, _, _)| (c as f64, e)),
        BLUE.mix(0.8).stroke_width(3),
    ))?;

    efficiency_chart.draw_series(
        results
            .iter()
            .map(|&(c, e, _, _, _)| Circle::new((c as f64, e), 5, RED.filled())),
    )?;

    // Add efficiency labels
    for &(count, efficiency, _, _, _) in results {
        efficiency_chart.draw_series(std::iter::once(Text::new(
            format!("{:.2}", efficiency),
            (count as f64, efficiency + (max_efficiency * 0.05)),
            ("sans-serif", 15).into_font(),
        )))?;
    }

    // 2. Draw percentage chart
    let mut percent_chart = ChartBuilder::on(&percent_area)
        .caption(
            "Trade-off Percentages by Product Count",
            ("sans-serif", 22).into_font(),
        )
        .margin(10)
        .x_label_area_size(30)
        .y_label_area_size(40)
        .build_cartesian_2d(
            (min_count as f64 - count_padding as f64)..(max_count as f64 + count_padding as f64),
            0.0..(max_percent * 1.1),
        )?;

    percent_chart
        .configure_mesh()
        .x_desc("Number of Products")
        .y_desc("Percentage Difference (%)")
        .x_labels(results.len())
        .draw()?;

    // Draw percentage lines and points
    percent_chart
        .draw_series(LineSeries::new(
            results
                .iter()
                .map(|&(c, _, time_diff, _, _)| (c as f64, time_diff)),
            GREEN.mix(0.8).stroke_width(3),
        ))?
        .label("Time Savings %")
        .legend(|(x, y)| {
            PathElement::new(vec![(x, y), (x + 20, y)], GREEN.mix(0.8).stroke_width(3))
        });

    percent_chart
        .draw_series(LineSeries::new(
            results
                .iter()
                .map(|&(c, _, _, cost_diff, _)| (c as f64, cost_diff)),
            RED.mix(0.8).stroke_width(3),
        ))?
        .label("Cost Increase %")
        .legend(|(x, y)| PathElement::new(vec![(x, y), (x + 20, y)], RED.mix(0.8).stroke_width(3)));

    percent_chart.draw_series(
        results
            .iter()
            .map(|&(c, _, time_diff, _, _)| Circle::new((c as f64, time_diff), 5, GREEN.filled())),
    )?;

    percent_chart.draw_series(
        results
            .iter()
            .map(|&(c, _, _, cost_diff, _)| Circle::new((c as f64, cost_diff), 5, RED.filled())),
    )?;

    // Add legend
    percent_chart
        .configure_series_labels()
        .background_style(&WHITE.mix(0.8))
        .border_style(&BLACK)
        .position(SeriesLabelPosition::UpperRight)
        .draw()?;

    // 3. Draw routes and efficiency correlation chart
    let mut route_chart = ChartBuilder::on(&bottom_area)
        .caption(
            "Route Count and Efficiency Correlation",
            ("sans-serif", 22).into_font(),
        )
        .margin(10)
        .x_label_area_size(30)
        .y_label_area_size(40)
        .right_y_label_area_size(40)
        .build_cartesian_2d(
            (min_count as f64 - count_padding as f64)..(max_count as f64 + count_padding as f64),
            0.0..((max_routes as f64) * 1.1),
        )?;

    route_chart
        .configure_mesh()
        .x_desc("Number of Products")
        .y_desc("Number of Routes")
        .x_labels(results.len())
        .draw()?;

    // Draw route count as bars
    for &(count, _, _, _, routes) in results {
        route_chart.draw_series(std::iter::once(Rectangle::new(
            [
                (count as f64 - 0.3, 0.0),
                (count as f64 + 0.3, routes as f64),
            ],
            MAGENTA.mix(0.6).filled(),
        )))?;
    }

    // Draw the number of routes as text labels
    for &(count, _, _, _, routes) in results {
        route_chart.draw_series(std::iter::once(Text::new(
            format!("{}", routes),
            (count as f64, (routes as f64) + ((max_routes as f64) * 0.05)),
            ("sans-serif", 15).into_font(),
        )))?;
    }

    // Configure second y-axis for efficiency (right side)
    let mut efficiency_points = results
        .iter()
        .map(|&(c, e, _, _, _)| (c as f64, e))
        .collect::<Vec<_>>();
    efficiency_points.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap_or(std::cmp::Ordering::Equal));

    // Draw overlaid efficiency line with different color
    route_chart
        .draw_series(LineSeries::new(
            efficiency_points
                .iter()
                .map(|&(c, e)| (c, e * ((max_routes as f64) / max_efficiency))), // Scale efficiency to fit on same y-axis
            BLUE.mix(0.8).stroke_width(2),
        ))?
        .label("Trade-off Efficiency (scaled)")
        .legend(|(x, y)| {
            PathElement::new(vec![(x, y), (x + 20, y)], BLUE.mix(0.8).stroke_width(2))
        });

    route_chart.draw_series(efficiency_points.iter().map(|&(c, e)| {
        Circle::new(
            (c, e * ((max_routes as f64) / max_efficiency)),
            4,
            BLUE.filled(),
        )
    }))?;

    // Add a note about scaling
    route_chart.draw_series(std::iter::once(Text::new(
        format!(
            "Note: Efficiency scaled by factor {:.1}x",
            (max_routes as f64) / max_efficiency
        ),
        (
            (min_count as f64 + max_count as f64) / 2.0,
            ((max_routes as f64) * 0.05),
        ),
        ("sans-serif", 12).into_font(),
    )))?;

    // Add legend
    route_chart
        .configure_series_labels()
        .background_style(&WHITE.mix(0.8))
        .border_style(&BLACK)
        .position(SeriesLabelPosition::UpperRight)
        .draw()?;

    root.present()?;
    Ok(())
}
