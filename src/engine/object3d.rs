use std::{rc::{Rc, Weak}, cell::RefCell};
use std::cell::OnceCell;
use gl::{self, types::*};
use crate::engine::camera::{Camera};
use crate::engine::math::matrixfuncs::{compute_local_matrix, matrix_mul_4x4};
use crate::engine::shader::GLShaderProgram;

/// Represents a 3D object/node in a scene graph with position, rotation, scale,
/// and parent/children relationships for hierarchical transformations.
///
/// This struct maintains cached transformation matrices for efficiency and tracks
/// when updates are necessary using a 'dirty' flag.
///
/// Position, rotation, and scale describe the object's local transform relative to its parent.
/// The local transform matrix combines these three properties.
/// The world transform matrix represents the object's final transform in world space,
/// calculated by combining the local matrix with its parent's world matrix (if any).
///
/// The struct supports dynamic updating and caching to avoid redundant calculations,
/// especially useful in complex scenes with many objects.
#[derive(Debug)]
pub struct Object3D {
    /// The position of the object relative to its parent, represented as
    /// a 3D coordinate (x, y, z).
    pub position: [f32; 3],

    /// The rotation of the object relative to its parent, represented as a quaternion.
    /// Quaternions help avoid rotation issues like gimbal lock.
    /// Stored as [x, y, z, w] components.
    pub rotation: [f32; 4],

    /// The scale of the object relative to its parent, represented as scaling
    /// factors along the x, y, and z axes.
    pub scale: [f32; 3],

    /// Cached 4x4 matrix representing the local transformation (translation, rotation, scale).
    /// Stored in column-major order (used in many graphics APIs).
    local_matrix: [f32; 16],

    /// Cached 4x4 matrix representing the world transformation (combined local matrix and
    /// parent's world matrix).
    /// This is the final matrix used to position the object in the world.
    world_matrix: [f32; 16],

    /// Dirty flag indicating whether the local and/or world matrices need to be recalculated.
    /// When either the position, rotation, scale, or parent changes, this is set to true.
    dirty: bool,

    /// Weak reference to the parent object in the hierarchy.
    /// Weak reference is used to avoid reference cycles which can cause memory leaks.
    parent: Option<Weak<RefCell<Object3D>>>,

    /// Vector of strong references to children objects.
    /// Children are owned strongly to keep them alive as long as the parent exists.
    children: Vec<Rc<RefCell<Object3D>>>,

    /// Holds the geometry
    geometry: Option<Geometry>,

    /// Cached GL mesh built from the geometry (VAO, VBO, IBO).
    gl_mesh: OnceCell<GLMesh>,

    shader: Option<GLShaderProgram>

}

impl Object3D {
    /// Creates a new Object3D with default transform values:
    /// - position at origin (0, 0, 0)
    /// - identity rotation (no rotation)
    /// - uniform scale of 1 on all axes
    /// - identity matrices cached (no transform)
    /// The object initially marked dirty to force matrix calculation on first use.
    ///
    /// Returns a reference-counted, mutable Object3D wrapped in `Rc<RefCell<>>`
    /// to enable shared ownership and interior mutability.
    pub fn new() -> Rc<RefCell<Self>> {
        Rc::new(RefCell::new(Self {
            position: [0.0, 0.0, 0.0],
            rotation: [0.0, 0.0, 0.0, 1.0], // identity quaternion
            scale: [1.0, 1.0, 1.0],
            local_matrix: IDENTITY_MATRIX,
            world_matrix: IDENTITY_MATRIX,
            dirty: true,
            parent: None,
            children: Vec::new(),
            geometry: None,
            gl_mesh: OnceCell::new(),
            shader: None,
        }))
    }

    /// Adds a child to this object’s list of children.
    ///
    /// This sets the child's `parent` to this object,
    /// and marks the child (and its descendants) dirty to update matrices.
    ///
    /// `this` - the parent object (wrapped in Rc<RefCell<>>)
    /// `child` - the child object to add (also Rc<RefCell<>>)
    pub fn add_child(this: &Rc<RefCell<Self>>, child: Rc<RefCell<Self>>) {
        {
            // Mutably borrow the child to update its parent and dirty flag
            let mut child_borrow = child.borrow_mut();
            child_borrow.parent = Some(Rc::downgrade(this)); // store weak ref to avoid cycles
            child_borrow.mark_dirty(); // mark child and descendants dirty
        }
        // Mutably borrow self to add child to children vector
        this.borrow_mut().children.push(child);
    }

