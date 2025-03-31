use crate::models::{Cost, ShoppingRoute, Time};

/// Determines if a point is conventionally dominated by another
pub fn is_conventionally_dominated(
    time: Time,
    cost: Cost,
    dominator_time: Time,
    dominator_cost: Cost,
) -> bool {
    (dominator_time < time && dominator_cost <= cost)
        || (dominator_time <= time && dominator_cost < cost)
}

/// Check if a route is linearly dominated by the skyline
pub fn is_linearly_dominated(route: &ShoppingRoute, skyline: &[ShoppingRoute]) -> bool {
    if skyline.is_empty() {
        return false;
    }

    // Find the position where the route would be inserted (by shopping time)
    let mut pos = skyline.len();
    for (i, skyline_route) in skyline.iter().enumerate() {
        if route.shopping_time < skyline_route.shopping_time {
            pos = i;
            break;
        }
    }

    // Check conventional domination by comparing with the nearest point
    if pos < skyline.len() {
        let comparison = &skyline[pos];
        if is_conventionally_dominated(
            route.shopping_time,
            route.shopping_cost,
            comparison.shopping_time,
            comparison.shopping_cost,
        ) {
            return true;
        }
    }

    if pos > 0 {
        let comparison = &skyline[pos - 1];
        if is_conventionally_dominated(
            route.shopping_time,
            route.shopping_cost,
            comparison.shopping_time,
            comparison.shopping_cost,
        ) {
            return true;
        }
    }

    // Check for linear domination
    if pos > 0 && pos < skyline.len() {
        let left = &skyline[pos - 1];
        let right = &skyline[pos];

        // Check if the point is above the line connecting left and right
        // This is a simplified implementation - in practice, we need to check
        // all pairs of adjacent points in the skyline
        let x1 = left.shopping_time;
        let y1 = left.shopping_cost;
        let x2 = right.shopping_time;
        let y2 = right.shopping_cost;
        let x = route.shopping_time;
        let y = route.shopping_cost;

        if x2 > x1 {
            // Calculate the y-coordinate on the line at point x
            let slope = (y2 - y1) / (x2 - x1);
            let y_on_line = y1 + slope * (x - x1);

            if y > y_on_line {
                return true;
            }
        }
    }

    false
}

/// Inserts a route into the skyline if it's not dominated
/// Returns true if the route was inserted, false otherwise
pub fn insert_into_skyline(skyline: &mut Vec<ShoppingRoute>, route: ShoppingRoute) -> bool {
    // Check if the route is dominated
    if is_linearly_dominated(&route, skyline) {
        return false;
    }

    // Find insertion position
    let mut pos = skyline.len();
    for (i, skyline_route) in skyline.iter().enumerate() {
        if route.shopping_time < skyline_route.shopping_time {
            pos = i;
            break;
        }
    }

    // Insert the route
    skyline.insert(pos, route);

    // Remove dominated routes
    let mut i = 0;
    while i < skyline.len() {
        if i != pos && is_dominated_by_skyline(&skyline[i], skyline) {
            skyline.remove(i);
            if i < pos {
                pos -= 1;
            }
        } else {
            i += 1;
        }
    }

    true
}

/// Checks if a route is dominated by any route in the skyline
fn is_dominated_by_skyline(route: &ShoppingRoute, skyline: &[ShoppingRoute]) -> bool {
    for (_i, skyline_route) in skyline.iter().enumerate() {
        if std::ptr::eq(route, skyline_route) {
            continue; // Skip comparing with itself
        }

        if is_conventionally_dominated(
            route.shopping_time,
            route.shopping_cost,
            skyline_route.shopping_time,
            skyline_route.shopping_cost,
        ) {
            return true;
        }
    }

    false
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_conventional_domination() {
        // Point A: (10, 20)
        // Point B: (8, 15)  - B dominates A
        // Point C: (12, 18) - C doesn't dominate A
        // Point D: (10, 18) - D dominates A
        assert!(is_conventionally_dominated(10.0, 20.0, 8.0, 15.0));
        assert!(!is_conventionally_dominated(10.0, 20.0, 12.0, 18.0));
        assert!(is_conventionally_dominated(10.0, 20.0, 10.0, 18.0));
    }

    #[test]
    fn test_skyline_insertion() {
        let mut skyline = Vec::new();

        // Add first route: (10, 30)
        let route1 = ShoppingRoute {
            stores: vec![1, 2],
            shopping_time: 10.0,
            shopping_cost: 30.0,
        };

        assert!(insert_into_skyline(&mut skyline, route1));
        assert_eq!(skyline.len(), 1);

        // Add dominated route: (12, 35) - should be rejected
        let route2 = ShoppingRoute {
            stores: vec![1, 3],
            shopping_time: 12.0,
            shopping_cost: 35.0,
        };

        assert!(!insert_into_skyline(&mut skyline, route2));
        assert_eq!(skyline.len(), 1);

        // Add non-dominated route: (15, 25)
        let route3 = ShoppingRoute {
            stores: vec![2, 3],
            shopping_time: 15.0,
            shopping_cost: 25.0,
        };

        assert!(insert_into_skyline(&mut skyline, route3));
        assert_eq!(skyline.len(), 2);

        // Check skyline order (by shopping time)
        assert_eq!(skyline[0].shopping_time, 10.0);
        assert_eq!(skyline[1].shopping_time, 15.0);
    }
}
