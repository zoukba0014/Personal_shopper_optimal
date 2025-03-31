import matplotlib.pyplot as plt
import numpy as np
import os
from matplotlib.lines import Line2D

def load_vertices(file_path):
    """Load vertices from file."""
    vertices = {}
    with open(file_path, 'r') as f:
        for line in f:
            parts = line.strip().split()
            if len(parts) >= 3:
                vertex_id = int(parts[0])
                lon = float(parts[1])
                lat = float(parts[2])
                vertices[vertex_id] = (lon, lat)
    return vertices

def load_edges(file_path, vertices):
    """Load edges from file."""
    edges = []
    with open(file_path, 'r') as f:
        for line in f:
            parts = line.strip().split()
            if len(parts) >= 3:
                start_id = int(parts[1])
                end_id = int(parts[2])
                if start_id in vertices and end_id in vertices:
                    edges.append((vertices[start_id], vertices[end_id]))
    return edges

def load_restaurants(file_path):
    """Load restaurant POIs from file."""
    restaurants = []
    with open(file_path, 'r') as f:
        for line in f:
            parts = line.strip().split()
            if len(parts) >= 3:
                lon = float(parts[1])
                lat = float(parts[2])
                restaurants.append((lon, lat))
    return restaurants

def find_file(base_path, city_code, file_type):
    """Find the correct file for a given city and type."""
    # Try different possible filename formats
    possible_paths = [
        os.path.join(base_path, f"{file_type}{city_code}.txt"),  # Example: RoadVerticesAMS.txt
        os.path.join(base_path, "POI", f"{file_type}{city_code}.txt"),  # Example: POI/RestaurantsAMS.txt
        os.path.join(base_path, "POI", f"{file_type}{city_code[0]}{city_code[1:].lower()}.txt")  # Example: POI/RestaurantsAms.txt
    ]
    
    for path in possible_paths:
        if os.path.exists(path):
            return path
    
    print(f"Warning: Could not find {file_type} file for {city_code}")
    return None

def load_city_data(base_path, city_code, sample_edges=True, max_edges=10000):
    """Load data for a specific city."""
    # Find the correct files
    vertex_file = find_file(base_path, city_code, "RoadVertices")
    edge_file = find_file(base_path, city_code, "RoadEdges")
    restaurant_file = find_file(base_path, city_code, "Restaurants")
    
    # Load data
    data = {}
    
    if vertex_file:
        print(f"Loading vertices from {vertex_file}")
        data['vertices'] = load_vertices(vertex_file)
        print(f"Loaded {len(data['vertices'])} vertices")
    else:
        return None
    
    if edge_file:
        print(f"Loading edges from {edge_file}")
        data['edges'] = load_edges(edge_file, data['vertices'])
        print(f"Loaded {len(data['edges'])} edges")
        
        # Sample edges (if needed)
        if sample_edges and len(data['edges']) > max_edges:
            indices = np.random.choice(len(data['edges']), max_edges, replace=False)
            data['sampled_edges'] = [data['edges'][i] for i in indices]
            print(f"Sampled {len(data['sampled_edges'])} edges for visualization")
        else:
            data['sampled_edges'] = data['edges']
    else:
        return None
    
    if restaurant_file:
        print(f"Loading restaurants from {restaurant_file}")
        data['restaurants'] = load_restaurants(restaurant_file)
        print(f"Loaded {len(data['restaurants'])} restaurants")
    else:
        data['restaurants'] = []
    
    return data

def create_improved_visualization():
    """Create a combined visualization with visible road networks and restaurants on top."""
    base_path = "data"  # Path to data folder
    
    # Mapping of city codes to full names
    city_names = {"AMS": "Amsterdam", "BER": "Berlin", "OSLO": "Oslo"}
    
    # Create figure
    fig, axes = plt.subplots(1, 3, figsize=(18, 6))
    
    # For each city
    for i, city_code in enumerate(["AMS", "BER", "OSLO"]):
        # Load data
        data = load_city_data(base_path, city_code, sample_edges=True, max_edges=5000)
        
        if not data:
            print(f"Skipping {city_code} due to missing data")
            continue
        
        # Get coordinates from vertices for plotting
        vertex_coords = list(data['vertices'].values())
        vertex_lons, vertex_lats = zip(*vertex_coords)
        
        # 1. First draw the vertices
        axes[i].scatter(vertex_lons, vertex_lats, c='lightblue', s=0.5, alpha=0.4, zorder=1)
        
        # 2. Then draw the edges (roads)
        for (start_lon, start_lat), (end_lon, end_lat) in data['sampled_edges']:
            axes[i].plot([start_lon, end_lon], [start_lat, end_lat], 'b-', linewidth=0.2, alpha=0.5, zorder=2)
        
        # 3. Finally draw the restaurants, ensuring they are on top
        if data['restaurants']:
            lons, lats = zip(*data['restaurants'])
            axes[i].scatter(lons, lats, c='red', s=15, alpha=1.0, zorder=3)
        
        # Set plot title and labels
        axes[i].set_title(f"{city_names[city_code]}\n{len(data['vertices'])} vertices, {len(data['edges'])} edges, {len(data['restaurants'])} restaurants")
        axes[i].set_xlabel("Longitude")
        axes[i].set_ylabel("Latitude")
        axes[i].grid(True, linestyle='--', alpha=0.3)
    
    # Create legend
    legend_elements = [
        Line2D([0], [0], marker='o', color='w', markerfacecolor='lightblue', markersize=6, alpha=0.7, label='Vertices'),
        Line2D([0], [0], color='b', lw=1, alpha=0.6, label='Edges (Roads)'),
        Line2D([0], [0], marker='o', color='w', markerfacecolor='r', markersize=8, alpha=1.0, label='Restaurants')
    ]
    fig.legend(handles=legend_elements, loc='lower center', bbox_to_anchor=(0.5, 0.01), ncol=3)
    
    # Add main title
    plt.suptitle("Road Networks and Restaurant POIs Across Three European Cities", fontsize=16, y=0.98)
    
    # Adjust layout
    plt.tight_layout(rect=[0, 0.05, 1, 0.95])
    
    # Save and display
    output_file = "city_comparison_with_full_network.png"
    plt.savefig(output_file, dpi=300, bbox_inches='tight')
    print(f"Visualization saved to {output_file}")
    
    return fig

if __name__ == "__main__":
    # Create visualization
    fig = create_improved_visualization()
    plt.show()