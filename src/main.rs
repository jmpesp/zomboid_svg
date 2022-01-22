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
    #[serde(rename = "properties")]
    pub properties: Option<Properties>,
}

impl Feature {
    pub fn render(&self, document: &mut Document, bottom_left: &Point) {
        self.geometry.render(document, bottom_left, &self.properties)
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
    pub fn render(&self, document: &mut Document, bottom_left: &Point, properties: &Option<Properties>) {
        for coordinate in &self.coordinates {
            coordinate.render(document, bottom_left, properties)
        }
    }
}

#[derive(Deserialize, Debug)]
pub struct Coordinates {
    pub point: Vec<Point>,
}

impl Coordinates {
    pub fn render(&self, document: &mut Document, bottom_left: &Point, properties: &Option<Properties>) {
        let points: String = self.point
            .iter()
            .map(|p| {
                let adjusted_point = bottom_left.add(&p);
                format!("{},{}", adjusted_point.x, adjusted_point.y)
            })
            .collect::<Vec<String>>()
            .join(" ");

        let mut fill = "none";
        let mut stroke: Option<String> = Some("black".into());

        if let Some(properties) = properties {
            for property in &properties.property {
                if property.name == "water" {
                    fill = "blue";
                    stroke = None;
                }
            }
        }

        let mut polygon = Polygon::new();

        polygon.assign("fill", fill);

        if let Some(stroke) = stroke {
            polygon.assign("stroke", stroke);
            polygon.assign("stroke-width", 2);
        }

        polygon.assign("points", points);

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
}

#[derive(Deserialize, Debug)]
pub struct Properties {
    #[serde(rename = "property", default)]
    pub property: Vec<Property>,
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
