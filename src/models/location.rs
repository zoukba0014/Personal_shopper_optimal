// Location model representing coordinates in 2D space

/// Represents a location with (x, y) coordinates
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Location {
    pub x: f64,
    pub y: f64,
}

impl Location {
    /// Creates a new location with the given coordinates
    pub fn new(x: f64, y: f64) -> Self {
        Self { x, y }
    }

    /// Calculates the Euclidean distance between two locations
    pub fn distance_to(&self, other: &Location) -> f64 {
        ((self.x - other.x).powi(2) + (self.y - other.y).powi(2)).sqrt()
    }

    /// Manhattan distance between two locations
    pub fn manhattan_distance_to(&self, other: &Location) -> f64 {
        (self.x - other.x).abs() + (self.y - other.y).abs()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_distance() {
        let loc1 = Location::new(0.0, 0.0);
        let loc2 = Location::new(3.0, 4.0);

        assert_eq!(loc1.distance_to(&loc2), 5.0);
    }

    #[test]
    fn test_manhattan_distance() {
        let loc1 = Location::new(0.0, 0.0);
        let loc2 = Location::new(3.0, 4.0);

        assert_eq!(loc1.manhattan_distance_to(&loc2), 7.0);
    }
}
