use crate::geometry::Point;
use crate::geometry::Vector;
use crate::common::{JsonSerializable, FromJsonData};
use serde::{Deserialize, Serialize};
use std::ops::{Add, AddAssign, Sub, SubAssign};
use crate::common::Data;
use std::fmt;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Plane {
    /// The origin point.
    pub origin: Point,
    /// The x-axis.
    pub xaxis: Vector,
    /// The x-axis.
    pub yaxis: Vector,
    /// The x-axis.
    pub zaxis: Vector,
    /// The normal x coordinate.
    pub a : f32,
    /// The normal y coordinate.
    pub b : f32,
    /// The normal z coordinate.
    pub c : f32,
    /// The plane offset from origin.
    pub d : f32,
    /// Associated data - guid and name.
    pub data: Data,
}


impl Plane{
    /// Creates a new `Line` with default `Data`.
    ///
    /// # Arguments
    ///
    /// * `x0` - The x components of the start point.
    /// * `y0` - The y components of the start point.
    /// * `z0` - The z components of the start point.
    /// * `x1` - The x components of the end point.
    /// * `y1` - The y components of the end point.
    /// * `z1` - The z components of the end point.
    ///
    /// # Example
    ///
    /// ```
    /// use openmodel::geometry::Line;
    /// let line = Line::new(0.0, 0.0, 0.0, 0.0, 0.0, 1.0);
    /// assert_eq!(line.x0, 0.0);
    /// assert_eq!(line.y0, 0.0);
    /// assert_eq!(line.z0, 0.0);
    /// assert_eq!(line.x1, 0.0);
    /// assert_eq!(line.y1, 0.0);
    /// assert_eq!(line.z1, 1.0);
    /// 
    /// ```
    pub fn new(origin: Point, xaxis: Vector, yaxis: Vector) -> Self {
        let zaxis = Vector::cross(&xaxis, &yaxis);
        let a = zaxis.x;
        let b = zaxis.y;
        let c = zaxis.z;
        let d = -a * origin.x - b * origin.y - c * origin.z;
        Plane {
            origin,
            xaxis,
            yaxis,
            zaxis,
            a,
            b,
            c,
            d,
            data: Data::with_name("Plane")
        }
    }

    /// Creates a new `Plane` with a specified name for `Data`.
    ///
    /// # Arguments
    ///
    /// * `name` - The name for the `Data`.
    /// * `origin` - The origin point.
    /// * `xaxis` - The x-axis.
    /// * `yaxis` - The y-axis.
    ///
    /// # Example
    ///
    /// ```
    /// use openmodel::geometry::{Plane, Point, Vector};
    /// let plane = Plane::with_name("MyPlane".to_string(), Point::new(0.0, 0.0, 0.0), Vector::new(1.0, 0.0, 0.0), Vector::new(0.0, 1.0, 0.0));
    /// assert_eq!(plane.origin.x, 0.0);
    /// assert_eq!(plane.origin.y, 0.0);
    /// assert_eq!(plane.origin.z, 0.0);
    /// assert_eq!(plane.xaxis.x, 1.0);
    /// assert_eq!(plane.xaxis.y, 0.0);
    /// assert_eq!(plane.xaxis.z, 0.0);
    /// assert_eq!(plane.yaxis.x, 0.0);
    /// assert_eq!(plane.yaxis.y, 1.0);
    /// assert_eq!(plane.yaxis.z, 0.0);
    /// assert_eq!(plane.zaxis.x, 0.0);
    /// assert_eq!(plane.zaxis.y, 0.0);
    /// assert_eq!(plane.zaxis.z, 1.0);
    /// assert_eq!(plane.a, 0.0);
    /// assert_eq!(plane.b, 0.0);
    /// assert_eq!(plane.c, 1.0);
    /// assert_eq!(plane.d, 0.0);
    /// ```
    pub fn with_name(name: String, origin: Point, xaxis: Vector, yaxis: Vector) -> Self {
        let zaxis = Vector::cross(&xaxis, &yaxis);
        let a = zaxis.x;
        let b = zaxis.y;
        let c = zaxis.z;
        let d = -a * origin.x - b * origin.y - c * origin.z;
        Plane {
            origin,
            xaxis,
            yaxis,
            zaxis,
            a,
            b,
            c,
            d,
            data: Data::with_name(&name)
        }
    }