    /// Recursively marks this object and all its children as 'dirty',
    /// indicating their local/world matrices need recalculating.
    ///
    /// This method prevents redundant marking by checking the flag before propagating.
    fn mark_dirty(&mut self) {
        if !self.dirty {
            self.dirty = true;
            // Mark all children recursively to ensure entire subtree updates
            for child in &self.children {
                child.borrow_mut().mark_dirty();
            }
        }
    }

    pub fn set_geometry(&mut self, geometry: Geometry) {
        self.geometry = Option::from(geometry.to_owned());
        self.mark_dirty();
    }

    /// Updates the object's position and marks it dirty for recalculation.
    ///
    /// `pos` is the new position vector [x, y, z].
    pub fn set_position(&mut self, pos: [f32; 3]) {
        self.position = pos;
        self.mark_dirty();
    }

    /// Updates the object's rotation quaternion and marks it dirty.
    ///
    /// `rot` is the new quaternion [x, y, z, w].
    pub fn set_rotation(&mut self, rot: [f32; 4]) {
        self.rotation = rot;
        self.mark_dirty();
    }

    /// Updates the object's scale and marks it dirty.
    ///
    /// `scale` is the new scale vector [x, y, z].
    pub fn set_scale(&mut self, scale: [f32; 3]) {
        self.scale = scale;
        self.mark_dirty();
    }

    /// Returns the cached local transformation matrix.
    ///
    /// If the object is marked dirty, recomputes the matrix based on
    /// current position, rotation, and scale, caches it, then returns it.
    ///
    /// The local matrix represents the object's transform relative to its parent.
    pub fn local_matrix(&mut self) -> [f32; 16] {
        if self.dirty {
            self.local_matrix = compute_local_matrix(self.position, self.rotation, self.scale);
        }
        self.local_matrix
    }

    /// Returns the cached world transformation matrix.
    ///
    /// If the object is marked dirty, this method recomputes both the local matrix
    /// and the world matrix. The world matrix is computed by multiplying the parent's
    /// world matrix (if any) with this object's local matrix, effectively combining
    /// the transforms to produce a final world-space transformation.
    ///
    /// If there is no parent, the world matrix is the same as the local matrix.
    pub fn world_matrix(&mut self) -> [f32; 16] {
        if self.dirty {
            // Recompute the local matrix first if dirty
            self.local_matrix = compute_local_matrix(self.position, self.rotation, self.scale);

            // Compute world matrix by combining with parent's world matrix
            if let Some(ref weak_parent) = self.parent {
                if let Some(parent_rc) = weak_parent.upgrade() {
                    // Borrow parent mutably to get its world matrix (recursive update if needed)
                    let mut parent = parent_rc.borrow_mut();
                    let parent_world = parent.world_matrix();

                    // Multiply parent's world matrix by local matrix to get world matrix
                    self.world_matrix = matrix_mul_4x4(&parent_world, &self.local_matrix);
                } else {
                    // Parent was dropped; fallback to local matrix
                    self.world_matrix = self.local_matrix;
                }
            } else {
                // No parent, so world matrix is just local matrix
                self.world_matrix = self.local_matrix;
            }

            // Mark as clean (not dirty)
            self.dirty = false;
        }

        self.world_matrix
    }


    /// Renders the object if geometry is available and valid.
    ///
    /// Uploads vertex/index data to the GPU on the first draw call,
    /// then issues a glDrawElements command.
    /// Renders the object and all of its children using the provided shader and camera.
    ///
    /// Performs frustum culling and sets the "u_model" uniform before drawing.
    ///
    /// # Parameters
    /// - `shader`: Compiled OpenGL shader program used for rendering.
    /// - `camera`: The active camera providing projection and view matrices.
    /// - `frustum`: Frustum derived from the camera, used for basic culling.
    pub fn draw(&mut self, camera: &Camera) {
        // Recalculate transforms if needed
        let world_matrix = self.world_matrix();

        // Naive bounding-sphere culling: assume unit bounding radius
        let world_pos = [
            world_matrix[12],
            world_matrix[13],
            world_matrix[14],
        ];

        if !camera.intersects_sphere(world_pos, 1.0f32) {
            return; // skip drawing this object and its children
        }

        // Upload transform to shader

        if let Some(ref shader) = self.shader {
            shader.set_uniform_matrix4("u_model", &world_matrix);
            shader.set_uniform_matrix4("u_proj_view", &camera.proj_view_matrix());
        }

        // Draw geometry if present
        if let Some(mesh) = self.gl_mesh.get() {
            unsafe {
                gl::BindVertexArray(mesh.vao);
                gl::DrawElements(
                    gl::TRIANGLES,
                    mesh.index_count as GLsizei,
                    gl::UNSIGNED_SHORT,
                    std::ptr::null(),
                );
                gl::BindVertexArray(0);
            }
        }

        // Draw all children
        for child in &self.children {
            child.borrow_mut().draw(camera);
        }
    }

}

