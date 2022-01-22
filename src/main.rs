use std::collections::BTreeMap;
use std::str::FromStr;

use serde::{Deserialize, Deserializer};

use svg::Document;
use svg::Node;
use svg::node::element::{Polygon, Rectangle, Text};

#[derive(Deserialize, Debug)]
pub struct World {
    pub cell: Vec<Cell>,
}

impl World {
    pub fn render(&self, svg_layers: &mut SVGLayers) {
        for cell in &self.cell {
            cell.render(svg_layers);
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

    pub fn render(&self, svg_layers: &mut SVGLayers) {
        for feature in &self.feature {
            feature.render(svg_layers, &self.bottom_left())
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
    pub fn render(&self, svg_layers: &mut SVGLayers, bottom_left: &Point) {
        self.geometry.render(svg_layers, bottom_left, &self.properties)
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
    pub fn render(&self, svg_layers: &mut SVGLayers, bottom_left: &Point, properties: &Option<Properties>) {
        for coordinate in &self.coordinates {
            match self.geometry_type {
                GeometryType::LineString => {
                },
                GeometryType::Polygon => {
                    coordinate.render_polygon(svg_layers, bottom_left, properties)
                },
                GeometryType::Point => {
                    coordinate.render_point(svg_layers, bottom_left, properties)
                }
            }
        }
    }
}

#[derive(Deserialize, Debug)]
pub struct Coordinates {
    pub point: Vec<Point>,
}

impl Coordinates {
    pub fn render_polygon(&self, svg_layers: &mut SVGLayers, bottom_left: &Point, properties: &Option<Properties>) {
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
        let mut layer_key = "polygons".into();

        if let Some(properties) = properties {
            for property in &properties.property {
                /*
                 */
                if property.name == "water" {
                    layer_key = "water".into();
                    fill = "blue";
                    stroke = None;
                } else if property.name == "natural" && property.value == "wood" {
                    fill = "green";
                    stroke = None;
                } else if property.name == "building" {
                    if property.value == "Medical" {
                        layer_key = "medical".into();
                        fill = "red";
                        stroke = None;
                    }
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

        svg_layers.add_to_layer(layer_key, polygon.into());
    }

    pub fn render_point(&self, svg_layers: &mut SVGLayers, bottom_left: &Point, properties: &Option<Properties>) {
        assert_eq!(self.point.len(), 1);
        let point = &self.point[0];
        let adjusted_point = bottom_left.add(&point);

        if let Some(properties) = properties {
            for property in &properties.property {
                if property.name == "name_en" {
                    let mut text = Text::new();
                    text.assign("x", adjusted_point.x);
                    text.assign("y", adjusted_point.y);
                    text.assign("font-family", "Verdana");
                    text.assign("font-size", "64");
                    text.assign("fill", "blue");
                    text.append(svg::node::Text::new(property.value.clone()));

                    svg_layers.add_to_layer("text".into(), text.into());
                }
            }
        }
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

pub struct SVGLayers {
    min_x: i32,
    max_x: i32,
    min_y: i32,
    max_y: i32,

    pub layers: BTreeMap<String, Document>,
}

impl SVGLayers {
    pub fn new(min_x: i32, min_y: i32, max_x: i32, max_y: i32) -> Self {
        Self {
            min_x,
            min_y,
            max_x,
            max_y,
            layers: BTreeMap::default(),
        }
    }

    fn get_layer(&mut self, key: String) -> &mut Document {
        self.layers.entry(key).or_insert_with(|| {
            Document::new()
                .set("viewBox", (self.min_x, self.min_y, self.max_x, self.max_y))
        })
    }

    pub fn add_to_layer(&mut self, key: String, node: svg::node::element::Element) {
        self.get_layer(key).append(node.clone());
        self.get_layer("map".into()).append(node);
    }

    pub fn save(&self) {
        for (key, layer) in &self.layers {
            svg::save(format!("{}.svg", key), layer).unwrap();
        }
    }
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

    let mut svg_layers = SVGLayers::new(
        min_cell_x * 300, min_cell_y * 300,
        max_cell_x * 300, max_cell_y * 300,
    );

    svg_layers.add_to_layer(
        "background".into(),
        Rectangle::new()
            .set("x", 0)
            .set("y", 0)
            .set("width", max_cell_x * 300)
            .set("height", max_cell_y * 300)
            .set("fill", "white").into(),
        );

    xml.render(&mut svg_layers);

    svg_layers.save();

    Ok(())
}