    /// Creates a new `Plane` from a point and a normal vector.
    ///
    /// # Arguments
    ///
    /// * `point` - A point on the plane.
    /// * `normal` - The normal vector of the plane.
    ///
    /// # Example
    ///
    /// ```
    /// use openmodel::geometry::{Plane, Point, Vector};
    /// let point = Point::new(1.0, 2.0, 3.0);
    /// let normal = Vector::new(0.0, 0.0, 1.0);
    /// let plane = Plane::from_point_normal(&point, &normal);
    /// assert_eq!(plane.origin.x, 1.0);
    /// assert_eq!(plane.origin.y, 2.0);
    /// assert_eq!(plane.origin.z, 3.0);
    /// assert_eq!(plane.zaxis.x, 0.0);
    /// assert_eq!(plane.zaxis.y, 0.0);
    /// assert_eq!(plane.zaxis.z, 1.0);
    /// assert_eq!(plane.d, -3.0); // -(0*1 + 0*2 + 1*3)
    /// ```
    /// 
    /// # Panics
    /// 
    /// Panics if the normal vector has zero length.
    pub fn from_point_normal(point: &Point, normal: &Vector) -> Self {
        // Clone and unitize the normal vector
        let mut zaxis = normal.clone();
        if !zaxis.unitize() {
            panic!("Normal vector cannot be zero length");
        }

        // Create two perpendicular vectors to form a coordinate system
        // Choose an initial vector that's not parallel to the normal
        let initial = if zaxis.x.abs() < 0.9 {
            Vector::new(1.0, 0.0, 0.0)  // Use x-axis if normal is not too close to x-axis
        } else {
            Vector::new(0.0, 1.0, 0.0)  // Use y-axis if normal is close to x-axis
        };

        // Get first perpendicular vector (x-axis of the plane)
        let mut xaxis = initial.cross(&zaxis);
        xaxis.unitize();

        // Get second perpendicular vector (y-axis of the plane)
        let mut yaxis = zaxis.cross(&xaxis);
        yaxis.unitize();

        // Calculate plane equation coefficients (Ax + By + Cz + D = 0)
        let a = zaxis.x;
        let b = zaxis.y;
        let c = zaxis.z;
        let d = -(a * point.x + b * point.y + c * point.z);

        Plane {
            origin: point.clone(),
            xaxis,
            yaxis,
            zaxis,
            a,
            b,
            c,
            d,
            data: Data::with_name("Plane")
        }
    }

    /// Creates a new `Plane` from a collection of points.
    ///
    /// For 2 points: Creates a plane where the line between points is the x-axis.
    /// For 3+ points: Computes an average cross product from consecutive point triplets.
    ///
    /// # Arguments
    ///
    /// * `points` - A slice of points (minimum 2 points required).
    ///
    /// # Example
    ///
    /// ```
    /// use openmodel::geometry::{Plane, Point};
    /// let points = vec![
    ///     Point::new(0.0, 0.0, 0.0),
    ///     Point::new(1.0, 0.0, 0.0),
    ///     Point::new(0.0, 1.0, 0.0)
    /// ];
    /// let plane = Plane::plane_from_points(&points);
    /// assert_eq!(plane.origin.x, 0.0);
    /// assert_eq!(plane.origin.y, 0.0);
    /// assert_eq!(plane.origin.z, 0.0);
    /// ```
    /// 
    /// # Panics
    /// 
    /// Panics if fewer than 2 points are provided.
    pub fn plane_from_points(points: &[Point]) -> Self {
        
        if points.len() == 0{
            return Plane::default();
        }else if points.len() == 1 {
            return Plane::new(points[0].clone(), Vector::new(1.0, 0.0, 0.0), Vector::new(0.0, 1.0, 0.0));
        }else if points.len() >= 3 {
            
            // For three or more points, compute an average cross product for the plane normal
            let mut normal_sum = Vector::new(0.0, 0.0, 0.0);
            let mut normal_count = 0;
            
            // Calculate cross products from consecutive point triplets
            for i in 0..(points.len() - 2) {
                let v1 = Vector::new(
                    points[i + 1].x - points[i].x,
                    points[i + 1].y - points[i].y,
                    points[i + 1].z - points[i].z,
                );
                let v2 = Vector::new(
                    points[i + 2].x - points[i + 1].x,
                    points[i + 2].y - points[i + 1].y,
                    points[i + 2].z - points[i + 1].z,
                );
                
                let cross = v1.cross(&v2);
                // Only add non-zero cross products (skip colinear segments)
                if cross.length() > 1e-10 {
                    normal_sum.x += cross.x;
                    normal_sum.y += cross.y;
                    normal_sum.z += cross.z;
                    normal_count += 1;
                }
            }
            
            if normal_count > 0 {
                // Average the normals
                normal_sum.x /= normal_count as f32;
                normal_sum.y /= normal_count as f32;
                normal_sum.z /= normal_count as f32;
                
                // Create plane from first point and averaged normal
                Self::from_point_normal(&points[0], &normal_sum)
            } else {
                // All segments are colinear, fall back to 2-point logic
                Self::plane_from_two_points(&points[0], &points[1])
            }
        } else {
            // For two points, guess the y-axis since the first line is x-axis
            Self::plane_from_two_points(&points[0], &points[1])
        }
    }
    
