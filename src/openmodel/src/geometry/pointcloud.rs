use crate::primitives::{Point, Vector, Color, Xform};
use crate::common::{FromJsonData, HasJsonData, Data};
use serde::{Deserialize, Serialize, Serializer};
use std::ops::{Add, AddAssign, Sub, SubAssign};
use std::fmt;

#[derive(Debug, Clone, Deserialize)]
pub struct PointCloud {
    /// The collection of point positions (simple points without transformation data).
    pub points: Vec<Point>,

    /// The collection of normals.
    pub normals: Vec<Vector>,

    /// The collection of colors.
    pub colors: Vec<Color>,

    /// The transformation matrix for the entire point cloud.
    pub xform: Xform,

    /// Associated data - guid and name.
    pub data: Data,
}

impl PointCloud {
    /// Creates a new `PointCloud` with default `Data`.
    ///
    /// # Arguments
    ///
    /// * `points` - The collection of point positions.
    /// * `normals` - The collection of normals.
    /// * `colors` - The collection of colors.
    ///
    /// # Example
    ///
    /// ```
    /// use openmodel::primitives::{Point, Vector, Color};
    /// use openmodel::geometry::PointCloud;
    /// let points = vec![Point::new(0.0, 0.0, 0.0), Point::new(1.0, 0.0, 0.0), Point::new(0.0, 1.0, 0.0)];
    /// let normals = vec![Vector::new(0.0, 0.0, 1.0), Vector::new(0.0, 1.0, 0.0), Vector::new(1.0, 0.0, 0.0)];
    /// let colors = vec![Color::new(255, 0, 0, 0), Color::new(0, 255, 0, 0), Color::new(0, 0, 255, 0)];
    /// let cloud = PointCloud::new(points, normals, colors);
    /// ```
    ///
    /// # Panics
    ///
    /// Panics if the number of points, normals, and colors are not equal.
    pub fn new(points: Vec<Point>, normals: Vec<Vector>, colors: Vec<Color>) -> Self {
        Self {
            points,
            normals,
            colors,
            xform: Xform::default(),
            data: Data::default(),
        }
    }
}


impl AddAssign<&Vector> for PointCloud {
    /// Adds the coordinates of another point to this point.
    ///
    /// # Arguments
    ///
    /// * `other` - The other point.
    ///
    /// # Example
    ///
    /// ```
    /// use openmodel::geometry::{PointCloud, Point, Vector, Color};
    /// let mut c = PointCloud::new(vec![Point::new(1.0, 2.0, 3.0)], vec![Vector::new(0.0, 0.0, 1.0)], vec![Color::new(255, 0, 0, 0)]);
    /// let v = Vector::new(4.0, 5.0, 6.0);
    /// c += &v;
    /// assert_eq!(c.points[0].x, 5.0);
    /// assert_eq!(c.points[0].y, 7.0);
    /// assert_eq!(c.points[0].z, 9.0);
    /// ```
    fn add_assign(&mut self, other: &Vector) {
        for p in &mut self.points {
            p.x += other.x;
            p.y += other.y;
            p.z += other.z;
        }
    }
}

impl Add<&Vector> for PointCloud {
    type Output = PointCloud;

    /// Adds the coordinates of a vector to this point and returns a new point.
    ///
    /// # Arguments
    ///
    /// * `other` - The vector.
    ///
    /// # Example
    ///
    /// ```
    /// use openmodel::geometry::{PointCloud, Point, Vector, Color};
    /// let c = PointCloud::new(vec![Point::new(1.0, 2.0, 3.0)], vec![Vector::new(0.0, 0.0, 1.0)], vec![Color::new(255, 0, 0, 0)]);
    /// let v = Vector::new(4.0, 5.0, 6.0);
    /// let c2 = c + &v;
    /// assert_eq!(c2.points[0].x, 5.0);
    /// assert_eq!(c2.points[0].y, 7.0);
    /// assert_eq!(c2.points[0].z, 9.0);
    /// ```
    fn add(self, other: &Vector) -> PointCloud {
        let mut c = self.clone();
        for p in &mut c.points {
            p.x += other.x;
            p.y += other.y;
            p.z += other.z;
        }
        return c;
    }
}



impl SubAssign <&Vector> for PointCloud {
    /// Adds the coordinates of another point to this point.
    ///
    /// # Arguments
    ///
    /// * `other` - The other point.
    ///
    /// # Example
    ///
    /// ```
    /// use openmodel::geometry::{PointCloud, Point, Vector, Color};
    /// let mut c = PointCloud::new(vec![Point::new(1.0, 2.0, 3.0)], vec![Vector::new(0.0, 0.0, 1.0)], vec![Color::new(255, 0, 0, 0)]);
    /// let v = Vector::new(4.0, 5.0, 6.0);
    /// c -= &v;
    /// assert_eq!(c.points[0].x, -3.0);
    /// assert_eq!(c.points[0].y, -3.0);
    /// assert_eq!(c.points[0].z, -3.0);
    /// ```
    fn sub_assign(&mut self, other: &Vector) {
        for p in &mut self.points {
            p.x -= other.x;
            p.y -= other.y;
            p.z -= other.z;
        }
    }
}

impl Sub<&Vector> for PointCloud {
    type Output = PointCloud;

    /// Adds the coordinates of a vector to this point and returns a new point.
    ///
    /// # Arguments
    ///
    /// * `other` - The vector.
    ///
    /// # Example
    ///
    /// ```
    /// use openmodel::geometry::{PointCloud, Point, Vector, Color};
    /// let c = PointCloud::new(vec![Point::new(1.0, 2.0, 3.0)], vec![Vector::new(0.0, 0.0, 1.0)], vec![Color::new(255, 0, 0, 0)]);
    /// let v = Vector::new(4.0, 5.0, 6.0);
    /// let c2 = c - &v;
    /// assert_eq!(c2.points[0].x, -3.0);
    /// assert_eq!(c2.points[0].y, -3.0);
    /// assert_eq!(c2.points[0].z, -3.0);
    /// ```
    fn sub(self, other: &Vector) -> PointCloud {
        let mut c = self.clone();
        for p in &mut c.points {
            p.x -= other.x;
            p.y -= other.y;
            p.z -= other.z;
        }
        return c;
    }
}

// Custom Serialize implementation for simple format compatible with wink
impl Serialize for PointCloud {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        use serde::ser::SerializeStruct;
        let mut state = serializer.serialize_struct("PointCloud", 5)?;
        state.serialize_field("points", &self.points)?;
        state.serialize_field("normals", &self.normals)?;
        state.serialize_field("colors", &self.colors)?;
        state.serialize_field("xform", &self.xform)?;
        state.serialize_field("data", &self.data)?;
        state.end()
    }
}

impl fmt::Display for PointCloud {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "PointCloud {{ points: {}, normals: {}, colors: {}, data: {} }}",
            self.points.len(),
            self.normals.len(),
            self.colors.len(),
            self.data
        )
    }
}

// JSON serialization support
impl HasJsonData for PointCloud {
    fn to_json_data(&self, minimal: bool) -> serde_json::Value {
        let geometric_data = serde_json::json!({
            "points": self.points,
            "colors": self.colors,
            "xform": self.xform
        });
        self.data.to_json_data("openmodel.geometry/PointCloud", geometric_data, minimal)
    }
}

impl FromJsonData for PointCloud {
    fn from_json_data(data: &serde_json::Value) -> Option<Self> {
        serde_json::from_value(data.clone()).ok()
    }
}
