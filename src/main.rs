use crate::engine::renderer::Renderer;
mod engine;

fn main() {
    let mut renderer = Renderer::new("Attrition", 800, 600);
    renderer.set_clear_color(0.0, 0.0, 0.0, 1.0);
    renderer.run();  // handles everything, blocking until the window closes
}
