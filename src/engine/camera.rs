//! Camera and view frustum utilities for 3D rendering.
//!
//! This module provides foundational structures for viewing and culling in a 3D scene graph-based renderer.
//! It includes a `Camera` for perspective projection and a simplified `Frustum` for spatial visibility testing.

use crate::engine::math::matrixfuncs::{matrix_mul_4x4, perspective_matrix, rotation_matrix_from_quat, translation_matrix};

/// Represents a perspective projection camera in a 3D scene.
///
/// This camera uses a right-handed coordinate system and outputs column-major 4x4 transformation matrices
/// suitable for use in OpenGL or other graphics APIs.
///
/// The camera tracks:
/// - Position and rotation (as a quaternion)
/// - Perspective projection parameters (field of view, aspect ratio, near/far planes)
///
/// Use `view_matrix()` and `projection_matrix()` to obtain camera transforms for use in rendering.
///
/// # Example
/// ```
/// let mut camera = Camera::new(16.0 / 9.0);
/// let view = camera.view_matrix();
/// let proj = camera.projection_matrix();
/// let proj_view = camera.proj_view_matrix();
/// ```
#[derive(Debug, Clone)]
pub struct Camera {
    /// The camera's world-space position.
    pub position: [f32; 3],

    /// The camera's orientation represented as a unit quaternion `[x, y, z, w]`.
    /// Defaults to identity (facing -Z).
    pub rotation: [f32; 4],

    /// Vertical field of view in radians.
    pub fov_y: f32,

    /// Aspect ratio of the view (width / height).
    pub aspect: f32,

    /// Distance to the near clipping plane.
    pub near: f32,

    /// Distance to the far clipping plane.
    pub far: f32,
}

impl Camera {
    /// Creates a new camera with default parameters and given aspect ratio.
    ///
    /// Defaults:
    /// - Position: `[0.0, 0.0, 5.0]` (looking toward the origin)
    /// - Rotation: identity quaternion
    /// - FOV: 60 degrees vertical
    /// - Near/Far: 0.1 / 100.0
    ///
    /// # Parameters
    /// - `aspect`: Width-to-height ratio of the viewport.
    pub fn new(aspect: f32) -> Self {
        Self {
            position: [0.0, 0.0, 5.0],
            rotation: [0.0, 0.0, 0.0, 1.0],
            fov_y: 60.0_f32.to_radians(),
            aspect,
            near: 0.1,
            far: 100.0
        }
    }

    /// Updates the object's position and marks it dirty for recalculation.
    ///
    /// `pos` is the new position vector [x, y, z].
    pub fn set_position(&mut self, pos: [f32; 3]) {
        self.position = pos;
    }

    /// Updates the object's rotation quaternion and marks it dirty.
    ///
    /// `rot` is the new quaternion [x, y, z, w].
    pub fn set_rotation(&mut self, rot: [f32; 4]) {
        self.rotation = rot;
    }

    /// Sets the camera's Near & Far ranges
    pub fn set_near_far(&mut self, near: f32, far: f32) {
        self.near = near;
        self.far = far;
    }

    /// Sets the camera's FOV
    pub fn set_fov(&mut self, fov: f32) {
        self.fov_y = fov.to_radians();
    }

    /// Computes the view matrix from the camera's position and rotation.
    ///
    /// This transform converts world-space coordinates into view-space,
    /// where the camera is at the origin looking down the negative Z-axis.
    ///
    /// # Returns
    /// A 4x4 column-major view matrix.
    pub fn view_matrix(&self) -> [f32; 16] {
        let rot_matrix = rotation_matrix_from_quat(self.rotation);
        let trans_matrix = translation_matrix([
            -self.position[0],
            -self.position[1],
            -self.position[2],
        ]);

        matrix_mul_4x4(&rot_matrix, &trans_matrix)
    }

    /// Computes the perspective projection matrix based on the camera's FOV, aspect ratio, and near/far planes.
    ///
    /// # Returns
    /// A 4x4 column-major perspective projection matrix.
    pub fn projection_matrix(&self) -> [f32; 16] {
        perspective_matrix(self.fov_y, self.aspect, self.near, self.far)
    }

    /// Returns the combined projection * view matrix for transforming world-space coordinates
    /// directly into clip space.
    pub fn proj_view_matrix(&self) -> [f32; 16] {
        matrix_mul_4x4(&self.projection_matrix(), &self.view_matrix())
    }

    /// Performs a simple bounding-sphere culling test in clip space.
    ///
    /// Transforms the world-space center of the bounding sphere into clip space
    /// and checks whether the Z component lies within the canonical clip space range (-1 to +1).
    ///
    /// # Parameters
    /// - `world_pos`: Center of the object in world coordinates.
    /// - `radius`: Radius of the object's bounding sphere.
    ///
    /// # Returns
    /// `true` if the object may be visible; `false` if it is fully outside the Z frustum.
    pub fn intersects_sphere(&self, world_pos: [f32; 3], radius: f32) -> bool {
        let m = &self.proj_view_matrix();
        let clip_z =
            m[2] * world_pos[0] +
                m[6] * world_pos[1] +
                m[10] * world_pos[2] +
                m[14];

        // Z-only depth clip test (simplified)
        clip_z + radius > -1.0 && clip_z - radius < 1.0
    }
}