    /// Helper function to create a plane from two points
    fn plane_from_two_points(p1: &Point, p2: &Point) -> Self {
        // The line from p1 to p2 becomes the x-axis
        let x_axis = Vector::new(
            p2.x - p1.x,
            p2.y - p1.y,
            p2.z - p1.z,
        );
        
        // Guess a reasonable y-axis by choosing a vector not parallel to x-axis
        let y_guess = if x_axis.z.abs() < 0.9 {
            Vector::new(0.0, 0.0, 1.0)  // Use world Z if x-axis is not too close to Z
        } else {
            Vector::new(0.0, 1.0, 0.0)  // Use world Y if x-axis is close to Z
        };
        
        // Get perpendicular y-axis via cross product
        let z_axis = x_axis.cross(&y_guess);
        let y_axis = z_axis.cross(&x_axis);
        
        // Compute plane normal (z-axis)
        let normal = x_axis.cross(&y_axis);
        
        // Create plane from first point and computed normal
        Self::from_point_normal(p1, &normal)

    }

    


}

impl Default for Plane {
    /// Creates a zero length `Plane`.
    ///
    /// # Example
    ///
    /// ```
    /// use openmodel::geometry::Plane;
    /// let plane = Plane::default();
    /// assert_eq!(plane.origin.x, 0.0);
    /// assert_eq!(plane.origin.y, 0.0);
    /// assert_eq!(plane.origin.z, 0.0);
    /// assert_eq!(plane.xaxis.x, 1.0);
    /// assert_eq!(plane.xaxis.y, 0.0);
    /// assert_eq!(plane.xaxis.z, 0.0);
    /// assert_eq!(plane.yaxis.x, 0.0);
    /// assert_eq!(plane.yaxis.y, 1.0);
    /// assert_eq!(plane.yaxis.z, 0.0);
    /// assert_eq!(plane.zaxis.x, 0.0);
    /// assert_eq!(plane.zaxis.y, 0.0);
    /// assert_eq!(plane.zaxis.z, 1.0);
    /// assert_eq!(plane.a, 0.0);
    /// assert_eq!(plane.b, 0.0);
    /// assert_eq!(plane.c, 1.0);
    /// assert_eq!(plane.d, 0.0);
    /// ```
    fn default() -> Self {
        Plane {
            origin: Point::new(0.0, 0.0, 0.0),
            xaxis: Vector::new(1.0, 0.0, 0.0),
            yaxis: Vector::new(0.0, 1.0, 0.0),
            zaxis: Vector::new(0.0, 0.0, 1.0),
            a: 0.0,
            b: 0.0,
            c: 1.0,
            d: 0.0,
            data: Data::with_name("Plane"),
        }
    }
}




impl Add<&Vector> for Plane {
    type Output = Plane;

    /// Adds the coordinates of a vector to this plane and returns a new plane.
    ///
    /// # Arguments
    ///
    /// * `other` - The vector.
    ///
    /// # Example
    ///
    /// ```
    /// use openmodel::geometry::{Plane, Vector};
    /// let plane0 = Plane::default();
    /// let v = Vector::new(0.0, 0.0, 1.0);
    /// let plane1 = plane0 + &v;
    /// assert_eq!(plane1.origin.x, 0.0);
    /// assert_eq!(plane1.origin.y, 0.0);
    /// assert_eq!(plane1.origin.z, 1.0);
    /// ```
    fn add(self, other: &Vector) -> Plane {
        Plane {
            origin: self.origin + other,
            xaxis: self.xaxis,
            yaxis: self.yaxis,
            zaxis: self.zaxis,
            a: self.a,
            b: self.b,
            c: self.c,
            d: self.d,
            data: Data::with_name("Plane"),
        }
    }
}


