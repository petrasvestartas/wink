/// Scene bounding box for orthographic camera framing
#[derive(Debug, Clone, Copy)]
pub struct SceneBounds {
    pub min: openmodel::geometry::Point,
    pub max: openmodel::geometry::Point,
}

impl Default for SceneBounds {
    fn default() -> Self {
        Self::new()
    }
}

impl SceneBounds {
    pub fn new() -> Self {
        Self {
            min: openmodel::geometry::Point::new(f32::INFINITY, f32::INFINITY, f32::INFINITY),
            max: openmodel::geometry::Point::new(f32::NEG_INFINITY, f32::NEG_INFINITY, f32::NEG_INFINITY),
        }
    }

    pub fn expand_point(&mut self, point: openmodel::geometry::Point) {
        self.min.x = self.min.x.min(point.x);
        self.min.y = self.min.y.min(point.y);
        self.min.z = self.min.z.min(point.z);
        self.max.x = self.max.x.max(point.x);
        self.max.y = self.max.y.max(point.y);
        self.max.z = self.max.z.max(point.z);
    }

    pub fn center(&self) -> openmodel::geometry::Point {
        openmodel::geometry::Point::new(
            (self.min.x + self.max.x) * 0.5,
            (self.min.y + self.max.y) * 0.5,
            (self.min.z + self.max.z) * 0.5,
        )
    }

    pub fn size(&self) -> openmodel::geometry::Vector {
        openmodel::geometry::Vector::new(
            self.max.x - self.min.x,
            self.max.y - self.min.y,
            self.max.z - self.min.z,
        )
    }

    pub fn is_valid(&self) -> bool {
        self.min.x <= self.max.x && self.min.y <= self.max.y && self.min.z <= self.max.z
    }
}
