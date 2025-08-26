use crate::geometry::{Point, Mesh};
use crate::geometry::Vector;
use crate::common::Data;
use crate::common::{JsonSerializable, FromJsonData};
use crate::primitives::Xform;

use serde::{Deserialize, Serialize};
use std::ops::{Add, AddAssign, Div, DivAssign, Index, IndexMut, Mul, MulAssign, Sub, SubAssign};
use std::fmt;
// use std::f64::consts::PI;  // Not needed

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Arrow {
    /// The x coordinate of the start point.
    pub x0: f32,
    /// The y coordinate of the start point.
    pub y0: f32,
    /// The z coordinate of the start point.
    pub z0: f32,
    /// The x coordinate of the end point.
    pub x1: f32,
    /// The y coordinate of the end point.
    pub y1: f32,
    /// The z coordinate of the end point.
    pub z1: f32,
    /// The data associated with the Arrow (includes color and thickness).
    pub data: Data,
    /// Mesh for visualization (pipe)
    #[serde(skip)]
    pub mesh: Option<Mesh>,
}

impl Arrow{
    /// Creates a new `Arrow` with default `Data`.
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
    /// use openmodel::geometry::Arrow;
    /// let Arrow = Arrow::new(0.0, 0.0, 0.0, 0.0, 0.0, 1.0);
    /// assert_eq!(Arrow.x0, 0.0);
    /// assert_eq!(Arrow.y0, 0.0);
    /// assert_eq!(Arrow.z0, 0.0);
    /// assert_eq!(Arrow.x1, 0.0);
    /// assert_eq!(Arrow.y1, 0.0);
    /// assert_eq!(Arrow.z1, 1.0);
    /// 
    /// ```
    pub fn new(x0: f32, y0: f32, z0:f32, x1: f32, y1: f32, z1:f32) -> Self {
        Arrow {
            x0,
            y0,
            z0,
            x1,
            y1,
            z1,
            data: Data::with_name("Arrow"),
            mesh: None,
        }
    }

    /// Creates a new `Arrow` with a specified name for `Data`.
    ///
    /// # Arguments
    ///
    /// * `name` - The name for the `Data`.
    /// * `x0` - The x component of the start point.
    /// * `y0` - The y component of the start point.
    /// * `z0` - The z component of the start point.
    /// * `x1` - The x component of the end point.
    /// * `y1` - The y component of the end point.
    /// * `z1` - The z component of the end point.
    ///
    /// # Example
    ///
    /// ```
    /// use openmodel::geometry::Arrow;
    /// let Arrow = Arrow::with_name("MyArrow".to_string(), 0.0, 0.0, 0.0, 0.0, 0.0, 1.0);
    /// assert_eq!(Arrow.x0, 0.0);
    /// assert_eq!(Arrow.y0, 0.0);
    /// assert_eq!(Arrow.z0, 0.0);
    /// assert_eq!(Arrow.x1, 0.0);
    /// assert_eq!(Arrow.y1, 0.0);
    /// assert_eq!(Arrow.z1, 1.0);
    /// ```
    pub fn with_name(name: String, x0: f32, y0: f32, z0: f32, x1: f32, y1: f32, z1: f32) -> Self {
        Arrow {
            x0,
            y0,
            z0,
            x1,
            y1,
            z1,
            data: Data::with_name(&name),
            mesh: None,
        }
    }

    /// Creates a new `Arrow` from start ´Point´ and end `Point`.
    ///
    /// # Arguments
    ///
    /// * `p0` - The start point.
    /// * `p1` - The end point.
    /// # Example
    ///
    /// ```
    /// use openmodel::geometry::Point;
    /// use openmodel::geometry::Arrow;
    /// let p0 = Point::new(0.0, 0.0, 0.0);
    /// let p1 = Point::new(0.0, 0.0, 1.0);
    /// let Arrow = Arrow::from_points(&p0, &p1);
    /// assert_eq!(Arrow.x0, 0.0);
    /// assert_eq!(Arrow.y0, 0.0);
    /// assert_eq!(Arrow.z0, 0.0);
    /// assert_eq!(Arrow.x1, 0.0);
    /// assert_eq!(Arrow.y1, 0.0);
    /// assert_eq!(Arrow.z1, 1.0);
    /// ```
    pub fn from_points(p0: &Point, p1: &Point) -> Self{
        Arrow {
            x0:p0.x,
            y0:p0.y,
            z0:p0.z,
            x1:p1.x,
            y1:p1.y,
            z1:p1.z,
            data: Data::with_name("Arrow"),
            mesh: None,
        }
    }

