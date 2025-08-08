use crate::geometry::{Line, Color, Xform, Mesh};
use crate::common::{FromJsonData, HasJsonData, Data};
use serde::{Deserialize, Serialize};
use std::ops::{Add, AddAssign, Sub, SubAssign};
use std::fmt;
use crate::primitives::Vector;
use crate::geometry::Point;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LineCloud {
    /// The collection of lines.
    pub lines: Vec<Line>,

    /// The collection of colors.
    pub colors: Vec<Color>,

    /// The transformation matrix.
    pub xform: Xform,

    /// Associated data - guid and name.
    pub data: Data,
    
    /// Collection of meshes for visualization (pipes)
    #[serde(skip)]
    pub meshes: Vec<Mesh>,
    
    /// Flag indicating if meshes need to be rebuilt
    #[serde(skip)]
    dirty: bool,
}

impl LineCloud {
    /// Creates a new `LineCloud` with default `Data`.
    ///
    /// # Arguments
    ///
    /// * `lines` - The collection of lines.
    /// * `colors` - The collection of colors.
    ///
    /// # Example
    ///
    /// ```
    /// use openmodel::geometry::{LineCloud, Line, Point, Color};
    /// let lines = vec![Line::new(0.0, 0.0, 0.0, 1.0, 0.0, 0.0)];
    /// let colors = vec![Color::new(255, 0, 0, 0)];
    /// let line_cloud = LineCloud::new(lines, colors);
    /// ```
    ///
    /// # Panics
    ///
    /// Panics if the number of lines and colors are not equal.
    pub fn new(lines: Vec<Line>, colors: Vec<Color>) -> Self {
        assert_eq!(lines.len(), colors.len(), "Number of lines and colors must be equal");
        Self {
            lines,
            colors,
            xform: Xform::default(),
            data: Data::default(),
            meshes: Vec::new(),
            dirty: true,
        }
    }
}

impl AddAssign<&Vector> for LineCloud {
    /// Adds a vector to all line points.
    ///
    /// # Arguments
    ///
    /// * `other` - The vector to add.
    ///
    /// # Example
    ///
    /// ```
    /// use openmodel::geometry::{LineCloud, Line, Point, Vector, Color};
    /// let mut lc = LineCloud::new(
    ///     vec![Line::new(1.0, 2.0, 3.0, 4.0, 5.0, 6.0)],
    ///     vec![Color::new(255, 0, 0, 0)]
    /// );
    /// let v = Vector::new(1.0, 1.0, 1.0);
    /// lc += &v;
    /// assert_eq!(lc.lines[0].x0, 2.0);
    /// assert_eq!(lc.lines[0].x1, 5.0);
    /// ```
    fn add_assign(&mut self, other: &Vector) {
        for line in &mut self.lines {
            line.x0 += other.x;
            line.y0 += other.y;
            line.z0 += other.z;
            line.x1 += other.x;
            line.y1 += other.y;
            line.z1 += other.z;
        }
        self.dirty = true;
    }
}

impl Add<&Vector> for LineCloud {
    type Output = LineCloud;

    /// Adds a vector to all line points and returns a new LineCloud.
    ///
    /// # Arguments
    ///
    /// * `other` - The vector to add.
    ///
    /// # Example
    ///
    /// ```
    /// use openmodel::geometry::{LineCloud, Line, Point, Vector, Color};
    /// let lc = LineCloud::new(
    ///     vec![Line::new(1.0, 2.0, 3.0, 4.0, 5.0, 6.0)],
    ///     vec![Color::new(255, 0, 0, 0)]
    /// );
    /// let v = Vector::new(1.0, 1.0, 1.0);
    /// let lc2 = lc + &v;
    /// assert_eq!(lc2.lines[0].x0, 2.0);
    /// assert_eq!(lc2.lines[0].x1, 5.0);
    /// ```
    fn add(self, other: &Vector) -> LineCloud {
        let mut lc = self.clone();
        for line in &mut lc.lines {
            line.x0 += other.x;
            line.y0 += other.y;
            line.z0 += other.z;
            line.x1 += other.x;
            line.y1 += other.y;
            line.z1 += other.z;
        }
        lc
    }
}

impl SubAssign<&Vector> for LineCloud {
    /// Subtracts a vector from all line points.
    ///
    /// # Arguments
    ///
    /// * `other` - The vector to subtract.
    ///
    /// # Example
    ///
    /// ```
    /// use openmodel::geometry::{LineCloud, Line, Point, Vector, Color};
    /// let mut lc = LineCloud::new(
    ///     vec![Line::new(1.0, 2.0, 3.0, 4.0, 5.0, 6.0)],
    ///     vec![Color::new(255, 0, 0, 0)]
    /// );
    /// let v = Vector::new(1.0, 1.0, 1.0);
    /// lc -= &v;
    /// assert_eq!(lc.lines[0].x0, 0.0);
    /// assert_eq!(lc.lines[0].x1, 3.0);
    /// ```
    fn sub_assign(&mut self, other: &Vector) {
        for line in &mut self.lines {
            line.x0 -= other.x;
            line.y0 -= other.y;
            line.z0 -= other.z;
            line.x1 -= other.x;
            line.y1 -= other.y;
            line.z1 -= other.z;
        }
        self.dirty = true;
    }
}

