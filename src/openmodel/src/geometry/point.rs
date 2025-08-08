use crate::common::{Data, HasJsonData, FromJsonData};
use serde_json::Value;
use crate::geometry::Vector;
use serde::{Deserialize, Serialize};
use std::ops::{Add, AddAssign, Div, DivAssign, Index, IndexMut, Mul, MulAssign, Sub, SubAssign};
use std::fmt;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Point {
    /// The x coordinate of the point.
    pub x: f64,
    /// The y coordinate of the point.
    pub y: f64,
    /// The z coordinate of the point.
    pub z: f64,
    /// Associated data - guid and name.
    pub data: Data,
}

impl Point {
    /// Creates a new `Point` with default `Data`.
    ///
    /// # Arguments
    ///
    /// * `x` - The x component.
    /// * `y` - The y component.
    /// * `z` - The z component.
    ///
    /// # Example
    ///
    /// ```
    /// use openmodel::geometry::Point;
    /// let p = Point::new(1.0, 2.0, 3.0);
    /// assert_eq!(p.x, 1.0);
    /// assert_eq!(p.y, 2.0);
    /// assert_eq!(p.z, 3.0);
    /// ```
    pub fn new(x: f64, y: f64, z: f64) -> Self {
        Point {
            x,
            y,
            z,
            data: Data::with_name("Point"),
        }
    }

    /// Creates a new `Point` with a specified name for `Data`.
    ///
    /// # Arguments
    ///
    /// * `name` - The name for the `Data`.
    /// * `x` - The x component.
    /// * `y` - The y component.
    /// * `z` - The z component.
    ///
    /// # Example
    ///
    /// ```
    /// use openmodel::geometry::Point;
    /// let p = Point::with_name("MyPoint".to_string(), 1.0, 2.0, 3.0);
    /// assert_eq!(p.x, 1.0);
    /// assert_eq!(p.y, 2.0);
    /// assert_eq!(p.z, 3.0);
    /// ```
    pub fn with_name(name: String, x: f64, y: f64, z: f64) -> Self {
        Point {
            x,
            y,
            z,
            data: Data::with_name(&name),
        }
    }

    /// Computes the distance between two points.
    ///
    /// # Arguments
    ///
    /// * `other` - The other point.
    ///
    /// # Example
    ///
    /// ```
    /// use openmodel::geometry::Point;
    /// let p1 = Point::new(1.0, 2.0, 2.0);
    /// let p2 = Point::new(4.0, 6.0, 6.0);
    /// let distance = p1.distance(&p2);
    /// assert_eq!(distance, 6.4031242374328485);
    /// ```
    pub fn distance(&self, other: &Point) -> f64 {
        ((self.x - other.x).powi(2) + (self.y - other.y).powi(2) + (self.z - other.z).powi(2))
            .sqrt()
    }
}

impl Default for Point {
    /// Creates a default `Point` with all coordinates set to 0.0.
    ///
    /// # Example
    ///
    /// ```
    /// use openmodel::geometry::Point;
    /// let p = Point::default();
    /// assert_eq!(p.x, 0.0);
    /// assert_eq!(p.y, 0.0);
    /// assert_eq!(p.z, 0.0);
    /// ```
    fn default() -> Self {
        Point {
            x: 0.0,
            y: 0.0,
            z: 0.0,
            data: Data::with_name("Point"),
        }
    }
}

impl From<Vector> for Point {
    /// Converts a `Vector` into a `Point`.
    ///
    /// This implementation allows a `Vector` to be converted into a `Point`
    /// by taking the `x`, `y`, and `z` components of the `Vector` and using
    /// them to create a new `Point`.
    ///
    /// # Arguments
    ///
    /// * `vector` - The `Vector` to be converted.
    ///
    /// # Example
    ///
    /// ```
    /// use openmodel::geometry::{Point, Vector};
    /// let v = Vector::new(1.0, 2.0, 3.0);
    /// let p: Point = v.into();
    /// assert_eq!(p.x, 1.0);
    /// assert_eq!(p.y, 2.0);
    /// assert_eq!(p.z, 3.0);
    /// ```
    fn from(vector: Vector) -> Self {
        Point {
            x: vector.x,
            y: vector.y,
            z: vector.z,
            data: Data::with_name("Point"),
        }
    }
}

impl AddAssign<&Vector> for Point {
    /// Adds the coordinates of another point to this point.
    ///
    /// # Arguments
    ///
    /// * `other` - The other point.
    ///
    /// # Example
    ///
    /// ```
    /// use openmodel::geometry::Point;
    /// use openmodel::geometry::Vector;
    /// let mut p = Point::new(1.0, 2.0, 3.0);
    /// let v = Vector::new(4.0, 5.0, 6.0);
    /// p += &v;
    /// assert_eq!(p.x, 5.0);
    /// assert_eq!(p.y, 7.0);
    /// assert_eq!(p.z, 9.0);
    /// ```
    fn add_assign(&mut self, other: &Vector) {
        self.x += other.x;
        self.y += other.y;
        self.z += other.z;
    }
}

