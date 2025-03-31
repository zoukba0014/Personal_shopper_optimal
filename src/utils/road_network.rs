use crate::models::Location;
use std::cmp::Ordering;
use std::collections::{BinaryHeap, HashMap, HashSet};

/// Road network graph structure
pub struct RoadGraph {
    vertices: HashMap<u64, (f64, f64)>, // Vertex ID -> (longitude, latitude)
    adjacency_list: HashMap<u64, Vec<(u64, f64)>>, // Vertex ID -> [(adjacent vertex ID, distance)]
}

/// Node for Dijkstra algorithm
#[derive(Copy, Clone, Eq, PartialEq)]
struct DijkstraNode {
    vertex: u64,
    distance: u64, // Use integer distance (in millimeters) to avoid floating-point comparison issues
}

// Implement Ord for DijkstraNode, so that nodes with smaller distances have higher priority
impl Ord for DijkstraNode {
    fn cmp(&self, other: &Self) -> Ordering {
        // Note: This is reversed order, because we want a min-heap
        other.distance.cmp(&self.distance)
    }
}

impl PartialOrd for DijkstraNode {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl RoadGraph {
    /// Create a new road network graph
    pub fn new(vertices: HashMap<u64, (f64, f64)>, edges: HashMap<u64, (u64, u64)>) -> Self {
        // Build adjacency list
        let mut adjacency_list = HashMap::new();

        for (_, (start_id, end_id)) in edges {
            if let (Some(start_pos), Some(end_pos)) =
                (vertices.get(&start_id), vertices.get(&end_id))
            {
                // Calculate edge distance
                let distance = RoadGraph::euclidean_distance(start_pos, end_pos);

                // Add edge to adjacency list (undirected graph)
                adjacency_list
                    .entry(start_id)
                    .or_insert_with(Vec::new)
                    .push((end_id, distance));

                adjacency_list
                    .entry(end_id)
                    .or_insert_with(Vec::new)
                    .push((start_id, distance));
            }
        }

        RoadGraph {
            vertices,
            adjacency_list,
        }
    }

    /// Calculate Euclidean distance between two points
    fn euclidean_distance(point1: &(f64, f64), point2: &(f64, f64)) -> f64 {
        let dx = point1.0 - point2.0;
        let dy = point1.1 - point2.1;
        (dx * dx + dy * dy).sqrt()
    }

    /// Find the nearest road vertex to a given location
    pub fn find_nearest_vertex(&self, location: &Location) -> Option<u64> {
        let mut nearest_vertex = None;
        let mut min_distance = f64::MAX;

        for (vertex_id, (lon, lat)) in &self.vertices {
            let distance = RoadGraph::euclidean_distance(&(*lon, *lat), &(location.x, location.y));
            if distance < min_distance {
                min_distance = distance;
                nearest_vertex = Some(*vertex_id);
            }
        }

        nearest_vertex
    }

    /// Calculate the shortest path distance between two vertices using Dijkstra algorithm
    pub fn shortest_path_distance(&self, start_vertex: u64, end_vertex: u64) -> Option<f64> {
        // Special case: start and end vertices are the same
        if start_vertex == end_vertex {
            return Some(0.0);
        }

        // Initialize distance map and visited set
        let mut distances = HashMap::new();
        let mut visited = HashSet::new();
        let mut priority_queue = BinaryHeap::new();

        // Set start vertex distance to 0 and add to queue
        distances.insert(start_vertex, 0.0);
        priority_queue.push(DijkstraNode {
            vertex: start_vertex,
            distance: 0,
        });

        // Main loop of Dijkstra algorithm
        while let Some(DijkstraNode {
            vertex,
            distance: _,
        }) = priority_queue.pop()
        {
            // If found the end vertex, return the distance
            if vertex == end_vertex {
                return Some(distances[&vertex]);
            }

            // If already visited this vertex, skip
            if visited.contains(&vertex) {
                continue;
            }

            // Mark vertex as visited
            visited.insert(vertex);

            // Traverse adjacent vertices
            if let Some(neighbors) = self.adjacency_list.get(&vertex) {
                for &(neighbor, edge_distance) in neighbors {
                    // If already visited, skip
                    if visited.contains(&neighbor) {
                        continue;
                    }

                    // Calculate new distance to neighbor through current vertex
                    let new_distance = distances[&vertex] + edge_distance;

                    // If found a shorter path, update distance
                    let is_shorter = match distances.get(&neighbor) {
                        Some(&current) => new_distance < current,
                        None => true,
                    };

                    if is_shorter {
                        // Update distance and add to queue
                        distances.insert(neighbor, new_distance);
                        priority_queue.push(DijkstraNode {
                            vertex: neighbor,
                            distance: (new_distance * 1000.0) as u64, // Convert to millimeters (integer) for comparison
                        });
                    }
                }
            }
        }

        // No path found
        None
    }

    /// Calculate the distance between two locations on the road network
    pub fn location_distance(&self, from: &Location, to: &Location) -> Option<f64> {
        // Find the nearest start and end vertices
        let start_vertex = self.find_nearest_vertex(from)?;
        let end_vertex = self.find_nearest_vertex(to)?;

        // Calculate distance from start location to start vertex
        let start_point = self.vertices.get(&start_vertex)?;
        let start_distance =
            RoadGraph::euclidean_distance(&(start_point.0, start_point.1), &(from.x, from.y));

        // Calculate distance from end vertex to end location
        let end_point = self.vertices.get(&end_vertex)?;
        let end_distance =
            RoadGraph::euclidean_distance(&(end_point.0, end_point.1), &(to.x, to.y));

        // Calculate shortest path in the road network
        let network_distance = self.shortest_path_distance(start_vertex, end_vertex)?;

        // Total distance = start to start vertex + network shortest path + end vertex to end
        Some(start_distance + network_distance + end_distance)
    }
}
