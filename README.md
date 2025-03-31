# Personal Shopper - Route Optimization Algorithm

A simple and efficient shopping route optimization algorithm based on the BSL-PSD (Best Shopping List - Personal Shopper's Dilemma) algorithm.

## Installation

### Installing Rust

1. Install Rust environment (using Rustup):

   ```bash
   # For MacOS/Linux systems
   curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
   
   # For Windows systems
   # Download and run rustup-init.exe from https://rustup.rs
   ```

2. Follow the on-screen instructions to complete the installation

3. Verify the installation:
   ```bash
   rustc --version
   cargo --version
   ```

### Project Setup

1. Clone the repository:
   ```bash
   git clone https://github.com/yourusername/personal_shopper.git
   cd personal_shopper
   ```

2. Build the project:
   ```bash
   cargo build --release
   ```

## Running the Project
The default map is AMS, if you want to change the map location, you can change it in the code 
```bash
let city_code = "AMS"
```

### Running the Main Program

```bash
cargo run --release
```

The application parameters are configured in the source code. There are no command-line options.

### Running Tests

The project includes the following tests:

1. **Standard Unit Tests**
   ```bash
   cargo test
   ```

2. **Threshold Performance Tests** - Tests how different threshold values affect algorithm performance
   ```bash
   cargo test -p personal_shopper --test threshold_performance_analysis -- --nocapture
   ```

3. **Extreme Threshold Tests** - Tests algorithm behavior with extreme threshold values
   ```bash
   cargo test -p personal_shopper --test extreme_threshold_analysis -- --nocapture
   ```

4. **Product Count Tests** - Tests how different product counts affect algorithm performance
   ```bash
   cargo test -p personal_shopper --test product_count_threshold_analysis -- --nocapture
   ```

5. **Supply Comparison Tests** - Compares algorithm performance under different supply conditions
   ```bash
   cargo test -p personal_shopper --test test_supply_comparison -- --nocapture
   ```

6. **Visualization Solve Tests** - Runs BSL-PSD algorithm tests with visualization features
   ```bash
   cargo test -p personal_shopper --test test_bsl_psd_with_visualization_solve -- --nocapture
   ```

7. **Product Count Comparison Tests** - Compares performance with different product counts
   ```bash
   cargo test -p personal_shopper --test product_count_comparison -- --nocapture
   ```

Tests will generate output files in the project root directory. The `--nocapture` flag ensures that test output is displayed in the console.

## Data Files

The project requires the following data files, which should be placed in the `data/` directory:

- `RoadVerticesAMS.txt`: Road network vertices (ID, longitude, latitude)
- `RoadEdgesAMS.txt`: Road network edges (ID, start_vertex_id, end_vertex_id)
- `RestaurantsAMS.txt`: Restaurant/store locations (ID, longitude, latitude, edge_id, distance)

## Project Overview

This project solves the Personal Shopper's Dilemma (PSD) problem, which involves:
- Purchasing a set of products from multiple stores
- Different pricing of products at different stores
- Different store locations, affecting travel time
- Finding optimal routes that balance total shopping time and cost

The algorithm generates a set of non-dominated routes (skyline/Pareto optimal set), representing different trade-offs between time and cost, allowing users to select the route that best matches their preferences. 