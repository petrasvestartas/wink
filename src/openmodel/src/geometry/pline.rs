use crate::geometry::Point;
use crate::geometry::Vector;
use crate::geometry::Plane;
use crate::geometry::Mesh;
use crate::common::{FromJsonData, HasJsonData};
use serde::{Deserialize, Serialize};
use std::ops::{Add, AddAssign, Sub, SubAssign};
use crate::common::Data;
use std::fmt;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Pline {
    /// The collection of points.
    pub points: Vec<Point>,

    /// The plane of the polyline.
    pub plane: Plane,

    /// Associated data - guid and name.
    pub data: Data,
}

impl Pline {
    /// Creates a new `Pline` with default `Data`.
    ///
    /// # Arguments
    ///
    /// * `points` - The collection of points.
    ///
    /// # Example
    ///
    /// ```
    /// use openmodel::geometry::Point;
    /// use openmodel::geometry::Plane;
    /// use openmodel::geometry::Pline;
    /// let points = vec![Point::new(0.0, 0.0, 0.0), Point::new(1.0, 0.0, 0.0), Point::new(0.0, 1.0, 0.0)];
    /// let Pline = Pline::new(points);
    /// ```
    ///
    pub fn new(points: Vec<Point>) -> Self {

        // Delegate plane computation to Plane::plane_from_points
        let plane = Plane::plane_from_points(&points);

        Self {
            points,
            plane,
            data: Data::default(),
        }
    }
    
    /// Convert polyline segments to pipe meshes for visualization.
    /// Each segment between consecutive points becomes a cylindrical pipe mesh.
    /// 
    /// # Arguments
    /// 
    /// * `radius` - The radius of the pipe meshes (uses data.thickness if None)
    /// * `sides` - Number of sides for the cylindrical pipes (default: 8)
    /// 
    /// # Returns
    /// 
    /// A vector of `Mesh` objects, one for each segment in the polyline.
    /// 
    /// # Example
    /// 
    /// ```
    /// use openmodel::geometry::{Point, Pline};
    /// let points = vec![
    ///     Point::new(0.0, 0.0, 0.0),
    ///     Point::new(1.0, 0.0, 0.0),
    ///     Point::new(1.0, 1.0, 0.0)
    /// ];
    /// let pline = Pline::new(points);
    /// let pipe_meshes = pline.to_pipe_meshes(Some(0.1), None);
    /// assert_eq!(pipe_meshes.len(), 2); // Two segments
    /// ```
    pub fn to_pipe_meshes(&self, radius: Option<f64>, sides: Option<usize>) -> Vec<Mesh> {
        let mut meshes = Vec::new();
        
        // Need at least 2 points to create segments
        if self.points.len() < 2 {
            return meshes;
        }
        
        // Use provided radius or fall back to data thickness, or default to 0.05
        let pipe_radius = radius.unwrap_or_else(|| {
            let thickness = self.data.get_thickness();
            if thickness > 0.0 { thickness } else { 0.05 }
        });
        let _pipe_sides = sides.unwrap_or(8);
        
        // Create a pipe mesh for each segment
        for i in 0..self.points.len() - 1 {
            let start_point = &self.points[i];
            let end_point = &self.points[i + 1];
            
            // Create pipe mesh for this segment
            let mut pipe_mesh = Mesh::create_pipe(
                start_point.clone(),
                end_point.clone(),
                pipe_radius
            );
            
            // Apply color from data if available
            let color = self.data.get_color();
            if color != [0, 0, 0] { // Only apply if not default black
                pipe_mesh.data.set_color(color);
            }
            
            meshes.push(pipe_mesh);
        }
        
        meshes
    }
}


impl AddAssign<&Vector> for Pline {
    /// Adds the coordinates of another point to this point.
    ///
    /// # Arguments
    ///
    /// * `other` - The other point.
    ///
    /// # Example
    ///
    /// ```
    /// use openmodel::geometry::{Pline, Point, Vector};
    /// let mut c = Pline::new(vec![Point::new(1.0, 2.0, 3.0), Point::new(4.0, 5.0, 6.0)]);
    /// let v = Vector::new(4.0, 5.0, 6.0);
    /// c += &v;
    /// assert_eq!(c.points[0].x, 5.0);
    /// assert_eq!(c.points[0].y, 7.0);
    /// assert_eq!(c.points[0].z, 9.0);
    /// assert_eq!(c.points[1].x, 8.0);
    /// assert_eq!(c.points[1].y, 10.0);
    /// assert_eq!(c.points[1].z, 12.0);
    /// ```
    fn add_assign(&mut self, other: &Vector) {
        for p in &mut self.points {
            p.x += other.x;
            p.y += other.y;
            p.z += other.z;
        }
    }
}