impl Add<&Vector> for Point {
    type Output = Point;

    /// Adds the coordinates of a vector to this point and returns a new point.
    ///
    /// # Arguments
    ///
    /// * `other` - The vector.
    ///
    /// # Example
    ///
    /// ```
    /// use openmodel::geometry::{Point, Vector};
    /// let p = Point::new(1.0, 2.0, 3.0);
    /// let v = Vector::new(4.0, 5.0, 6.0);
    /// let p2 = p + &v;
    /// assert_eq!(p2.x, 5.0);
    /// assert_eq!(p2.y, 7.0);
    /// assert_eq!(p2.z, 9.0);
    /// ```
    fn add(self, other: &Vector) -> Point {
        Point {
            x: self.x + other.x,
            y: self.y + other.y,
            z: self.z + other.z,
            data: Data::with_name("Point"),
        }
    }
}

impl AddAssign<&Point> for Point {
    /// Adds the coordinates of another point to this point.
    ///
    /// # Arguments
    ///
    /// * `other` - The other point.
    ///
    /// # Example
    ///
    /// ```
    /// use openmodel::geometry::Point;
    /// let mut p = Point::new(1.0, 2.0, 3.0);
    /// let p2 = Point::new(4.0, 5.0, 6.0);
    /// p += &p2;
    /// assert_eq!(p.x, 5.0);
    /// assert_eq!(p.y, 7.0);
    /// assert_eq!(p.z, 9.0);
    /// ```
    fn add_assign(&mut self, other: &Point) {
        self.x += other.x;
        self.y += other.y;
        self.z += other.z;
    }
}

impl Add<&Point> for Point {
    type Output = Point;

    /// Adds the coordinates of a point to this point and returns a new point.
    ///
    /// # Arguments
    ///
    /// * `other` - The point.
    ///
    /// # Example
    ///
    /// ```
    /// use openmodel::geometry::Point;
    /// let p = Point::new(1.0, 2.0, 3.0);
    /// let p2 = Point::new(4.0, 5.0, 6.0);
    /// let p3 = p + &p2;
    /// assert_eq!(p3.x, 5.0);
    /// assert_eq!(p3.y, 7.0);
    /// assert_eq!(p3.z, 9.0);
    /// ```
    fn add(self, other: &Point) -> Point {
        Point {
            x: self.x + other.x,
            y: self.y + other.y,
            z: self.z + other.z,
            data: Data::with_name("Point"),
        }
    }
}

impl DivAssign<f64> for Point {
    /// Divides the coordinates of the point by a scalar.
    ///
    /// # Arguments
    ///
    /// * `factor` - The scalar to divide by.
    ///
    /// # Example
    ///
    /// ```
    /// use openmodel::geometry::Point;
    /// let mut p = Point::new(1.0, 2.0, 3.0);
    /// p /= 2.0;
    /// assert_eq!(p.x, 0.5);
    /// assert_eq!(p.y, 1.0);
    /// assert_eq!(p.z, 1.5);
    /// ```
    fn div_assign(&mut self, factor: f64) {
        self.x /= factor;
        self.y /= factor;
        self.z /= factor;
    }
}

impl Div<f64> for Point {
    type Output = Point;

    /// Divides the coordinates of the point by a scalar and returns a new point.
    ///
    /// # Arguments
    ///
    /// * `factor` - The scalar to divide by.
    ///
    /// # Example
    ///
    /// ```
    /// use openmodel::geometry::Point;
    /// let p = Point::new(1.0, 2.0, 3.0);
    /// let p2 = p / 2.0;
    /// assert_eq!(p2.x, 0.5);
    /// assert_eq!(p2.y, 1.0);
    /// assert_eq!(p2.z, 1.5);
    /// ```
    fn div(self, factor: f64) -> Point {
        let mut result = self;
        result /= factor;
        result
    }
}

impl IndexMut<usize> for Point {
    /// Provides mutable access to the coordinates of the point using the `[]` operator.
    ///
    /// # Arguments
    ///
    /// * `index` - The index of the coordinate (0 for x, 1 for y, 2 for z).
    ///
    /// # Panics
    ///
    /// Panics if the index is out of bounds.
    ///
    /// # Example
    ///
    /// ```
    /// use openmodel::geometry::Point;
    /// let mut p = Point::new(1.0, 2.0, 3.0);
    /// p[0] = 4.0;
    /// p[1] = 5.0;
    /// p[2] = 6.0;
    /// assert_eq!(p[0], 4.0);
    /// assert_eq!(p[1], 5.0);
    /// assert_eq!(p[2], 6.0);
    /// ```
    fn index_mut(&mut self, index: usize) -> &mut Self::Output {
        match index {
            0 => &mut self.x,
            1 => &mut self.y,
            2 => &mut self.z,
            _ => panic!("Index out of bounds"),
        }
    }
}