    /// Computes the length of the Arrow.
    ///
    ///
    /// # Example
    ///
    /// ```
    /// use openmodel::geometry::Arrow;
    /// let Arrow = Arrow::new(0.0, 0.0, 0.0, 0.0, 0.0, 1.0);
    /// let length = Arrow.length();
    /// assert_eq!(length, 1.0);
    /// ```
    pub fn length(&self) -> f32 {
        ((self.x0 - self.x1).powi(2) + (self.y0 - self.y1).powi(2) + (self.z0 - self.z1).powi(2))
            .sqrt()
    }

    /// Updates the mesh representation using thickness from data.
    /// 
    /// # Returns
    /// A reference to self for method chaining.
    pub fn update_mesh(&mut self) -> &mut Self {
        // Get thickness from data
        let thickness = self.data.get_thickness();
        
        // Create start and end points for the pipe
        let start = Point::new(self.x0, self.y0, self.z0);
        let end = Point::new(self.x1, self.y1, self.z1);
        
        // Use fixed 8 sides for the pipe cross-section
        // Generate the mesh
        self.mesh = Some(Mesh::create_pipe(start, end, thickness));
        
        // If the Arrow has a color, apply it to the mesh
        if self.data.has_color() {
            if let Some(mesh) = &mut self.mesh {
                mesh.data.set_color(self.data.get_color());
            }
        }
        
        self
    }

    /// Gets the mesh representation of this Arrow as a pipe.
    /// If the mesh doesn't exist, creates it first.
    /// 
    /// # Returns
    /// An Option containing a reference to the Mesh if it exists.
    /// 
    /// # Example
    /// 
    /// ```
    /// use openmodel::geometry::Arrow;
    /// let mut Arrow = Arrow::new(0.0, 0.0, 0.0, 0.0, 0.0, 1.0);
    /// let mesh = Arrow.get_mesh();
    /// assert!(mesh.is_some());
    /// ```
    pub fn get_mesh(&mut self) -> Option<&Mesh> {
        // Create the mesh if it doesn't exist yet
        if self.mesh.is_none() {
            self.update_mesh();
        }
        
        self.mesh.as_ref()
    }

    /// Returns a transform that maps the canonical unit pipe (aligned to +Z, length=1, radius=0.5,
    /// centered at the origin with z in [-0.5, +0.5]) onto this Arrow segment.
    /// Uses the JSON transformation matrix directly if available, otherwise falls back to coordinate-based calculation.
    pub fn to_pipe_transform(&self) -> Option<Xform> {
        // Check if we have a non-identity transformation matrix in the JSON data
        let json_transform = self.data.transformation();
        let is_identity = json_transform.iter().enumerate().all(|(i, &val)| {
            (i % 5 == 0 && (val - 1.0).abs() < f32::EPSILON) || // Diagonal elements are 1.0
            (i % 5 != 0 && val.abs() < f32::EPSILON) // Non-diagonal elements are 0.0
        });

        if !is_identity {
            // Use the JSON transformation matrix directly - much more efficient!
            return Some(Xform { m: *json_transform });
        }

        // Fallback: rebuild from coordinates (for backward compatibility)
        let p0 = Point::new(self.x0, self.y0, self.z0);
        let p1 = Point::new(self.x1, self.y1, self.z1);

        // Direction and length
        let dir = Vector::new(p1.x - p0.x, p1.y - p0.y, p1.z - p0.z);
        let len = dir.length();
        let eps = 1e-9;
        if len < eps { return None; }

        let axis = dir.normalize();
        let z_axis = Vector::new(0.0, 0.0, 1.0);

        // Rotation aligning +Z to the Arrow direction
        let mut dot = axis.dot(&z_axis);
        if dot > 1.0 { dot = 1.0; } else if dot < -1.0 { dot = -1.0; }
        let rotation = if (dot - 1.0).abs() < eps {
            Xform::identity()
        } else if (dot + 1.0).abs() < eps {
            // +Z to -Z: 180° around any axis perpendicular to Z (choose X)
            Xform::rotation_x(std::f32::consts::PI)
        } else {
            let rot_axis = z_axis.cross(&axis).normalize();
            let angle = dot.acos();
            Xform::rotation(&rot_axis, angle)
        };

        // Midpoint translation
        let midpoint = Point::new(
            (p0.x + p1.x) * 0.5,
            (p0.y + p1.y) * 0.5,
            (p0.z + p1.z) * 0.5,
        );
        let translation = Xform::translation(midpoint.x, midpoint.y, midpoint.z);

        // Non-uniform scale: XY = 1.0 (unit pipe geometry radius is unused for thickness), Z = length
        let scale = Xform::scaling(1.0, 1.0, len);

        // Compose T * R * S (scale → rotate → translate)
        Some(translation * rotation * scale)
    }
}