impl Add<&Vector> for Pline {
    type Output = Pline;

    /// Adds the coordinates of a vector to this point and returns a new point.
    ///
    /// # Arguments
    ///
    /// * `other` - The vector.
    ///
    /// # Example
    ///
    /// ```
    /// use openmodel::geometry::{Pline, Point, Vector};
    /// let c = Pline::new(vec![Point::new(1.0, 2.0, 3.0), Point::new(4.0, 5.0, 6.0)]);
    /// let v = Vector::new(4.0, 5.0, 6.0);
    /// let c2 = c + &v;
    /// assert_eq!(c2.points[0].x, 5.0);
    /// assert_eq!(c2.points[0].y, 7.0);
    /// assert_eq!(c2.points[0].z, 9.0);
    /// ```
    fn add(self, other: &Vector) -> Pline {
        let mut c = self.clone();
        for p in &mut c.points {
            p.x += other.x;
            p.y += other.y;
            p.z += other.z;
        }
        return c;
    }
}



impl SubAssign <&Vector> for Pline {
    /// Adds the coordinates of another point to this point.
    ///
    /// # Arguments
    ///
    /// * `other` - The other point.
    ///
    /// # Example
    ///
    /// ```
    /// use openmodel::geometry::{Pline, Point, Vector};
    /// let mut c = Pline::new(vec![Point::new(1.0, 2.0, 3.0), Point::new(4.0, 5.0, 6.0)]);
    /// let v = Vector::new(4.0, 5.0, 6.0);
    /// c -= &v;
    /// assert_eq!(c.points[0].x, -3.0);
    /// assert_eq!(c.points[0].y, -3.0);
    /// assert_eq!(c.points[0].z, -3.0);
    /// assert_eq!(c.points[1].x, 0.0);
    /// assert_eq!(c.points[1].y, 0.0);
    /// assert_eq!(c.points[1].z, 0.0);
    /// ```
    fn sub_assign(&mut self, other: &Vector) {
        for p in &mut self.points {
            p.x -= other.x;
            p.y -= other.y;
            p.z -= other.z;
        }
    }
}

impl Sub<&Vector> for Pline {
    type Output = Pline;

    /// Adds the coordinates of a vector to this point and returns a new point.
    ///
    /// # Arguments
    ///
    /// * `other` - The vector.
    ///
    /// # Example
    ///
    /// ```
    /// use openmodel::geometry::{Pline, Point, Vector};
    /// let c = Pline::new(vec![Point::new(1.0, 2.0, 3.0), Point::new(4.0, 5.0, 6.0)]);
    /// let v = Vector::new(4.0, 5.0, 6.0);
    /// let c2 = c - &v;
    /// assert_eq!(c2.points[0].x, -3.0);
    /// assert_eq!(c2.points[0].y, -3.0);
    /// assert_eq!(c2.points[0].z, -3.0);
    /// assert_eq!(c2.points[1].x, 0.0);
    /// assert_eq!(c2.points[1].y, 0.0);
    /// assert_eq!(c2.points[1].z, 0.0);
    /// ```
    fn sub(self, other: &Vector) -> Pline {
        let mut c = self.clone();
        for p in &mut c.points {
            p.x -= other.x;
            p.y -= other.y;
            p.z -= other.z;
        }
        return c;
    }
}

impl fmt::Display for Pline {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "Pline {{ points: {}, data: {} }}",
            self.points.len(),
            self.data
        )
    }
}

// JSON serialization support
impl HasJsonData for Pline {
    fn to_json_data(&self, minimal: bool) -> serde_json::Value {
        let geometric_data = serde_json::json!({
            "points": self.points,
            "plane": self.plane
        });
        self.data.to_json_data("openmodel.geometry/Pline", geometric_data, minimal)
    }
}

impl FromJsonData for Pline {
    fn from_json_data(data: &serde_json::Value) -> Option<Self> {
        serde_json::from_value(data.clone()).ok()
    }
}
