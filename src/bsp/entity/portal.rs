use std::collections::HashMap;

use glam::DVec3;

use crate::bsp::{Data, texture::Kind};

use super::property;

const MODEL: &str = "model";
const URI: &str = "uri";
const SIZE: DVec3 = DVec3::new(128.0, 96.0, 16.0);
const POSITION_TOLERANCE: f64 = 0.01;
const NORMAL_TOLERANCE: f64 = 0.0001;

#[derive(Debug, PartialEq)]
pub struct Portal {
    pub model: Option<usize>,
    pub uri: Option<String>,
    pub validation: Validation,
}

#[derive(Debug, PartialEq)]
pub enum Validation {
    Valid(Geometry),
    Invalid(String),
}

#[derive(Debug, PartialEq)]
pub struct Geometry {
    /// Center of the marked face in BSP coordinates.
    pub front_center: [f32; 3],
    /// Outward unit normal of the marked face.
    pub front_normal: [f32; 3],
}

// Compute the area vector of an ordered planar polygon.
fn area_vector(points: &[DVec3]) -> DVec3 {
    let origin = points[0];
    points[1..]
        .windows(2)
        .map(|edge| (edge[0] - origin).cross(edge[1] - origin))
        .sum::<DVec3>()
        * 0.5
}

// Validate the dimensions, six sides, and marked front in doorway coordinates.
fn geometry(bsp: &Data, model: usize) -> Result<Geometry, String> {
    let mut polygons = Vec::new();
    let mut front = None;
    for index in bsp.models()[model].faces.clone() {
        let face = &bsp.faces()[index];
        let texture_id = bsp.texinfo()[face.texinfo].texture;
        let marked = bsp.textures()[texture_id]
            .as_ref()
            .is_some_and(|texture| texture.kind() == Kind::Portal);
        let points: Vec<_> = bsp
            .positions(index)
            .map(|position| DVec3::from_array(position.map(f64::from)))
            .collect();
        if points.len() < 3 || points.iter().any(|point| !point.is_finite()) {
            return Err("a brush face has invalid vertices".into());
        }
        if marked && front.is_none() {
            front = Some((polygons.len(), texture_id));
        }
        polygons.push((texture_id, marked, points));
    }
    let (front_index, front_texture) =
        front.ok_or_else(|| "no side uses a portal texture".to_owned())?;
    let front_points = &polygons[front_index].2;
    let origin = front_points[0];
    let normal = area_vector(front_points)
        .try_normalize()
        .filter(|normal| normal.z.abs() <= NORMAL_TOLERANCE)
        .ok_or_else(|| {
            "the portal-textured face must be vertical and have nonzero area".to_owned()
        })?;

    let forward = DVec3::new(normal.x, normal.y, 0.0).normalize();
    let right = DVec3::Z.cross(forward);
    let mut min = DVec3::splat(f64::INFINITY);
    let mut max = DVec3::splat(f64::NEG_INFINITY);
    for (_, _, points) in &mut polygons {
        for point in points {
            let offset = *point - origin;
            *point = DVec3::new(offset.dot(right), offset.z, offset.dot(forward));
            min = min.min(*point);
            max = max.max(*point);
        }
    }

    let front_is_max = max.z.abs() <= POSITION_TOLERANCE;
    let front_depth = if front_is_max { max.z } else { min.z };
    if front_depth.abs() > POSITION_TOLERANCE {
        return Err("the portal-textured face must be on the exterior of the cuboid".into());
    }
    let size = max - min;
    if !size.abs_diff_eq(SIZE, POSITION_TOLERANCE) {
        return Err(format!(
            "brush is {:.3} × {:.3} × {:.3}, expected {} × {} × {} (width × height × depth)",
            size.x, size.y, size.z, SIZE.x, SIZE.y, SIZE.z
        ));
    }

    let mut side_areas = [[0.0; 2]; 3];
    for (texture, marked, points) in &polygons {
        let on_front = points
            .iter()
            .all(|point| (point.z - front_depth).abs() <= POSITION_TOLERANCE);
        if *marked && (!on_front || *texture != front_texture) {
            return Err(
                "only the front side may use a portal texture, and it must use one texture".into(),
            );
        }
        if on_front && !*marked {
            return Err("the entire front side must use the portal texture".into());
        }
        let (axis, side) = (0..3)
            .flat_map(|axis| (0..2).map(move |side| (axis, side)))
            .find(|&(axis, side)| {
                let boundary = if side == 0 { min[axis] } else { max[axis] };
                points
                    .iter()
                    .all(|point| (point[axis] - boundary).abs() <= POSITION_TOLERANCE)
            })
            .ok_or_else(|| "a brush face is outside the six cuboid sides".to_owned())?;
        let area = area_vector(points)[axis].abs();
        if area <= POSITION_TOLERANCE {
            return Err("a brush face has zero area".into());
        }
        side_areas[axis][side] += area;
    }
    for (axis, areas) in side_areas.into_iter().enumerate() {
        let width = size[(axis + 1) % 3];
        let height = size[(axis + 2) % 3];
        let tolerance = POSITION_TOLERANCE * (width + height);
        if areas
            .into_iter()
            .any(|area| (area - width * height).abs() > tolerance)
        {
            return Err("brush faces must cover all six cuboid sides".into());
        }
    }
    for corner in 0..8 {
        let corner = DVec3::from_array(std::array::from_fn(|axis| {
            if corner & (1 << axis) == 0 {
                min[axis]
            } else {
                max[axis]
            }
        }));
        if !polygons
            .iter()
            .flat_map(|(_, _, points)| points)
            .any(|point| point.abs_diff_eq(corner, POSITION_TOLERANCE))
        {
            return Err("brush is missing a cuboid corner".into());
        }
    }

    let middle = (min + max) * 0.5;
    let center = origin + right * middle.x + DVec3::Z * middle.y + forward * front_depth;
    let outward = if front_is_max { forward } else { -forward };
    Ok(Geometry {
        front_center: center.as_vec3().to_array(),
        front_normal: outward.as_vec3().to_array(),
    })
}

impl Portal {
    /// Parse the portal and retain a diagnostic when its properties or brush are invalid.
    pub fn parse(properties: &HashMap<String, String>, bsp: &Data) -> Self {
        let model = property::model(properties, MODEL, bsp.models().len());
        let uri = property::string(properties, URI).filter(|uri| {
            !uri.chars()
                .any(|character| character.is_whitespace() || character.is_control())
        });
        let validation = match (&model, &uri) {
            (Err(error), _) => Validation::Invalid(error.to_string()),
            (Ok(_), None) => Validation::Invalid("missing or invalid uri property".into()),
            (Ok(index), Some(_)) => match geometry(bsp, *index) {
                Ok(geometry) => Validation::Valid(geometry),
                Err(error) => Validation::Invalid(format!("model *{index}: {error}")),
            },
        };
        Self {
            model: model.ok(),
            uri,
            validation,
        }
    }
}