impl AddAssign<&Vector> for Plane {
    /// Adds the coordinates of a vector to this plane.
    ///
    /// # Arguments
    ///
    /// * `vector` - traslation vector.
    ///
    /// # Example
    ///
    /// ```
    /// use openmodel::geometry::Plane;
    /// use openmodel::geometry::Vector;
    /// let mut plane = Plane::default();
    /// let v = Vector::new(1.0, 1.0, 1.0);
    /// plane += &v;
    /// assert_eq!(plane.origin.x, 1.0);
    /// assert_eq!(plane.origin.y, 1.0);
    /// assert_eq!(plane.origin.z, 1.0);
    /// ```
    fn add_assign(&mut self, vector: &Vector) {
        self.origin += vector;
    }
}


impl Sub<&Vector> for Plane {
    type Output = Plane;

    /// Subtracts the coordinates of a vector to this plane and returns a new plane.
    ///
    /// # Arguments
    ///
    /// * `other` - The vector.
    ///
    /// # Example
    ///
    /// ```
    /// use openmodel::geometry::{Plane, Vector};
    /// let plane0 = Plane::default();
    /// let v = Vector::new(0.0, 0.0, 1.0);
    /// let plane1 = plane0 - &v;
    /// assert_eq!(plane1.origin.x, 0.0);
    /// assert_eq!(plane1.origin.y, 0.0);
    /// assert_eq!(plane1.origin.z, -1.0);
    /// ```
    fn sub(self, vector: &Vector) -> Plane {
        Plane {
            origin: self.origin - vector,
            xaxis: self.xaxis,
            yaxis: self.yaxis,
            zaxis: self.zaxis,
            a: self.a,
            b: self.b,
            c: self.c,
            d: self.d,
            data: Data::with_name("Plane"),
        }
    }
}


impl SubAssign<&Vector> for Plane {
    /// Subtracts the coordinates of a vector to this plane.
    ///
    /// # Arguments
    ///
    /// * `vector` - traslation vector.
    ///
    /// # Example
    ///
    /// ```
    /// use openmodel::geometry::Plane;
    /// use openmodel::geometry::Vector;
    /// let mut plane = Plane::default();
    /// let v = Vector::new(1.0, 1.0, 1.0);
    /// plane -= &v;
    /// assert_eq!(plane.origin.x, -1.0);
    /// assert_eq!(plane.origin.y, -1.0);
    /// assert_eq!(plane.origin.z, -1.0);
    /// ```
    fn sub_assign(&mut self, vector: &Vector) {
        self.origin -= vector;
    }
}


impl fmt::Display for Plane{
    /// Log color.
    /// # Example
    ///
    /// ```
    /// use openmodel::geometry::Plane;
    /// let plane = Plane::default();
    /// println!("{}", plane);
    /// ```
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result{
        write!(f, "Plane {{ origin: {}, xaxis {}, yaxis: {}, zaxis: {}, Data: {} }}", self.origin, self.xaxis, self.yaxis, self.zaxis, self.data)
    }
}

/// Implementation of DataObject trait for Plane to support COMPAS-style JSON serialization
impl Plane {
    /// Get the type identifier for polymorphic deserialization
    pub fn dtype(&self) -> &'static str {
        "openmodel.geometry/Plane"
    }
    
    /// Get the object's geometric data for serialization
    pub fn geometric_data(&self) -> serde_json::Value {
        serde_json::json!({
            "origin": {
                "x": self.origin.x,
                "y": self.origin.y,
                "z": self.origin.z
            },
            "xaxis": {
                "x": self.xaxis.x,
                "y": self.xaxis.y,
                "z": self.xaxis.z
            },
            "yaxis": {
                "x": self.yaxis.x,
                "y": self.yaxis.y,
                "z": self.yaxis.z
            },
            "zaxis": {
                "x": self.zaxis.x,
                "y": self.zaxis.y,
                "z": self.zaxis.z
            },
            "a": self.a,
            "b": self.b,
            "c": self.c,
            "d": self.d
        })
    }
    
    /// Get the object's GUID
    pub fn guid(&self) -> Uuid {
        self.data.guid()
    }
    
    /// Get the object's name
    pub fn name(&self) -> &str {
        self.data.name()
    }
    
    /// Set the object's name
    pub fn set_name(&mut self, name: &str) {
        self.data.set_name(name);
    }
    
    /// Create a structured JSON representation similar to COMPAS
    pub fn to_json_data(&self, minimal: bool) -> serde_json::Value {
        self.data.to_json_data(self.dtype(), self.geometric_data(), minimal)
    }
}

// JSON serialization support
impl JsonSerializable for Plane {
    fn to_json_value(&self) -> serde_json::Value {
        self.to_json_data(false)
    }
}

impl FromJsonData for Plane {
    fn from_json_data(data: &serde_json::Value) -> Option<Self> {
        serde_json::from_value(data.clone()).ok()
    }
}