impl Default for Arrow{
    /// Creates a default `Arrow` as a vertical Arrow.
    ///
    /// # Example
    ///
    /// ```
    /// use openmodel::geometry::Arrow;
    /// let l = Arrow::default();
    /// ```
    fn default() -> Self {
        Arrow {
            x0: 0.0,
            y0: 0.0,
            z0: 0.0,
            x1: 0.0,
            y1: 0.0,
            z1: 1.0,
            data: Data::with_name("Arrow"),
            mesh: None,
        }
    }
}

impl Add<&Vector> for Arrow {
    type Output = Arrow;

    /// Adds the coordinates of a vector to this Arrow and returns a new Arrow.
    ///
    /// # Arguments
    ///
    /// * `other` - The vector.
    ///
    /// # Example
    ///
    /// ```
    /// use openmodel::geometry::{Arrow, Vector};
    /// let Arrow0 = Arrow::new(0.0, 1.0, 2.0, 3.0, 4.0, 5.0);
    /// let v = Vector::new(0.0, 0.0, 1.0);
    /// let Arrow1 = Arrow0 + &v;
    /// assert_eq!(Arrow1.x0, 0.0);
    /// assert_eq!(Arrow1.y0, 1.0);
    /// assert_eq!(Arrow1.z0, 3.0);
    /// assert_eq!(Arrow1.x1, 3.0);
    /// assert_eq!(Arrow1.y1, 4.0);
    /// assert_eq!(Arrow1.z1, 6.0);
    /// ```
    fn add(self, other: &Vector) -> Arrow {
        Arrow {
            x0: self.x0 + other.x,
            y0: self.y0 + other.y,
            z0: self.z0 + other.z,
            x1: self.x1 + other.x,
            y1: self.y1 + other.y,
            z1: self.z1 + other.z,
            data: Data::with_name("Arrow"),
            mesh: None,
        }
    }
}

impl AddAssign<&Vector> for Arrow {
    /// Adds the coordinates of a vector to this Arrow.
    ///
    /// # Arguments
    ///
    /// * `vector` - traslation vector.
    ///
    /// # Example
    ///
    /// ```
    /// use openmodel::geometry::Arrow;
    /// use openmodel::geometry::Vector;
    /// let mut Arrow = Arrow::new(0.0, 0.0, 0.0, 0.0, 0.0, 1.0);
    /// let v = Vector::new(1.0, 1.0, 1.0);
    /// Arrow += &v;
    /// assert_eq!(Arrow.x0, 1.0);
    /// assert_eq!(Arrow.y0, 1.0);
    /// assert_eq!(Arrow.z0, 1.0);
    /// assert_eq!(Arrow.x1, 1.0);
    /// assert_eq!(Arrow.y1, 1.0);
    /// assert_eq!(Arrow.z1, 2.0);
    /// ```
    fn add_assign(&mut self, vector: &Vector) {
        self.x0 += vector.x;
        self.y0 += vector.y;
        self.z0 += vector.z;
        self.x1 += vector.x;
        self.y1 += vector.y;
        self.z1 += vector.z;
    }
}

