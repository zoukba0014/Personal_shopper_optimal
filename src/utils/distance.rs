// Distance calculation utilities

use crate::models::Location;

/// Calculate the Euclidean distance between two points
pub fn euclidean_distance(p1: &Location, p2: &Location) -> f64 {
    ((p1.x - p2.x).powi(2) + (p1.y - p2.y).powi(2)).sqrt()
}

/// Calculate the Manhattan distance between two points
pub fn manhattan_distance(p1: &Location, p2: &Location) -> f64 {
    (p1.x - p2.x).abs() + (p1.y - p2.y).abs()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_euclidean_distance() {
        let p1 = Location::new(0.0, 0.0);
        let p2 = Location::new(3.0, 4.0);

        assert_eq!(euclidean_distance(&p1, &p2), 5.0);
    }

    #[test]
    fn test_manhattan_distance() {
        let p1 = Location::new(0.0, 0.0);
        let p2 = Location::new(3.0, 4.0);

        assert_eq!(manhattan_distance(&p1, &p2), 7.0);
    }
}
