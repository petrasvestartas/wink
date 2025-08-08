use crate::geometry::{Point, Mesh};
use crate::geometry::Vector;
use crate::common::Data;
use crate::common::{JsonSerializable, FromJsonData};
use serde::{Deserialize, Serialize};
use std::ops::{Add, AddAssign, Div, DivAssign, Index, IndexMut, Mul, MulAssign, Sub, SubAssign};
use std::fmt;
// use std::f64::consts::PI;  // Not needed

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Line {
    /// The x coordinate of the start point.
    pub x0: f64,
    /// The y coordinate of the start point.
    pub y0: f64,
    /// The z coordinate of the start point.
    pub z0: f64,
    /// The x coordinate of the end point.
    pub x1: f64,
    /// The y coordinate of the end point.
    pub y1: f64,
    /// The z coordinate of the end point.
    pub z1: f64,
    /// The data associated with the line (includes color and thickness).
    pub data: Data,
    /// Mesh for visualization (pipe)
    #[serde(skip)]
    pub mesh: Option<Mesh>,
}

impl Line{
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
    pub fn new(x0: f64, y0: f64, z0:f64, x1: f64, y1: f64, z1:f64) -> Self {
        Line {
            x0,
            y0,
            z0,
            x1,
            y1,
            z1,
            data: Data::with_name("Line"),
            mesh: None,
        }
    }