/// Vertex format storing position, normal, and uv texture coordinates.
/// Use `f32` as 3D floats are standard on GPUs.
#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct Vertex {
    pub position: [f32; 3], // x, y, z
    pub normal: [f32; 3],   // nx, ny, nz (for lighting)
    pub uv: [f32; 2],       // texture coordinates u, v
}

/// Index buffer using 16-bit indices for compactness.
/// Use u32 if you expect large meshes.
pub type Index = u16;

/// Represents the geometric data (mesh) used to define the shape of a 3D object.
///
/// This struct holds two primary buffers:
/// - **Vertices**: a list of points in 3D space, often including attributes like position,
///   normals, and texture coordinates.
/// - **Indices**: a list of integers that define how vertices are connected to form primitives
///   (typically triangles).
///
/// The separation of this data into a standalone `Geometry` struct allows for:
/// - Easy reuse of mesh data across multiple objects.
/// - Efficient memory and GPU data management.
/// - Support for procedural geometry generation (e.g., dynamic terrain, meshes built at runtime).
///
/// # Why Separate Geometry?
/// Keeping geometry separate from transform (`Object3D`) or scene nodes supports:
/// - **Decoupling:** Geometry doesn't carry world position or transform — that's handled by the node.
/// - **Efficiency:** Geometry can be shared between instances (e.g., multiple trees using the same mesh).
/// - **Flexibility:** Enables swapping or regenerating mesh data without affecting the rest of the scene graph.
///
/// # Fields
/// - `vertices`: A list of `Vertex` structs that define the attributes per vertex (e.g., position, normals, UVs).
/// - `indices`: A list of `Index` values that define the mesh's connectivity (which vertices make up each triangle).
///
/// # Example Usage
/// ```rust
/// let geometry = Geometry {
///     vertices: vec![/* ... */],
///     indices: vec![0, 1, 2, 2, 3, 0], // A simple quad made of two triangles
/// };
///
/// let mut object = Object3D::new();
/// object.borrow_mut().geometry = Some(geometry);
/// ```
///
/// # Performance Considerations
/// - Vertex/index buffers should be uploaded to GPU memory (e.g., via OpenGL VBO/IBO) during initialization.
/// - Regenerative geometry (e.g. dynamic terrain) should update buffers only when marked dirty.
/// - Use 16-bit indices (`u16`) when possible for lower memory footprint; switch to `u32` for large meshes.
///
/// # See Also
/// - [`Vertex`](struct.Vertex.html) – the structure defining per-vertex data.
/// - [`Object3D`](struct.Object3D.html) – where `Geometry` is attached for rendering.
///
/// # Note
/// This struct only holds CPU-side mesh data. Integration with GPU buffers (VBO/VAO) must be handled separately.
#[derive(Clone, Debug)]
pub struct Geometry {
    /// A list of vertices that define the shape of the mesh.
    ///
    /// Each vertex typically contains:
    /// - Position (x, y, z)
    /// - Normal (nx, ny, nz)
    /// - Texture coordinates (u, v)
    pub vertices: Vec<Vertex>,

    /// A list of indices defining how to connect vertices into primitives (typically triangles).
    ///
    /// These indices reference positions in the `vertices` array.
    /// For example, [0, 1, 2] creates one triangle using the first three vertices.
    pub indices: Vec<Index>,
}

/// Internal OpenGL mesh representation. Automatically created from Geometry.
#[derive(Debug)]
pub struct GLMesh {
    pub vao: GLuint,
    pub vbo: GLuint,
    pub ibo: GLuint,
    pub index_count: usize,
}

// -- Constants --
/// Identity matrix (4x4) representing 'no transformation'.
/// This matrix leaves points unchanged when multiplied.
const IDENTITY_MATRIX: [f32; 16] = [
    1.0, 0.0, 0.0, 0.0,  // Column 1
    0.0, 1.0, 0.0, 0.0,  // Column 2
    0.0, 0.0, 1.0, 0.0,  // Column 3
    0.0, 0.0, 0.0, 1.0,  // Column 4
];

