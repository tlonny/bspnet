use std::io::{Error, ErrorKind, Result};

mod clipnode;
mod contents;
mod directory;
mod edge;
mod entity;
mod face;
mod leaf;
mod marksurface;
mod model;
mod node;
mod plane;
pub mod sidecar;
mod surfedge;
mod texinfo;
pub mod texture;
mod vertex;
mod visibility;

pub use clipnode::{Child as ClipChild, Clipnode};
pub use contents::Contents;
pub use edge::Edge;
pub use entity::{Entity, Worldspawn};
pub use face::Face;
pub use leaf::Leaf;
pub use model::{ClipRoot, Model, RenderRoot};
pub use node::{Child, Node};
pub use plane::Plane;
pub use surfedge::{Direction, Surfedge};
pub use texinfo::Texinfo;
pub use texture::Texture;
pub use vertex::Vertex;
pub use visibility::Visibility;

#[cfg(test)]
mod test;

#[derive(Debug)]
pub struct Data {
    entities: Vec<Entity>,
    planes: Vec<Plane>,
    marksurfaces: Vec<usize>,
    vertices: Vec<Vertex>,
    edges: Vec<Edge>,
    surfedges: Vec<Surfedge>,
    faces: Vec<Face>,
    models: Vec<Model>,
    nodes: Vec<Node>,
    clipnodes: Vec<Clipnode>,
    visibility: Visibility,
    texinfo: Vec<Texinfo>,
    textures: Vec<Option<Texture>>,
    leaves: Vec<Leaf>,
}

impl Data {
    /// Borrow parsed entities in their original lump order.
    pub fn entities(&self) -> &[Entity] {
        &self.entities
    }

    /// Borrow splitting planes in their original lump order.
    pub fn planes(&self) -> &[Plane] {
        &self.planes
    }

    /// Borrow the face indices referenced by render leaves.
    pub fn marksurfaces(&self) -> &[usize] {
        &self.marksurfaces
    }

    /// Find a world leaf in Quake coordinates; invalid positions or cyclic nodes return None.
    pub fn leaf_at(&self, position: [f32; 3]) -> Option<usize> {
        if position.iter().any(|value| !value.is_finite()) {
            return None;
        }
        let mut child = match self.models[0].render_root {
            RenderRoot::Node(index) => Child::Node(index),
            RenderRoot::Leaf(index) => Child::Leaf(index),
        };
        for _ in 0..=self.nodes.len() {
            match child {
                Child::Leaf(index) => return Some(index),
                Child::Node(index) => {
                    let node = &self.nodes[index];
                    let plane = &self.planes[node.plane];
                    let distance = position
                        .into_iter()
                        .zip(plane.normal)
                        .map(|(position, normal)| f64::from(position) * f64::from(normal))
                        .sum::<f64>()
                        - f64::from(plane.distance);
                    child = if distance > 0.0 {
                        node.front
                    } else {
                        node.back
                    };
                }
            }
        }
        None
    }

    /// Borrow vertices in their original lump order and Quake coordinates.
    pub fn vertices(&self) -> &[Vertex] {
        &self.vertices
    }

    /// Borrow edges in their original lump order, including the unused edge zero.
    pub fn edges(&self) -> &[Edge] {
        &self.edges
    }

    /// Borrow directed edge references in their original lump order.
    pub fn surfedges(&self) -> &[Surfedge] {
        &self.surfedges
    }

    /// Borrow face records without resolving references into other lumps.
    pub fn faces(&self) -> &[Face] {
        &self.faces
    }

    /// Iterate a face's corner positions in surfedge winding order.
    pub fn positions(&self, face_id: usize) -> impl Iterator<Item = [f32; 3]> + '_ {
        self.surfedges[self.faces[face_id].surfedges.clone()]
            .iter()
            .map(|surfedge| {
                let edge = &self.edges[surfedge.edge];
                let endpoint = match surfedge.direction {
                    Direction::Forward => 0,
                    Direction::Reverse => 1,
                };
                self.vertices[edge.vertices[endpoint]].position
            })
    }

    /// Borrow models in lump order; model zero is the world.
    pub fn models(&self) -> &[Model] {
        &self.models
    }

    /// Borrow render nodes in their original lump order.
    pub fn nodes(&self) -> &[Node] {
        &self.nodes
    }

    /// Borrow render leaves in their original lump order, including solid leaf zero.
    pub fn leaves(&self) -> &[Leaf] {
        &self.leaves
    }

    /// Borrow collision nodes in their original lump order.
    pub fn clipnodes(&self) -> &[Clipnode] {
        &self.clipnodes
    }

    /// Borrow the flattened visible leaf indices, excluding solid leaf zero.
    pub fn visibility(&self) -> &[usize] {
        &self.visibility
    }

    /// Borrow indexed textures in directory order, preserving missing entries.
    pub fn textures(&self) -> &[Option<Texture>] {
        &self.textures
    }

    /// Borrow texture projection records in their original lump order.
    pub fn texinfo(&self) -> &[Texinfo] {
        &self.texinfo
    }

    /// Decode the supported lumps of a Quake BSP29 file, requiring world model zero.
    pub fn decode(bytes: &[u8]) -> Result<Self> {
        let directory = directory::decode(bytes)?;
        let entities = entity::decode(&bytes[directory.entities])?;
        let planes = plane::decode(&bytes[directory.planes])?;
        let vertices = vertex::decode(&bytes[directory.vertices])?;
        let edges = edge::decode(&bytes[directory.edges], vertices.len())?;
        let surfedges = surfedge::decode(&bytes[directory.surfedges], edges.len())?;
        let textures = texture::decode(&bytes[directory.textures])?;
        let texinfo = texinfo::decode(&bytes[directory.texinfo], &textures)?;
        let faces = face::decode(
            &bytes[directory.faces],
            surfedges.len(),
            texinfo.len(),
            planes.len(),
        )?;
        let marksurfaces = marksurface::decode(&bytes[directory.marksurfaces], faces.len())?;
        let clipnodes = clipnode::decode(&bytes[directory.clipnodes], planes.len())?;
        let nodes = node::decode(&bytes[directory.nodes], faces.len(), planes.len())?;
        let models = model::decode(
            &bytes[directory.models],
            faces.len(),
            nodes.len(),
            clipnodes.len(),
        )?;
        let world = models
            .first()
            .ok_or_else(|| Error::new(ErrorKind::InvalidData, "missing world model"))?;
        let (visibility, visibility_ranges) =
            visibility::decode(&bytes[directory.visibility], world.visleafs)?;
        let leaves = leaf::decode(
            &bytes[directory.leaves],
            &visibility_ranges,
            marksurfaces.len(),
        )?;
        for node in &nodes {
            node.validate_leaves(leaves.len())?;
        }
        for model in &models {
            model.validate_leaves(leaves.len())?;
        }
        Ok(Self {
            entities,
            planes,
            marksurfaces,
            vertices,
            edges,
            surfedges,
            faces,
            models,
            nodes,
            clipnodes,
            visibility,
            texinfo,
            textures,
            leaves,
        })
    }
}
