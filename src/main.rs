use std::str::FromStr;

use serde::{Deserialize, Deserializer};

use svg::Document;
use svg::Node;
use svg::node::element::{Polygon, Rectangle};

#[derive(Deserialize, Debug)]
pub struct World {
    pub cell: Vec<Cell>,
}

impl World {
    pub fn render(&self, document: &mut Document) {
        for cell in &self.cell {
            cell.render(document);
        }
    }
}

#[derive(Deserialize, Debug)]
pub struct Cell {
    pub x: i32,
    pub y: i32,
    pub feature: Vec<Feature>,
}

impl Cell {
    pub fn bottom_left(&self) -> Point {
        // each cell seems to be 300 by 300 pixels
        Point {
            x: self.x * 300,
            y: self.y * 300,
        }
    }

    pub fn render(&self, document: &mut Document) {
        for feature in &self.feature {
            feature.render(document, &self.bottom_left())
        }
    }
}

#[derive(Deserialize, Debug)]
pub struct Feature {
    pub geometry: Geometry,
    pub property: Option<Vec<Property>>,
}

impl Feature {
    pub fn render(&self, document: &mut Document, bottom_left: &Point) {
        self.geometry.render(document, bottom_left)
    }
}

#[derive(Debug)]
pub enum GeometryType {
    LineString,
    Polygon,
    Point,
}

impl FromStr for GeometryType {
    type Err = std::io::Error;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(match s {
            "LineString" => GeometryType::LineString,
            "Polygon" => GeometryType::Polygon,
            "Point" => GeometryType::Point,
            _ => panic!("wat"),
        })
    }
}

impl<'de> Deserialize<'de> for GeometryType {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        GeometryType::from_str(&s).map_err(serde::de::Error::custom)
    }
}

#[derive(Deserialize, Debug)]
pub struct Geometry {
    #[serde(rename = "type")]
    pub geometry_type: GeometryType,

    pub coordinates: Vec<Coordinates>,
}

impl Geometry {
    pub fn render(&self, document: &mut Document, bottom_left: &Point) {
        for coordinate in &self.coordinates {
            coordinate.render(document, bottom_left)
        }
    }
}

#[derive(Deserialize, Debug)]
pub struct Coordinates {
    pub point: Vec<Point>,
}

impl Coordinates {
    pub fn render(&self, document: &mut Document, bottom_left: &Point) {
        let points: String = self.point
            .iter()
            .map(|p| {
                let adjusted_point = bottom_left.add(&p);
                format!("{},{}", adjusted_point.x, adjusted_point.y)
            })
            .collect::<Vec<String>>()
            .join(" ");

        let polygon = Polygon::new()
            .set("fill", "none")
            .set("stroke", "black")
            .set("stroke-width", 2)
            .set("points", points);

        document.append(polygon);
    }
}

#[derive(Deserialize, Debug, PartialEq, Clone)]
pub struct Point {
    pub x: i32,
    pub y: i32,
}

impl Point {
    pub fn add(&self, p: &Point) -> Point {
        Point {
            x: self.x + p.x,
            y: self.y + p.y,
        }
    }

    pub fn line_to(&self, p: &Point) -> Point {
        Point {
            x: p.x - self.x,
            y: p.y - self.y,
        }
    }
}

#[test]
fn test_coordinate_stuff() {
    // refer to first feature in worldmap.xml, which is a box:
    //
    // <coordinates>
    //  <point x="300" y="0"/>
    //  <point x="300" y="300"/>
    //  <point x="0" y="300"/>
    //  <point x="0" y="0"/>
    // </coordinates>
    //
    // make sure the Point functions make sense for svg

    // cell has bottom left
    let start = Point { x: 0, y: 0 };

    // svg starts with "move_to" - this corresponds to the first point in the
    // coordinates list
    let move_to = start.add(&Point { x: 300, y: 0 });
    assert_eq!(move_to, Point { x: 300, y: 0 });

    // then does line_by. so if I want to move to 300, 300,
    // it should do a line_by of 0, 300
    let line_by = move_to.line_to(&Point { x: 300, y: 300 });
    assert_eq!(line_by, Point { x: 0, y: 300 });

    // then I end up at point
    let point = Point { x: 300, y: 300 };
    assert_eq!(move_to.add(&line_by), point);

    // then, go to next
    let line_by = point.line_to(&Point { x: 0, y: 300 });
    assert_eq!(line_by, Point { x: -300, y: 0 });
    let point = Point { x: 0, y: 300 };

    // then next
    let line_by = point.line_to(&Point { x: 0, y: 0 });
    assert_eq!(line_by, Point { x: 0, y: -300 });
    let point = Point { x: 0, y: 0 };

    // but this doesn't actually draw a box - connect to the first point again!
}

#[derive(Deserialize, Debug)]
pub struct Property {
    pub name: String,
    pub value: String,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let file_path = "/home/jwm/GOG Games/Project Zomboid/game/projectzomboid/media/maps/Muldraugh, KY/worldmap.xml";
    let file_str = std::fs::read_to_string(file_path)?;

    let xml: World = quick_xml::de::from_str(&file_str)?;

    let mut min_cell_x = 0;
    let mut max_cell_x = 0;
    let mut min_cell_y = 0;
    let mut max_cell_y = 0;

    for cell in &xml.cell {
        min_cell_x = std::cmp::min(min_cell_x, cell.x);
        max_cell_x = std::cmp::max(max_cell_x, cell.x);

        min_cell_y = std::cmp::min(min_cell_y, cell.y);
        max_cell_y = std::cmp::max(max_cell_y, cell.y);
    }

    println!("{} cells", xml.cell.len());
    println!("{} {} {} {}", min_cell_x, max_cell_x, min_cell_y, max_cell_y);

    let mut document = Document::new()
        .set("viewBox", (0, 0, max_cell_x * 300, max_cell_y * 300));

    // background
    document.append(Rectangle::new()
        .set("x", 0)
        .set("y", 0)
        .set("width", max_cell_x * 300)
        .set("height", max_cell_y * 300)
        .set("fill", "white"));

    xml.render(&mut document);

    svg::save("map.svg", &document).unwrap();

    Ok(())
}
