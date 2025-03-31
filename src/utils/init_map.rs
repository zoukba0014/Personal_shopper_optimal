use rayon::prelude::*;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::collections::HashMap as StdHashMap;
use std::fs;
use std::fs::File;
use std::io::{self, BufRead};
use std::path::Path;
use std::sync::{Arc, Mutex};

use crate::models::Location;
use crate::{Product, Store};

use super::road_network::RoadGraph;

// Assuming StoreId is u32 type
pub type StoreId = u32;

// Extended init_map function that returns road network data and pre-computed travel times
pub fn init_map_with_road_network(
    city_code: &str,
    infinity: bool,
    total_product_type: u32,
) -> Result<(HashMap<StoreId, Store>, HashMap<(StoreId, StoreId), f64>), io::Error> {
    println!("Initializing map data for city {}...", city_code);

    // Load road vertex data
    let vertices = load_road_vertices(city_code)?;
    println!("Loaded {} road vertices", vertices.len());

    // Load road edge data
    let edges = load_road_edges(city_code)?;
    println!("Loaded {} roads", edges.len());

    // Load restaurant data
    let restaurants = load_restaurants(city_code)?;
    println!("Loaded {} restaurants", restaurants.len());

    // Convert restaurants to stores
    let stores =
        convert_restaurants_to_stores(restaurants, &vertices, infinity, total_product_type);
    println!("Converted restaurant data to {} stores", stores.len());

    // Use road network to pre-compute travel times
    println!("Calculating travel times between stores based on road network...");
    let travel_times = precompute_travel_times_with_road_network(&stores, &vertices, &edges);
    println!(
        "Calculated travel times for {} store pairs",
        travel_times.len()
    );

    Ok((stores, travel_times))
}

// Load road vertex data
fn load_road_vertices(city_code: &str) -> Result<HashMap<u64, (f64, f64)>, io::Error> {
    let file_name = format!("data/RoadVertices{}.txt", city_code);
    let path = Path::new(&file_name);
    let file = File::open(path)?;
    let reader = io::BufReader::new(file);

    let mut vertices = HashMap::new();

    for line in reader.lines() {
        let line = line?;
        let parts: Vec<&str> = line.split_whitespace().collect();

        if parts.len() >= 3 {
            let id = parts[0].parse::<u64>().unwrap_or(0);
            let longitude = parts[1].parse::<f64>().unwrap_or(0.0);
            let latitude = parts[2].parse::<f64>().unwrap_or(0.0);

            vertices.insert(id, (longitude, latitude));
        }
    }

    Ok(vertices)
}

// Load road edge data
fn load_road_edges(city_code: &str) -> Result<HashMap<u64, (u64, u64)>, io::Error> {
    let file_name = format!("data/RoadEdges{}.txt", city_code);
    let path = Path::new(&file_name);
    let file = File::open(path)?;
    let reader = io::BufReader::new(file);

    let mut edges = HashMap::new();

    for line in reader.lines() {
        let line = line?;
        let parts: Vec<&str> = line.split_whitespace().collect();

        if parts.len() >= 3 {
            let id = parts[0].parse::<u64>().unwrap_or(0);
            let start_id = parts[1].parse::<u64>().unwrap_or(0);
            let end_id = parts[2].parse::<u64>().unwrap_or(0);

            edges.insert(id, (start_id, end_id));
        }
    }

    Ok(edges)
}

// Load restaurant data
fn load_restaurants(city_code: &str) -> Result<Vec<(u64, f64, f64, u64, f64)>, io::Error> {
    let file_name = format!("data/Restaurants{}.txt", city_code);
    let path = Path::new(&file_name);
    let file = File::open(path)?;
    let reader = io::BufReader::new(file);

    let mut restaurants = Vec::new();

    for line in reader.lines() {
        let line = line?;
        let parts: Vec<&str> = line.split_whitespace().collect();

        if parts.len() >= 5 {
            let id = parts[0].parse::<u64>().unwrap_or(0);
            let longitude = parts[1].parse::<f64>().unwrap_or(0.0);
            let latitude = parts[2].parse::<f64>().unwrap_or(0.0);
            let edge_id = parts[3].parse::<u64>().unwrap_or(0);
            let distance = parts[4].parse::<f64>().unwrap_or(0.0);

            restaurants.push((id, longitude, latitude, edge_id, distance));
        }
    }

    Ok(restaurants)
}