impl Div<f32> for Arrow {
    type Output = Arrow;

    /// Divides the coordinates of the Arrow by a scalar and returns a new Arrow.
    ///
    /// # Arguments
    ///
    /// * `factor` - The scalar to divide by.
    ///
    /// # Example
    ///
    /// ```
    /// use openmodel::geometry::Arrow;
    /// let Arrow0 = Arrow::new(0.0, 0.0, 0.0, 0.0, 0.0, 1.0);
    /// let Arrow1 = Arrow0 / 2.0;
    /// assert_eq!(Arrow1.x0, 0.0);
    /// assert_eq!(Arrow1.y0, 0.0);
    /// assert_eq!(Arrow1.z0, 0.0);
    /// assert_eq!(Arrow1.x1, 0.0);
    /// assert_eq!(Arrow1.y1, 0.0);
    /// assert_eq!(Arrow1.z1, 0.5);
    /// ```
    fn div(self, factor: f32) -> Arrow {
        let mut result = self;
        result /= factor;
        result
    }
}

impl DivAssign<f32> for Arrow {
    /// Divides the coordinates of the Arrow by a scalar.
    ///
    /// # Arguments
    ///
    /// * `factor` - The scalar to divide by.
    ///
    /// # Example
    ///
    /// ```
    /// use openmodel::geometry::Arrow;
    /// let mut Arrow = Arrow::new(0.0, 0.0, 0.0, 0.0, 0.0, 1.0);
    /// Arrow /= 2.0;
    /// assert_eq!(Arrow.x0, 0.0);
    /// assert_eq!(Arrow.y0, 0.0);
    /// assert_eq!(Arrow.z0, 0.0);
    /// assert_eq!(Arrow.x1, 0.0);
    /// assert_eq!(Arrow.y1, 0.0);
    /// assert_eq!(Arrow.z1, 0.5);
    /// ```
    fn div_assign(&mut self, factor: f32) {
        self.x0 /= factor;
        self.y0 /= factor;
        self.z0 /= factor;
        self.x1 /= factor;
        self.y1 /= factor;
        self.z1 /= factor;
    }
}

impl Index<usize> for Arrow {
    type Output = f32;

    /// Provides read-only access to the coordinates of the point using the `[]` operator.
    ///
    /// # Arguments
    ///
    /// * `index` - The index of the coordinate (0 for x0, 1 for y0, 2 for z0, 3 for x1, 4 for y1, 5 for z1).
    ///
    /// # Panics
    ///
    /// Panics if the index is out of bounds.
    ///
    /// # Example
    ///
    /// ```
    /// use openmodel::geometry::Arrow;
    /// let Arrow = Arrow::new(0.0, 1.0, 2.0, 3.0, 4.0, 5.0);
    /// assert_eq!(Arrow[0], 0.0);
    /// assert_eq!(Arrow[1], 1.0);
    /// assert_eq!(Arrow[2], 2.0);
    /// assert_eq!(Arrow[3], 3.0);
    /// assert_eq!(Arrow[4], 4.0);
    /// assert_eq!(Arrow[5], 5.0);
    /// ```
    fn index(&self, index: usize) -> &Self::Output {
        match index {
            0 => &self.x0,
            1 => &self.y0,
            2 => &self.z0,
            3 => &self.x1,
            4 => &self.y1,
            5 => &self.z1,
            _ => panic!("Index out of bounds"),
        }
    }
}

