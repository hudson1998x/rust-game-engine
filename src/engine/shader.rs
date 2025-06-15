use gl::types::{GLenum, GLuint};

pub fn compile_shader(src: &str, kind: GLenum) -> GLuint {
    unsafe {
        let shader = gl::CreateShader(kind);
        gl::ShaderSource(shader, 1, [src.as_ptr() as *const _].as_ptr(), std::ptr::null());
        gl::CompileShader(shader);

        // Check compile status
        let mut status = 0;
        gl::GetShaderiv(shader, gl::COMPILE_STATUS, &mut status);
        if status == 0 {
            let mut len = 0;
            gl::GetShaderiv(shader, gl::INFO_LOG_LENGTH, &mut len);
            let mut buf = Vec::with_capacity(len as usize);
            gl::GetShaderInfoLog(shader, len, std::ptr::null_mut(), buf.as_mut_ptr() as *mut _);
            panic!("Shader compile error: {:?}", String::from_utf8_lossy(&buf));
        }

        shader
    }
}

pub fn create_shader_program(vs_src: &str, fs_src: &str) -> GLuint {
    unsafe {
        let vs = compile_shader(vs_src, gl::VERTEX_SHADER);
        let fs = compile_shader(fs_src, gl::FRAGMENT_SHADER);

        let program = gl::CreateProgram();
        gl::AttachShader(program, vs);
        gl::AttachShader(program, fs);
        gl::LinkProgram(program);

        // Check link status
        let mut status = 0;
        gl::GetProgramiv(program, gl::LINK_STATUS, &mut status);
        if status == 0 {
            panic!("Shader linking failed");
        }

        gl::DeleteShader(vs);
        gl::DeleteShader(fs);

        program
    }
}

#[derive(Clone, Debug)]
pub struct GLShaderProgram {

}

impl GLShaderProgram {
    pub fn set_uniform_matrix4(&self, name: &str, matrix: &[f32; 16]) {
        
    }
}
