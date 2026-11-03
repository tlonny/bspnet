use std::io::{Error, ErrorKind, Result};

use glam::Vec3;

mod edge;
mod face;
mod leaf;
mod marksurface;
mod model;
mod node;
mod plane;
mod surfedge;
mod texinfo;
mod texture;
mod vertex;

pub use face::Face;
pub use leaf::Leaf;
pub use model::Model;
pub use node::Node;
pub use plane::Plane;
pub use texinfo::Texinfo;
pub use texture::Texture;

const VERSION: i32 = 29;
const HEADER_SIZE: usize = 124;
const PLANES_ENTRY: usize = 4 + 8;
const TEXTURES_ENTRY: usize = 4 + 2 * 8;
const VERTICES_ENTRY: usize = 4 + 3 * 8;
const VISIBILITY_ENTRY: usize = 4 + 4 * 8;
const NODES_ENTRY: usize = 4 + 5 * 8;
const TEXINFO_ENTRY: usize = 4 + 6 * 8;
const FACES_ENTRY: usize = 4 + 7 * 8;
const LEAVES_ENTRY: usize = 4 + 10 * 8;
const MARKSURFACES_ENTRY: usize = 4 + 11 * 8;
const EDGES_ENTRY: usize = 4 + 12 * 8;
const SURFEDGES_ENTRY: usize = 4 + 13 * 8;
const MODELS_ENTRY: usize = 4 + 14 * 8;

#[cfg(test)]
mod test;

// Locate a lump using its directory entry in an already validated header.
fn lump(bytes: &[u8], entry: usize) -> Result<&[u8]> {
    let entry = &bytes[entry..entry + 8];
    let offset = i32::from_le_bytes(entry[..4].try_into().unwrap());
    let length = i32::from_le_bytes(entry[4..].try_into().unwrap());
    if offset < HEADER_SIZE as i32 || length < 0 {
        return Err(Error::new(
            ErrorKind::InvalidData,
            "invalid lump offset or length",
        ));
    }
    bytes
        .get(offset as usize..)
        .and_then(|remaining| remaining.get(..length as usize))
        .ok_or_else(|| Error::new(ErrorKind::InvalidData, "lump outside BSP file"))
}

#[derive(Debug)]
pub struct Data {
    pub planes: Vec<Plane>,
    pub nodes: Vec<Node>,
    pub leaves: Vec<Leaf>,
    /// Compressed PVS data; leaf visibility offsets address this byte array.
    pub visibility: Vec<u8>,
    /// Face indices referenced by each leaf's marksurface range.
    pub marksurfaces: Vec<u16>,
    /// Texture metadata in directory order; pixels are loaded separately.
    pub textures: Vec<Texture>,
    pub texinfo: Vec<Texinfo>,
    /// Positions in the file's original Quake coordinates and map units.
    pub vertices: Vec<Vec3>,
    /// Pairs of indices into `vertices`, in the file's original edge order.
    pub edges: Vec<[u16; 2]>,
    /// Signed edge indices; negative values traverse the edge in reverse.
    pub surfedges: Vec<i32>,
    pub faces: Vec<Face>,
    /// Model 0 is the world; subsequent models describe brush entities.
    pub models: Vec<Model>,
}

impl Data {
    /// Find a world leaf for a position in the BSP's original Quake coordinates.
    /// Points on a splitting plane follow its back child, matching Quake.
    pub fn leaf_at(&self, position: Vec3) -> Result<usize> {
        if !position.is_finite() {
            return Err(Error::new(
                ErrorKind::InvalidInput,
                "non-finite leaf query position",
            ));
        }
        let mut child = self
            .models
            .first()
            .ok_or_else(|| Error::new(ErrorKind::InvalidData, "missing world model"))?
            .headnode;
        // A valid path cannot visit more nodes than the tree contains.
        for _ in 0..=self.nodes.len() {
            if child < 0 {
                let index = child.unsigned_abs() as usize - 1;
                return self.leaves.get(index).map(|_| index).ok_or_else(|| {
                    Error::new(ErrorKind::InvalidData, "tree references missing leaf")
                });
            }
            let node = self.nodes.get(child as usize).ok_or_else(|| {
                Error::new(ErrorKind::InvalidData, "tree references missing node")
            })?;
            let plane = usize::try_from(node.plane)
                .ok()
                .and_then(|index| self.planes.get(index))
                .ok_or_else(|| {
                    Error::new(ErrorKind::InvalidData, "node references missing plane")
                })?;
            let front = plane.normal.dot(position) > plane.distance;
            child = i32::from(node.children[usize::from(!front)]);
        }
        Err(Error::new(ErrorKind::InvalidData, "cycle in BSP node tree"))
    }

    /// Decode geometry and texture metadata from a Quake BSP29 file.
    pub fn decode(bytes: &[u8]) -> Result<Self> {
        let header = bytes
            .get(..HEADER_SIZE)
            .ok_or_else(|| Error::new(ErrorKind::InvalidData, "truncated BSP header"))?;
        let version = i32::from_le_bytes(header[..4].try_into().unwrap());
        if version != VERSION {
            return Err(Error::new(ErrorKind::InvalidData, "expected Quake BSP29"));
        }

        let planes = plane::decode(lump(bytes, PLANES_ENTRY)?)?;
        let nodes = node::decode(lump(bytes, NODES_ENTRY)?)?;
        let leaves = leaf::decode(lump(bytes, LEAVES_ENTRY)?)?;
        let visibility = lump(bytes, VISIBILITY_ENTRY)?.to_vec();
        let marksurfaces = marksurface::decode(lump(bytes, MARKSURFACES_ENTRY)?)?;
        let textures = texture::decode(lump(bytes, TEXTURES_ENTRY)?)?;
        let texinfo = texinfo::decode(lump(bytes, TEXINFO_ENTRY)?)?;
        let vertices = vertex::decode(lump(bytes, VERTICES_ENTRY)?)?;
        let edges = edge::decode(lump(bytes, EDGES_ENTRY)?)?;
        let surfedges = surfedge::decode(lump(bytes, SURFEDGES_ENTRY)?)?;
        let faces = face::decode(lump(bytes, FACES_ENTRY)?)?;
        let models = model::decode(lump(bytes, MODELS_ENTRY)?)?;
        Ok(Self {
            planes,
            nodes,
            leaves,
            visibility,
            marksurfaces,
            textures,
            texinfo,
            vertices,
            edges,
            surfedges,
            faces,
            models,
        })
    }
}