impl IndexMut<usize> for Arrow {
    /// Provides mutable access to the coordinates of the Arrow using the `[]` operator.
    ///
    /// # Arguments
    ///
    /// * `index` - The index of the coordinate (0 for x0, 1 for y0, 2 for z0, 3 for x1, 4 for y1, 5 for z1).
    ///
    ///
    /// # Panics
    ///
    /// Panics if the index is out of bounds.
    ///
    /// # Example
    ///
    /// ```
    /// use openmodel::geometry::Arrow;
    /// let mut Arrow = Arrow::new(0.0, 0.0, 0.0, 0.0, 0.0, 1.0);
    /// Arrow[0] = 1.0;
    /// Arrow[1] = 2.0;
    /// Arrow[2] = 3.0;
    /// Arrow[3] = 4.0;
    /// Arrow[4] = 5.0;
    /// Arrow[5] = 6.0;
    /// assert_eq!(Arrow[0], 1.0);
    /// assert_eq!(Arrow[1], 2.0);
    /// assert_eq!(Arrow[2], 3.0);
    /// assert_eq!(Arrow[3], 4.0);
    /// assert_eq!(Arrow[4], 5.0);
    /// assert_eq!(Arrow[5], 6.0);
    /// ```
    fn index_mut(&mut self, index: usize) -> &mut Self::Output {
        match index {
            0 => &mut self.x0,
            1 => &mut self.y0,
            2 => &mut self.z0,
            3 => &mut self.x1,
            4 => &mut self.y1,
            5 => &mut self.z1,
            _ => panic!("Index out of bounds"),
        }
    }
}

impl MulAssign<f32> for Arrow {
    /// Multiplies the coordinates of the Arrow by a scalar.
    ///
    /// # Arguments
    ///
    /// * `factor` - The scalar to multiply by.
    ///
    /// # Example
    ///
    /// ```
    /// use openmodel::geometry::Arrow;
    /// let mut Arrow = Arrow::new(0.0, 1.0, 2.0, 3.0, 4.0, 5.0);
    /// Arrow *= 2.0;
    /// assert_eq!(Arrow.x0, 0.0);
    /// assert_eq!(Arrow.y0, 2.0);
    /// assert_eq!(Arrow.z0, 4.0);
    /// assert_eq!(Arrow.x1, 6.0);
    /// assert_eq!(Arrow.y1, 8.0);
    /// assert_eq!(Arrow.z1, 10.0);
    /// ```
    fn mul_assign(&mut self, factor: f32) {
        self.x0 *= factor;
        self.y0 *= factor;
        self.z0 *= factor;
        self.x1 *= factor;
        self.y1 *= factor;
        self.z1 *= factor;
    }
}

impl Mul<f32> for Arrow {
    type Output = Arrow;

    /// Multiplies the coordinates of Arrow point by a scalar and returns a new Arrow.
    ///
    /// # Arguments
    ///
    /// * `factor` - The scalar to multiply by.
    ///
    /// # Example
    ///
    /// ```
    /// use openmodel::geometry::Arrow;
    /// let Arrow0 = Arrow::new(0.0, 1.0, 2.0, 3.0, 4.0, 5.0);
    /// let Arrow1 = Arrow0 * 2.0;
    /// assert_eq!(Arrow1.x0, 0.0);
    /// assert_eq!(Arrow1.y0, 2.0);
    /// assert_eq!(Arrow1.z0, 4.0);
    /// assert_eq!(Arrow1.x1, 6.0);
    /// assert_eq!(Arrow1.y1, 8.0);
    /// assert_eq!(Arrow1.z1, 10.0);
    /// ```
    fn mul(self, factor: f32) -> Arrow {
        let mut result = self;
        result *= factor;
        result
    }
}

impl Sub<&Vector> for Arrow {
    type Output = Arrow;

