use criterion::{black_box, criterion_group, criterion_main, Criterion};
use personal_shopper::{
    algorithms::bsl_psd::BSLPSD,
    algorithms::PSDSolver,
    models::{Location, Product, ShoppingList, Store},
};
use std::collections::HashMap;

fn benchmark_bsl_psd(c: &mut Criterion) {
    // Create benchmark data
    let (stores, shopping_list, shopper_location, customer_location) = create_benchmark_data();

    // Initialize BSL-PSD
    let mut bsl_psd = BSLPSD::new(stores.clone());
    bsl_psd.precompute_data();

    // Benchmark the solve function
    c.bench_function("bsl_psd_solve", |b| {
        b.iter(|| {
            bsl_psd.solve(
                black_box(&shopping_list),
                black_box(shopper_location),
                black_box(customer_location),
            )
        })
    });

    // Benchmark precomputation (mainly for larger datasets)
    c.bench_function("bsl_psd_precompute", |b| {
        b.iter(|| {
            let mut solver = BSLPSD::new(stores.clone());
            solver.precompute_data();
        })
    });
}

// Create data for benchmarking
fn create_benchmark_data() -> (HashMap<u32, Store>, ShoppingList, Location, Location) {
    let mut stores = HashMap::new();

    // Create 25 stores with different products
    for i in 1..=25 {
        let x = (i % 5) as f64 * 10.0;
        let y = (i / 5) as f64 * 10.0;
        let location = Location::new(x, y);

        let mut products = HashMap::new();

        // Each store has some products
        for j in 1..=20 {
            if j % 5 == i % 5 || j % 7 == i % 7 {
                let price = 5.0 + ((i * j) % 10) as f64;
                products.insert(j, Product::new(format!("Product {}", j), price));
            }
        }

        stores.insert(i, Store::new(i, location, products));
    }

    // Create shopping list with 5 products
    let mut shopping_list = ShoppingList::new();
    for i in 1..=5 {
        shopping_list.add_item(i, 1);
    }

    // Shopper and customer locations
    let shopper_location = Location::new(0.0, 0.0);
    let customer_location = Location::new(50.0, 50.0);

    (stores, shopping_list, shopper_location, customer_location)
}

criterion_group!(benches, benchmark_bsl_psd);
criterion_main!(benches);
