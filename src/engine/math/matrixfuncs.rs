
// -- Helper functions -- //

/// Computes the local transformation matrix by combining translation, rotation, and scale matrices.
///
/// # Parameters
/// - `position`: the local position [x, y, z].
/// - `rotation`: the local rotation quaternion [x, y, z, w].
/// - `scale`: the local scale factors [x, y, z].
///
/// # Returns
/// A 4x4 transformation matrix in column-major order representing the combined local transform.
pub fn compute_local_matrix(position: [f32;3], rotation: [f32;4], scale: [f32;3]) -> [f32;16] {
    // Compute individual transformation matrices
    let translation = translation_matrix(position);
    let rotation = rotation_matrix_from_quat(rotation);
    let scale = scale_matrix(scale);

    // Multiply translation * rotation * scale (order matters)
    matrix_mul_4x4(&matrix_mul_4x4(&translation, &rotation), &scale)
}

/// Creates a translation matrix from a position vector.
///
/// Moves points by the specified x, y, z amounts.
///
/// # Returns
/// A 4x4 translation matrix.
pub fn translation_matrix(pos: [f32; 3]) -> [f32; 16] {
    [
        1.0, 0.0, 0.0, 0.0,      // Column 1
        0.0, 1.0, 0.0, 0.0,      // Column 2
        0.0, 0.0, 1.0, 0.0,      // Column 3
        pos[0], pos[1], pos[2], 1.0,  // Column 4 (translation components)
    ]
}

/// Creates a scale matrix from scale factors.
///
/// Scales points along x, y, and z axes.
///
/// # Returns
/// A 4x4 scale matrix.
pub fn scale_matrix(scale: [f32; 3]) -> [f32; 16] {
    [
        scale[0], 0.0,      0.0,      0.0,  // Column 1
        0.0,      scale[1], 0.0,      0.0,  // Column 2
        0.0,      0.0,      scale[2], 0.0,  // Column 3
        0.0,      0.0,      0.0,      1.0,  // Column 4
    ]
}

/// Converts a quaternion rotation into a 4x4 rotation matrix.
///
/// The quaternion is given as [x, y, z, w].
/// This matrix can be multiplied with other transforms.
///
/// # Returns
/// A 4x4 rotation matrix in column-major order.
pub fn rotation_matrix_from_quat(q: [f32; 4]) -> [f32; 16] {
    let x = q[0];
    let y = q[1];
    let z = q[2];
    let w = q[3];

    // Precompute products to simplify matrix
    let xx = x * x;
    let yy = y * y;
    let zz = z * z;
    let xy = x * y;
    let xz = x * z;
    let yz = y * z;
    let wx = w * x;
    let wy = w * y;
    let wz = w * z;

    [
        1.0 - 2.0 * (yy + zz),  2.0 * (xy + wz),        2.0 * (xz - wy),        0.0,  // Column 1
        2.0 * (xy - wz),        1.0 - 2.0 * (xx + zz),  2.0 * (yz + wx),        0.0,  // Column 2
        2.0 * (xz + wy),        2.0 * (yz - wx),        1.0 - 2.0 * (xx + yy),  0.0,  // Column 3
        0.0,                    0.0,                    0.0,                    1.0,  // Column 4
    ]
}

/// Multiplies two 4x4 matrices `a` and `b` (both in column-major order).
///
/// The multiplication is `result = a * b`, where each matrix is 4x4.
///
/// # Returns
/// The resulting 4x4 matrix from the multiplication.
pub fn matrix_mul_4x4(a: &[f32; 16], b: &[f32; 16]) -> [f32; 16] {
    let mut result = [0.0f32; 16];

    // Multiply rows of a by columns of b
    for row in 0..4 {
        for col in 0..4 {
            result[col * 4 + row] =
                a[0 * 4 + row] * b[col * 4 + 0] +
                    a[1 * 4 + row] * b[col * 4 + 1] +
                    a[2 * 4 + row] * b[col * 4 + 2] +
                    a[3 * 4 + row] * b[col * 4 + 3];
        }
    }

    result
}

pub fn perspective_matrix(fovy: f32, aspect: f32, near: f32, far: f32) -> [f32; 16] {
    let f = 1.0 / (fovy / 2.0).tan();
    let nf = 1.0 / (near - far);

    [
        f / aspect, 0.0, 0.0, 0.0,
        0.0, f, 0.0, 0.0,
        0.0, 0.0, (far + near) * nf, -1.0,
        0.0, 0.0, (2.0 * far * near) * nf, 0.0,
    ]
}