impl Index<usize> for Point {
    type Output = f64;

    /// Provides read-only access to the coordinates of the point using the `[]` operator.
    ///
    /// # Arguments
    ///
    /// * `index` - The index of the coordinate (0 for x, 1 for y, 2 for z).
    ///
    /// # Panics
    ///
    /// Panics if the index is out of bounds.
    ///
    /// # Example
    ///
    /// ```
    /// use openmodel::geometry::Point;
    /// let p = Point::new(1.0, 2.0, 3.0);
    /// assert_eq!(p[0], 1.0);
    /// assert_eq!(p[1], 2.0);
    /// assert_eq!(p[2], 3.0);
    /// ```
    fn index(&self, index: usize) -> &Self::Output {
        match index {
            0 => &self.x,
            1 => &self.y,
            2 => &self.z,
            _ => panic!("Index out of bounds"),
        }
    }
}

impl MulAssign<f64> for Point {
    /// Multiplies the coordinates of the point by a scalar.
    ///
    /// # Arguments
    ///
    /// * `factor` - The scalar to multiply by.
    ///
    /// # Example
    ///
    /// ```
    /// use openmodel::geometry::Point;
    /// let mut p = Point::new(1.0, 2.0, 3.0);
    /// p *= 2.0;
    /// assert_eq!(p.x, 2.0);
    /// assert_eq!(p.y, 4.0);
    /// assert_eq!(p.z, 6.0);
    /// ```
    fn mul_assign(&mut self, factor: f64) {
        self.x *= factor;
        self.y *= factor;
        self.z *= factor;
    }
}

impl Mul<f64> for Point {
    type Output = Point;

    /// Multiplies the coordinates of the point by a scalar and returns a new point.
    ///
    /// # Arguments
    ///
    /// * `factor` - The scalar to multiply by.
    ///
    /// # Example
    ///
    /// ```
    /// use openmodel::geometry::Point;
    /// let p = Point::new(1.0, 2.0, 3.0);
    /// let p2 = p * 2.0;
    /// assert_eq!(p2.x, 2.0);
    /// assert_eq!(p2.y, 4.0);
    /// assert_eq!(p2.z, 6.0);
    /// ```
    fn mul(self, factor: f64) -> Point {
        let mut result = self;
        result *= factor;
        result
    }
}

impl SubAssign<&Vector> for Point {
    /// Subtracts the coordinates of vector from this point.
    ///
    /// # Arguments
    ///
    /// * `vector` - The vector.
    ///
    /// # Example
    ///
    /// ```
    /// use openmodel::geometry::{Point, Vector};
    /// let mut p1 = Point::new(4.0, 5.0, 6.0);
    /// let v = Vector::new(1.0, 2.0, 3.0);
    /// p1 -= &v;
    /// assert_eq!(p1.x, 3.0);
    /// assert_eq!(p1.y, 3.0);
    /// assert_eq!(p1.z, 3.0);
    /// ```
    fn sub_assign(&mut self, vector: &Vector) {
        self.x -= vector.x;
        self.y -= vector.y;
        self.z -= vector.z;
    }
}

impl Sub<&Vector> for Point {
    type Output = Point;

    /// Subtracts the coordinates of a vector from this point and returns a new point.
    ///
    /// # Arguments
    ///
    /// * `other` - The vector.
    ///
    /// # Example
    ///
    /// ```
    /// use openmodel::geometry::{Point, Vector};
    /// let p = Point::new(4.0, 5.0, 6.0);
    /// let v = Vector::new(1.0, 2.0, 3.0);
    /// let v2 = p - &v;
    /// assert_eq!(v2.x, 3.0);
    /// assert_eq!(v2.y, 3.0);
    /// assert_eq!(v2.z, 3.0);
    /// ```
    fn sub(self, vector: &Vector) -> Point {
        Point {
            x: self.x - vector.x,
            y: self.y - vector.y,
            z: self.z - vector.z,
            data: Data::with_name("Point"),
        }
    }
}