impl Sub<&Vector> for LineCloud {
    type Output = LineCloud;

    /// Subtracts a vector from all line points and returns a new LineCloud.
    ///
    /// # Arguments
    ///
    /// * `other` - The vector to subtract.
    ///
    /// # Example
    ///
    /// ```
    /// use openmodel::geometry::{LineCloud, Line, Point, Vector, Color};
    /// let lc = LineCloud::new(
    ///     vec![Line::new(1.0, 2.0, 3.0, 4.0, 5.0, 6.0)],
    ///     vec![Color::new(255, 0, 0, 0)]
    /// );
    /// let v = Vector::new(1.0, 1.0, 1.0);
    /// let lc2 = lc - &v;
    /// assert_eq!(lc2.lines[0].x0, 0.0);
    /// assert_eq!(lc2.lines[0].x1, 3.0);
    /// ```
    fn sub(self, other: &Vector) -> LineCloud {
        let mut lc = self.clone();
        for line in &mut lc.lines {
            line.x0 -= other.x;
            line.y0 -= other.y;
            line.z0 -= other.z;
            line.x1 -= other.x;
            line.y1 -= other.y;
            line.z1 -= other.z;
        }
        lc
    }
}

impl fmt::Display for LineCloud {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "LineCloud {{ lines: {}, colors: {}, data: {} }}",
            self.lines.len(),
            self.colors.len(),
            self.data
        )
    }
}

// JSON serialization support
impl HasJsonData for LineCloud {
    fn to_json_data(&self, minimal: bool) -> serde_json::Value {
        let geometric_data = serde_json::json!({
            "lines": self.lines,
            "colors": self.colors
        });
        self.data.to_json_data("openmodel.geometry/LineCloud", geometric_data, minimal)
    }
}

impl FromJsonData for LineCloud {
    fn from_json_data(data: &serde_json::Value) -> Option<Self> {
        serde_json::from_value(data.clone()).ok()
    }
}

// LineCloud visualization methods
impl LineCloud {
    /// Updates the meshes for all lines using thickness from data.
    /// Called internally when needed or when explicitly requested.
    ///
    /// # Example
    ///
    /// ```
    /// use openmodel::geometry::{LineCloud, Line, Color};
    /// let mut lc = LineCloud::new(
    ///     vec![Line::new(0.0, 0.0, 0.0, 1.0, 0.0, 0.0)],
    ///     vec![Color::new(255, 0, 0, 0)]
    /// );
    /// lc.update_meshes();
    /// assert_eq!(lc.meshes.len(), 1);
    /// ```
    pub fn update_meshes(&mut self) -> &mut Self {
        if !self.dirty {
            return self;
        }
        
        // Get thickness from data
        let thickness = self.data.get_thickness();
        // Use fixed 8 sides for all pipes

        
        self.meshes.clear();
        
        // Create a mesh for each line with its color
        for (i, line) in self.lines.iter().enumerate() {
            let start = Point::new(line.x0, line.y0, line.z0);
            let end = Point::new(line.x1, line.y1, line.z1);
            
            // Create pipe mesh
            let mut mesh = Mesh::create_pipe(start, end, thickness);
            
            // Apply line color to the mesh if available
            if i < self.colors.len() {
                // Store the color from the colors array in the mesh's data
                let color = &self.colors[i];
                mesh.data.set_color([color.r, color.g, color.b]);
            }
            
            self.meshes.push(mesh);
        }
        
        self.dirty = false;
        self
    }
    
    /// Gets all the mesh representations of this line cloud as pipes.
    /// If the meshes don't exist, creates them first.
    ///
    /// # Returns
    /// A reference to the vector of meshes.
    ///
    /// # Example
    ///
    /// ```
    /// use openmodel::geometry::{LineCloud, Line, Color};
    /// let mut lc = LineCloud::new(
    ///     vec![Line::new(0.0, 0.0, 0.0, 1.0, 0.0, 0.0)],
    ///     vec![Color::new(255, 0, 0, 0)]
    /// );
    /// let meshes = lc.get_meshes();
    /// assert_eq!(meshes.len(), 1);
    /// ```
    pub fn get_meshes(&mut self) -> &Vec<Mesh> {
        // Create meshes if they don't exist or if they need updating
        if self.meshes.is_empty() || self.dirty {
            self.update_meshes();
        }
        
        &self.meshes
    }
}