// Convert restaurant data to stores
fn convert_restaurants_to_stores(
    restaurants: Vec<(u64, f64, f64, u64, f64)>,
    _vertices: &HashMap<u64, (f64, f64)>,
    infinity: bool,
    total_product_type: u32,
) -> HashMap<StoreId, Store> {
    let mut stores = HashMap::new();

    for (i, (_rest_id, longitude, latitude, _, _)) in restaurants.iter().enumerate() {
        // Convert u64 ID to u32 StoreId (ensuring it doesn't exceed u32 range)
        let store_id = i;
        // println!("store id: {:?}", store_id);

        // Create random products and inventory for each restaurant
        let mut products = HashMap::new();
        let mut inventory = HashMap::new();

        // Randomly select 3-5 products for each store
        let num_products = 3 + (i % 3) as usize;
        let mut available_product_ids = Vec::new();

        // Use store ID multiplied by different prime numbers to create a more random distribution
        for offset in 0..total_product_type {
            let product_id = 1 + ((store_id as u32 * 17 + offset * 19) % total_product_type);

            if !available_product_ids.contains(&product_id) {
                available_product_ids.push(product_id);
                if available_product_ids.len() >= num_products {
                    break;
                }
            }
        }

        // If there aren't enough products, add some more
        while available_product_ids.len() < num_products {
            let product_id = (available_product_ids.len() as u32 % total_product_type) + 1;
            if !available_product_ids.contains(&product_id) {
                available_product_ids.push(product_id);
            }
        }
        // println!("produc ids: {:?}", available_product_ids.len());

        // Create products and inventory for the selected product IDs
        for &product_id in &available_product_ids {
            // Dynamically generate product names
            let product_name = if product_id <= 26 {
                (('A' as u8 + (product_id - 1) as u8) as char).to_string()
            } else {
                format!("Product{}", product_id)
            };

            let product_cost = 5.0 + ((store_id as u32 + product_id) % total_product_type) as f64; // Cost between 5-15
            products.insert(product_id, Product::new(&product_name, product_cost));

            if !infinity {
                inventory.insert(product_id, 3 + (product_id % total_product_type) as u32);
            // Inventory between 3-7
            } else {
                inventory.insert(product_id, 1000000);
            }
        }

        // Create store
        let store_location = Location::new(*longitude, *latitude);
        let store = Store::new_with_inventory(store_id as u32, store_location, products, inventory);
        stores.insert(store_id as u32, store);
    }

    stores
}
pub fn precompute_travel_times_with_road_network(
    stores: &HashMap<u32, crate::Store>,
    road_vertices: &HashMap<u64, (f64, f64)>,
    road_edges: &HashMap<u64, (u64, u64)>,
) -> HashMap<(u32, u32), f64> {
    // Define serializable structure for JSON
    #[derive(Serialize, Deserialize)]
    struct TravelTimesCache {
        // Since tuple keys in HashMap can't be directly serialized, we convert keys to strings
        times: StdHashMap<String, f64>,
    }

    // Cache file path
    let cache_path = "travel_times_cache.json";

    // Try to load from cache
    if Path::new(cache_path).exists() {
        println!("Attempting to load travel times from cache...");
        match fs::read_to_string(cache_path) {
            Ok(json_str) => {
                match serde_json::from_str::<TravelTimesCache>(&json_str) {
                    Ok(cache) => {
                        let mut result = HashMap::new();
                        // Convert string keys back to tuples
                        for (key_str, value) in cache.times {
                            if let Some((from_str, to_str)) = key_str.split_once('-') {
                                if let (Ok(from), Ok(to)) =
                                    (from_str.parse::<u32>(), to_str.parse::<u32>())
                                {
                                    result.insert((from, to), value);
                                }
                            }
                        }
                        println!(
                            "Successfully loaded {} travel time records from cache",
                            result.len()
                        );
                        return result;
                    }
                    Err(e) => println!("Failed to parse cache: {}, will recalculate", e),
                }
            }
            Err(e) => println!("Failed to read cache: {}, will recalculate", e),
        }
    }

    println!("Starting travel time calculation...");

    // Build road network graph
    let graph = Arc::new(RoadGraph::new(road_vertices.clone(), road_edges.clone()));

    // Wrap HashMap with Arc and Mutex for safe sharing between threads
    let travel_times = Arc::new(Mutex::new(HashMap::new()));
    let stores = Arc::new(stores.clone());

    let store_ids: Vec<u32> = stores.keys().cloned().collect();
    println!("Number of stores: {}", store_ids.len());

    // Generate all store pairs that need calculation
    let pairs: Vec<(usize, usize)> = (0..store_ids.len())
        .flat_map(|i| ((i + 1)..store_ids.len()).map(move |j| (i, j)))
        .collect();

    println!("Store pairs to calculate: {}", pairs.len());

    // Process all pairs in parallel
    pairs.par_iter().for_each(|&(i, j)| {
        let store_i = &stores[&store_ids[i]];
        let store_j = &stores[&store_ids[j]];

        // Calculate distance using road network
        let distance =
            if let Some(d) = graph.location_distance(&store_i.location, &store_j.location) {
                d * 1000.0 // Convert to meters
            } else {
                // If no path is found, fall back to Euclidean distance
                store_i.location.distance_to(&store_j.location) * 1000.0
            };
        // Print information (consider using atomic operations or other thread-safe logging methods)
        println!(
            "distance between {} and {}: {}",
            store_ids[i], store_ids[j], distance
        );

        // Acquire lock and update travel_times
        let mut times = travel_times.lock().unwrap();
        times.insert((store_ids[i], store_ids[j]), distance);
        times.insert((store_ids[j], store_ids[i]), distance);
    });

    // Get final result
    let result = Arc::try_unwrap(travel_times).unwrap().into_inner().unwrap();

    // Save to JSON cache
    println!("Saving results to cache...");
    let mut cache_data = StdHashMap::new();
    for ((from, to), distance) in &result {
        // Convert tuple keys to strings
        let key = format!("{}-{}", from, to);
        cache_data.insert(key, *distance);
    }

    let cache = TravelTimesCache { times: cache_data };

    match serde_json::to_string_pretty(&cache) {
        Ok(json_str) => match fs::write(cache_path, json_str) {
            Ok(_) => println!("Cache successfully saved to {}", cache_path),
            Err(e) => println!("Failed to save cache: {}", e),
        },
        Err(e) => println!("Failed to serialize cache: {}", e),
    }

    result
}