    /// Subtracts the coordinates of a vector from this Arrow and returns a new vector.
    ///
    /// # Arguments
    ///
    /// * `vector` - The vector to subtract coordinates.
    ///
    /// # Example
    ///
    /// ```
    /// use openmodel::geometry::{Arrow, Vector};
    /// let Arrow0 = Arrow::new(0.0, 1.0, 2.0, 3.0, 4.0, 5.0);
    /// let v = Vector::new(0.0, 0.0, 1.0);
    /// let Arrow1 = Arrow0 - &v;
    /// assert_eq!(Arrow1.x0, 0.0);
    /// assert_eq!(Arrow1.y0, 1.0);
    /// assert_eq!(Arrow1.z0, 1.0);
    /// assert_eq!(Arrow1.x1, 3.0);
    /// assert_eq!(Arrow1.y1, 4.0);
    /// assert_eq!(Arrow1.z1, 4.0);
    /// ```
    fn sub(self, vector: &Vector) -> Arrow {
        Arrow {
            x0: self.x0 - vector.x,
            y0: self.y0 - vector.y,
            z0: self.z0 - vector.z,
            x1: self.x1 - vector.x,
            y1: self.y1 - vector.y,
            z1: self.z1 - vector.z,
            data: Data::with_name("Arrow"),
            mesh: None,
        }
    }
}

impl SubAssign<&Vector> for Arrow {
    /// Subtracts the coordinates of a Arrow using a vector.
    ///
    /// # Arguments
    ///
    /// * `vector` - The subtraction vector.
    ///
    /// # Example
    ///
    /// ```
    /// use openmodel::geometry::Arrow;
    /// use openmodel::geometry::Vector;
    /// let mut Arrow = Arrow::new(0.0, 0.0, 0.0, 0.0, 0.0, 1.0);
    /// let v = Vector::new(1.0, 2.0, 3.0);
    /// Arrow -= &v;
    /// assert_eq!(Arrow.x0, -1.0);
    /// assert_eq!(Arrow.y0, -2.0);
    /// assert_eq!(Arrow.z0, -3.0);
    /// assert_eq!(Arrow.x1, -1.0);
    /// assert_eq!(Arrow.y1, -2.0);
    /// assert_eq!(Arrow.z1, -2.0);
    /// ```
    fn sub_assign(&mut self, vector: &Vector) {
        self.x0 -= vector.x;
        self.y0 -= vector.y;
        self.z0 -= vector.z;
        self.x1 -= vector.x;
        self.y1 -= vector.y;
        self.z1 -= vector.z;
    }
}

impl From<Arrow> for Vector {
    /// Converts a `Arrow` into a `Vector`.
    ///
    /// # Arguments
    ///
    /// * `Arrow` - The `Arrow` to be converted.
    ///
    /// # Example
    ///
    /// ```
    /// use openmodel::geometry::{Arrow, Vector};
    /// let Arrow = Arrow::new(0.0, 0.0, 0.0, 0.0, 0.0, 1.0);
    /// let v: Vector = Arrow.into();
    /// assert_eq!(v.x, 0.0);
    /// assert_eq!(v.y, 0.0);
    /// assert_eq!(v.z, 1.0);
    /// ```
    fn from(arr : Arrow) -> Self {
        Vector::new(
            arr.x1 - arr.x0,
            arr.y1 - arr.y0,
            arr.z1 - arr.z0
        )
    }
}

impl fmt::Display for Arrow{
    /// Log Arrow.
    /// # Example
    ///
    /// ```
    /// use openmodel::geometry::Arrow;
    /// let Arrow = Arrow::new(0.0, 0.0, 0.0, 0.0, 0.0, 1.0);
    /// println!("{}", Arrow);
    /// ```
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result{
        write!(f, "Arrow({}, {}, {}, {}, {}, {})", self.x0, self.y0, self.z0, self.x1, self.y1, self.z1)
    }
}

// JSON serialization support
impl JsonSerializable for Arrow {
    fn to_json_value(&self) -> serde_json::Value {
        let geometric_data = serde_json::json!({
            "x0": self.x0,
            "y0": self.y0,
            "z0": self.z0,
            "x1": self.x1,
            "y1": self.y1,
            "z1": self.z1
        });
        self.data.to_json_data("openmodel.geometry/Arrow", geometric_data, false)
    }
}

impl FromJsonData for Arrow {
    fn from_json_data(data: &serde_json::Value) -> Option<Self> {
        serde_json::from_value(data.clone()).ok()
    }
}