impl SubAssign<&Point> for Point {
    /// Subtracts the coordinates of point from this point.
    ///
    /// # Arguments
    ///
    /// * `point` - The point.
    ///
    /// # Example
    ///
    /// ```
    /// use openmodel::geometry::{Point, Vector};
    /// let mut p1 = Point::new(4.0, 5.0, 6.0);
    /// let p2 = Point::new(1.0, 2.0, 3.0);
    /// p1 -= &p2;
    /// assert_eq!(p1.x, 3.0);
    /// assert_eq!(p1.y, 3.0);
    /// assert_eq!(p1.z, 3.0);
    /// ```
    fn sub_assign(&mut self, point: &Point) {
        self.x -= point.x;
        self.y -= point.y;
        self.z -= point.z;
    }
}

impl Sub<&Point> for Point {
    type Output = Point;

    /// Subtracts the coordinates of a point from this point and returns a new point.
    ///
    /// # Arguments
    ///
    /// * `other` - The vector.
    ///
    /// # Example
    ///
    /// ```
    /// use openmodel::geometry::{Point, Vector};
    /// let p = Point::new(4.0, 5.0, 6.0);
    /// let p2 = Point::new(1.0, 2.0, 3.0);
    /// let p3 = p - &p2;
    /// assert_eq!(p3.x, 3.0);
    /// assert_eq!(p3.y, 3.0);
    /// assert_eq!(p3.z, 3.0);
    /// ```
    fn sub(self, point: &Point) -> Point {
        Point {
            x: self.x - point.x,
            y: self.y - point.y,
            z: self.z - point.z,
            data: Data::with_name("Point"),
        }
    }
}

impl PartialEq for Point {
    /// Checks if two points are equal.
    ///
    /// # Arguments
    ///
    /// * `other` - The other point.
    ///
    /// # Example
    ///
    /// ```
    /// use openmodel::geometry::Point;
    /// let p1 = Point::new(1.0, 2.0, 3.0);
    /// let p2 = Point::new(1.0, 2.0, 3.0);
    /// assert_eq!(p1, p2);
    /// ```
    fn eq(&self, other: &Self) -> bool {
        self.x == other.x && self.y == other.y && self.z == other.z
    }
}

impl PartialOrd for Point {
    /// Compares the distances of two points from the origin.
    ///
    /// # Arguments
    ///
    /// * `other` - The other point.
    ///
    /// # Example
    ///
    /// ```
    /// use openmodel::geometry::Point;
    /// let p1 = Point::new(1.0, 2.0, 3.0);
    /// let p2 = Point::new(4.0, 5.0, 6.0);
    /// assert!(p1 < p2);
    /// ```
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(
            self.distance(&Point::default())
                .partial_cmp(&other.distance(&Point::default()))?,
        )
    }
}

impl fmt::Display for Point {
    /// Log point.
    ///
    /// # Example
    ///
    /// ```
    /// use openmodel::geometry::Point;
    /// let p = Point::new(0.0, 0.0, 1.0);
    /// println!("{}", p);
    /// ```
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Point({}, {}, {})", self.x, self.y, self.z)
    }
}

/// Implementation for Point serialization
impl Point {
    /// Create a structured JSON representation similar to COMPAS
    ///
    /// # Arguments
    ///
    /// * `minimal` - If true, only includes dtype and data fields
    ///
    /// # Returns
    ///
    /// A JSON value with the point's data in COMPAS format
    ///
    /// # Example
    ///
    /// ```
    /// use openmodel::geometry::Point;
    /// let point = Point::new(1.0, 2.0, 3.0);
    /// let json = point.to_json_data(false);
    /// ```
    pub fn to_json_data(&self, minimal: bool) -> serde_json::Value {
        let geometric_data = serde_json::json!({
            "x": self.x,
            "y": self.y,
            "z": self.z
        });
        
        self.data.to_json_data("openmodel.geometry/Point", geometric_data, minimal)
    }
}

/// Implementation of HasJsonData trait for ultra-simple API
impl HasJsonData for Point {
    fn to_json_data(&self, minimal: bool) -> Value {
        self.to_json_data(minimal)
    }
}

/// Implementation of FromJsonData trait for direct deserialization
impl FromJsonData for Point {
    /// Create Point directly from JSON data - no casting needed!
    /// 
    /// # Example
    /// ```
    /// use openmodel::geometry::Point;
    /// use openmodel::common::FromJsonData;
    /// use serde_json::json;
    /// 
    /// let json = json!({"data": {"x": 1.0, "y": 2.0, "z": 3.0}, "name": "MyPoint"});
    /// let point = Point::from_json_data(&json).unwrap();
    /// assert_eq!(point.x, 1.0);
    /// ```
    fn from_json_data(data: &Value) -> Option<Self> {
        let x = data["data"]["x"].as_f64()?;
        let y = data["data"]["y"].as_f64()?;
        let z = data["data"]["z"].as_f64()?;
        
        let mut point = Point::new(x, y, z);
        
        // Apply all data fields from JSON
        point.data.apply_from_json(data);
        
        Some(point)
    }
}