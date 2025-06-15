use crate::engine::renderer::Renderer;
use crate::engine::camera::Camera;

mod engine;

fn main() {
    let mut renderer = Renderer::new("My Game", 800, 600);
    renderer.set_clear_color(0.0, 0.0, 0.0, 1.0);
    
    let mut camera = Camera::new((800 / 600) as f32);
    camera.set_fov(90f32);
    camera.set_near_far(0.01, 1000.00);
    
    
    
    renderer.run();  // handles everything, blocking until the window closes
}