    /// Creates a new `Line` with a specified name for `Data`.
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
    /// use openmodel::geometry::Line;
    /// let line = Line::with_name("MyLine".to_string(), 0.0, 0.0, 0.0, 0.0, 0.0, 1.0);
    /// assert_eq!(line.x0, 0.0);
    /// assert_eq!(line.y0, 0.0);
    /// assert_eq!(line.z0, 0.0);
    /// assert_eq!(line.x1, 0.0);
    /// assert_eq!(line.y1, 0.0);
    /// assert_eq!(line.z1, 1.0);
    /// ```
    pub fn with_name(name: String, x0: f64, y0: f64, z0: f64, x1: f64, y1: f64, z1: f64) -> Self {
        Line {
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

    /// Creates a new `Line` from start ´Point´ and end `Point`.
    ///
    /// # Arguments
    ///
    /// * `p0` - The start point.
    /// * `p1` - The end point.
    /// # Example
    ///
    /// ```
    /// use openmodel::geometry::Point;
    /// use openmodel::geometry::Line;
    /// let p0 = Point::new(0.0, 0.0, 0.0);
    /// let p1 = Point::new(0.0, 0.0, 1.0);
    /// let line = Line::from_points(&p0, &p1);
    /// assert_eq!(line.x0, 0.0);
    /// assert_eq!(line.y0, 0.0);
    /// assert_eq!(line.z0, 0.0);
    /// assert_eq!(line.x1, 0.0);
    /// assert_eq!(line.y1, 0.0);
    /// assert_eq!(line.z1, 1.0);
    /// ```
    pub fn from_points(p0: &Point, p1: &Point) -> Self{
        Line {
            x0:p0.x,
            y0:p0.y,
            z0:p0.z,
            x1:p1.x,
            y1:p1.y,
            z1:p1.z,
            data: Data::with_name("Line"),
            mesh: None,
        }
    }

    /// Computes the length of the line.
    ///
    ///
    /// # Example
    ///
    /// ```
    /// use openmodel::geometry::Line;
    /// let line = Line::new(0.0, 0.0, 0.0, 0.0, 0.0, 1.0);
    /// let length = line.length();
    /// assert_eq!(length, 1.0);
    /// ```
    pub fn length(&self) -> f64 {
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
        
        // If the line has a color, apply it to the mesh
        if self.data.has_color() {
            if let Some(mesh) = &mut self.mesh {
                mesh.data.set_color(self.data.get_color());
            }
        }
        
        self
    }

    /// Gets the mesh representation of this line as a pipe.
    /// If the mesh doesn't exist, creates it first.
    /// 
    /// # Returns
    /// An Option containing a reference to the Mesh if it exists.
    /// 
    /// # Example
    /// 
    /// ```
    /// use openmodel::geometry::Line;
    /// let mut line = Line::new(0.0, 0.0, 0.0, 0.0, 0.0, 1.0);
    /// let mesh = line.get_mesh();
    /// assert!(mesh.is_some());
    /// ```
    pub fn get_mesh(&mut self) -> Option<&Mesh> {
        // Create the mesh if it doesn't exist yet
        if self.mesh.is_none() {
            self.update_mesh();
        }
        
        self.mesh.as_ref()
    }
}

impl Default for Line{
    /// Creates a default `Line` as a vertical line.
    ///
    /// # Example
    ///
    /// ```
    /// use openmodel::geometry::Line;
    /// let l = Line::default();
    /// ```
    fn default() -> Self {
        Line {
            x0: 0.0,
            y0: 0.0,
            z0: 0.0,
            x1: 0.0,
            y1: 0.0,
            z1: 1.0,
            data: Data::with_name("Line"),
            mesh: None,
        }
    }
}

impl Add<&Vector> for Line {
    type Output = Line;

    /// Adds the coordinates of a vector to this line and returns a new line.
    ///
    /// # Arguments
    ///
    /// * `other` - The vector.
    ///
    /// # Example
    ///
    /// ```
    /// use openmodel::geometry::{Line, Vector};
    /// let line0 = Line::new(0.0, 1.0, 2.0, 3.0, 4.0, 5.0);
    /// let v = Vector::new(0.0, 0.0, 1.0);
    /// let line1 = line0 + &v;
    /// assert_eq!(line1.x0, 0.0);
    /// assert_eq!(line1.y0, 1.0);
    /// assert_eq!(line1.z0, 3.0);
    /// assert_eq!(line1.x1, 3.0);
    /// assert_eq!(line1.y1, 4.0);
    /// assert_eq!(line1.z1, 6.0);
    /// ```
    fn add(self, other: &Vector) -> Line {
        Line {
            x0: self.x0 + other.x,
            y0: self.y0 + other.y,
            z0: self.z0 + other.z,
            x1: self.x1 + other.x,
            y1: self.y1 + other.y,
            z1: self.z1 + other.z,
            data: Data::with_name("Line"),
            mesh: None,
        }
    }
}

impl AddAssign<&Vector> for Line {
    /// Adds the coordinates of a vector to this line.
    ///
    /// # Arguments
    ///
    /// * `vector` - traslation vector.
    ///
    /// # Example
    ///
    /// ```
    /// use openmodel::geometry::Line;
    /// use openmodel::geometry::Vector;
    /// let mut line = Line::new(0.0, 0.0, 0.0, 0.0, 0.0, 1.0);
    /// let v = Vector::new(1.0, 1.0, 1.0);
    /// line += &v;
    /// assert_eq!(line.x0, 1.0);
    /// assert_eq!(line.y0, 1.0);
    /// assert_eq!(line.z0, 1.0);
    /// assert_eq!(line.x1, 1.0);
    /// assert_eq!(line.y1, 1.0);
    /// assert_eq!(line.z1, 2.0);
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

impl Div<f64> for Line {
    type Output = Line;

    /// Divides the coordinates of the line by a scalar and returns a new line.
    ///
    /// # Arguments
    ///
    /// * `factor` - The scalar to divide by.
    ///
    /// # Example
    ///
    /// ```
    /// use openmodel::geometry::Line;
    /// let line0 = Line::new(0.0, 0.0, 0.0, 0.0, 0.0, 1.0);
    /// let line1 = line0 / 2.0;
    /// assert_eq!(line1.x0, 0.0);
    /// assert_eq!(line1.y0, 0.0);
    /// assert_eq!(line1.z0, 0.0);
    /// assert_eq!(line1.x1, 0.0);
    /// assert_eq!(line1.y1, 0.0);
    /// assert_eq!(line1.z1, 0.5);
    /// ```
    fn div(self, factor: f64) -> Line {
        let mut result = self;
        result /= factor;
        result
    }
}

impl DivAssign<f64> for Line {
    /// Divides the coordinates of the Line by a scalar.
    ///
    /// # Arguments
    ///
    /// * `factor` - The scalar to divide by.
    ///
    /// # Example
    ///
    /// ```
    /// use openmodel::geometry::Line;
    /// let mut line = Line::new(0.0, 0.0, 0.0, 0.0, 0.0, 1.0);
    /// line /= 2.0;
    /// assert_eq!(line.x0, 0.0);
    /// assert_eq!(line.y0, 0.0);
    /// assert_eq!(line.z0, 0.0);
    /// assert_eq!(line.x1, 0.0);
    /// assert_eq!(line.y1, 0.0);
    /// assert_eq!(line.z1, 0.5);
    /// ```
    fn div_assign(&mut self, factor: f64) {
        self.x0 /= factor;
        self.y0 /= factor;
        self.z0 /= factor;
        self.x1 /= factor;
        self.y1 /= factor;
        self.z1 /= factor;
    }
}

impl Index<usize> for Line {
    type Output = f64;

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
    /// use openmodel::geometry::Line;
    /// let line = Line::new(0.0, 1.0, 2.0, 3.0, 4.0, 5.0);
    /// assert_eq!(line[0], 0.0);
    /// assert_eq!(line[1], 1.0);
    /// assert_eq!(line[2], 2.0);
    /// assert_eq!(line[3], 3.0);
    /// assert_eq!(line[4], 4.0);
    /// assert_eq!(line[5], 5.0);
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

impl IndexMut<usize> for Line {
    /// Provides mutable access to the coordinates of the line using the `[]` operator.
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
    /// use openmodel::geometry::Line;
    /// let mut line = Line::new(0.0, 0.0, 0.0, 0.0, 0.0, 1.0);
    /// line[0] = 1.0;
    /// line[1] = 2.0;
    /// line[2] = 3.0;
    /// line[3] = 4.0;
    /// line[4] = 5.0;
    /// line[5] = 6.0;
    /// assert_eq!(line[0], 1.0);
    /// assert_eq!(line[1], 2.0);
    /// assert_eq!(line[2], 3.0);
    /// assert_eq!(line[3], 4.0);
    /// assert_eq!(line[4], 5.0);
    /// assert_eq!(line[5], 6.0);
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

impl MulAssign<f64> for Line {
    /// Multiplies the coordinates of the line by a scalar.
    ///
    /// # Arguments
    ///
    /// * `factor` - The scalar to multiply by.
    ///
    /// # Example
    ///
    /// ```
    /// use openmodel::geometry::Line;
    /// let mut line = Line::new(0.0, 1.0, 2.0, 3.0, 4.0, 5.0);
    /// line *= 2.0;
    /// assert_eq!(line.x0, 0.0);
    /// assert_eq!(line.y0, 2.0);
    /// assert_eq!(line.z0, 4.0);
    /// assert_eq!(line.x1, 6.0);
    /// assert_eq!(line.y1, 8.0);
    /// assert_eq!(line.z1, 10.0);
    /// ```
    fn mul_assign(&mut self, factor: f64) {
        self.x0 *= factor;
        self.y0 *= factor;
        self.z0 *= factor;
        self.x1 *= factor;
        self.y1 *= factor;
        self.z1 *= factor;
    }
}

impl Mul<f64> for Line {
    type Output = Line;

    /// Multiplies the coordinates of line point by a scalar and returns a new line.
    ///
    /// # Arguments
    ///
    /// * `factor` - The scalar to multiply by.
    ///
    /// # Example
    ///
    /// ```
    /// use openmodel::geometry::Line;
    /// let line0 = Line::new(0.0, 1.0, 2.0, 3.0, 4.0, 5.0);
    /// let line1 = line0 * 2.0;
    /// assert_eq!(line1.x0, 0.0);
    /// assert_eq!(line1.y0, 2.0);
    /// assert_eq!(line1.z0, 4.0);
    /// assert_eq!(line1.x1, 6.0);
    /// assert_eq!(line1.y1, 8.0);
    /// assert_eq!(line1.z1, 10.0);
    /// ```
    fn mul(self, factor: f64) -> Line {
        let mut result = self;
        result *= factor;
        result
    }
}

impl Sub<&Vector> for Line {
    type Output = Line;

    /// Subtracts the coordinates of a vector from this Line and returns a new vector.
    ///
    /// # Arguments
    ///
    /// * `vector` - The vector to subtract coordinates.
    ///
    /// # Example
    ///
    /// ```
    /// use openmodel::geometry::{Line, Vector};
    /// let line0 = Line::new(0.0, 1.0, 2.0, 3.0, 4.0, 5.0);
    /// let v = Vector::new(0.0, 0.0, 1.0);
    /// let line1 = line0 - &v;
    /// assert_eq!(line1.x0, 0.0);
    /// assert_eq!(line1.y0, 1.0);
    /// assert_eq!(line1.z0, 1.0);
    /// assert_eq!(line1.x1, 3.0);
    /// assert_eq!(line1.y1, 4.0);
    /// assert_eq!(line1.z1, 4.0);
    /// ```
    fn sub(self, vector: &Vector) -> Line {
        Line {
            x0: self.x0 - vector.x,
            y0: self.y0 - vector.y,
            z0: self.z0 - vector.z,
            x1: self.x1 - vector.x,
            y1: self.y1 - vector.y,
            z1: self.z1 - vector.z,
            data: Data::with_name("Line"),
            mesh: None,
        }
    }
}

impl SubAssign<&Vector> for Line {
    /// Subtracts the coordinates of a line using a vector.
    ///
    /// # Arguments
    ///
    /// * `vector` - The subtraction vector.
    ///
    /// # Example
    ///
    /// ```
    /// use openmodel::geometry::Line;
    /// use openmodel::geometry::Vector;
    /// let mut line = Line::new(0.0, 0.0, 0.0, 0.0, 0.0, 1.0);
    /// let v = Vector::new(1.0, 2.0, 3.0);
    /// line -= &v;
    /// assert_eq!(line.x0, -1.0);
    /// assert_eq!(line.y0, -2.0);
    /// assert_eq!(line.z0, -3.0);
    /// assert_eq!(line.x1, -1.0);
    /// assert_eq!(line.y1, -2.0);
    /// assert_eq!(line.z1, -2.0);
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

impl From<Line> for Vector {
    /// Converts a `Line` into a `Vector`.
    ///
    /// # Arguments
    ///
    /// * `line` - The `Line` to be converted.
    ///
    /// # Example
    ///
    /// ```
    /// use openmodel::geometry::{Line, Vector};
    /// let line = Line::new(0.0, 0.0, 0.0, 0.0, 0.0, 1.0);
    /// let v: Vector = line.into();
    /// assert_eq!(v.x, 0.0);
    /// assert_eq!(v.y, 0.0);
    /// assert_eq!(v.z, 1.0);
    /// ```
    fn from(line: Line) -> Self {
        Vector::new(
            line.x1 - line.x0,
            line.y1 - line.y0,
            line.z1 - line.z0
        )
    }
}

impl fmt::Display for Line{
    /// Log line.
    /// # Example
    ///
    /// ```
    /// use openmodel::geometry::Line;
    /// let line = Line::new(0.0, 0.0, 0.0, 0.0, 0.0, 1.0);
    /// println!("{}", line);
    /// ```
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result{
        write!(f, "Line({}, {}, {}, {}, {}, {})", self.x0, self.y0, self.z0, self.x1, self.y1, self.z1)
    }
}

// JSON serialization support
impl JsonSerializable for Line {
    fn to_json_value(&self) -> serde_json::Value {
        let geometric_data = serde_json::json!({
            "x0": self.x0,
            "y0": self.y0,
            "z0": self.z0,
            "x1": self.x1,
            "y1": self.y1,
            "z1": self.z1
        });
        self.data.to_json_data("openmodel.geometry/Line", geometric_data, false)
    }
}

impl FromJsonData for Line {
    fn from_json_data(data: &serde_json::Value) -> Option<Self> {
        serde_json::from_value(data.clone()).ok()
    }
}