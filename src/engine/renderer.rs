use glutin::{
    dpi::PhysicalSize,
    event::{Event, WindowEvent},
    event_loop::{ControlFlow, EventLoop},
    window::WindowBuilder,
    ContextBuilder,
    ContextWrapper,
    PossiblyCurrent,
    window::Window,
};
use gl;
use std::{rc::Rc, cell::RefCell};
use crate::engine::camera::Camera;
use crate::engine::object3d::{GLMesh, Object3D};

/// `Renderer` encapsulates the OpenGL rendering context,
/// window creation, event handling loop, and basic rendering operations.
///
/// This struct leverages the `glutin` crate to manage the platform-specific
/// OpenGL context creation and windowing. It provides a clean API for creating
/// a window with an OpenGL context, setting a clear color, resizing the window,
/// and running the event loop to handle window events and redraw requests.
///
/// # Design Notes
///
/// - Uses `glutin::EventLoop` to drive the event loop and process window events.
/// - Uses `glutin::ContextWrapper<PossiblyCurrent, Window>` to manage the
///   OpenGL context lifecycle and tie it to the window.
/// - OpenGL functions are loaded dynamically using the `gl` crate's loader mechanism.
/// - The `run` method drives the main event loop, processing events such as window close,
///   redraw, and requesting redraws efficiently.
///
/// # Threading & Ownership
///
/// The struct owns the event loop and windowed context, and moves ownership into
/// the `run` method. `Rc<RefCell<_>>` is used internally to enable multiple closures
/// to access and mutate the context inside the event loop.
///
/// # Example Usage
///
/// ```no_run
/// let mut renderer = Renderer::new("Example", 800, 600);
/// renderer.set_clear_color(0.0, 0.0, 0.0, 1.0);
/// renderer.run();
/// ```
pub struct Renderer {
    /// The event loop responsible for driving window events and rendering
    event_loop: EventLoop<()>,

    /// The OpenGL context tied to a window, currently in the `PossiblyCurrent` state,
    /// meaning OpenGL commands can be issued.
    windowed_context: ContextWrapper<PossiblyCurrent, Window>,

    /// The color used to clear the OpenGL framebuffer each frame, stored as RGBA floats.
    clear_color: [f32; 4],

    /// what camera are we rendering from?
    camera: Option<Camera>,

    /// What scene are we rendering?
    scene: Option<Object3D>
}

impl Renderer {
    /// Creates a new `Renderer` instance with the specified window title, width, and height.
    ///
    /// # Parameters
    /// - `title`: The window title shown in the title bar.
    /// - `width`: The initial width of the window in physical pixels.
    /// - `height`: The initial height of the window in physical pixels.
    ///
    /// # Panics
    /// Panics if window or OpenGL context creation fails.
    ///
    /// # Detailed Explanation
    /// 1. Initializes the event loop needed for window events and input.
    /// 2. Configures a window builder with title and size.
    /// 3. Creates an OpenGL context tied to this window with vsync enabled to avoid tearing.
    /// 4. Makes the OpenGL context current on the thread to allow GL calls.
    /// 5. Loads all OpenGL function pointers dynamically via the context.
    /// 6. Sets a default clear color (dark blueish).
    ///
    /// This setup ensures that the OpenGL context is properly initialized and ready
    /// for rendering commands.
    pub fn new(title: &str, width: u32, height: u32) -> Self {
        // Create the event loop instance for handling window and input events
        let event_loop = EventLoop::new();

        // Build a window with specified title and inner size (content area size)
        let wb = WindowBuilder::new()
            .with_title(title)
            .with_inner_size(PhysicalSize::new(width, height));

        // Create a windowed OpenGL context with vsync enabled to sync buffer swaps to display refresh
        let windowed_context = ContextBuilder::new()
            .with_vsync(true)
            .build_windowed(wb, &event_loop)
            .unwrap();

        // Make the OpenGL context current on this thread; required before issuing GL calls
        let windowed_context = unsafe { windowed_context.make_current().unwrap() };

        // Load all OpenGL function pointers using the context's proc address loader
        gl::load_with(|symbol| windowed_context.get_proc_address(symbol) as *const _);

        // Set the default clear color to a pleasant dark blue shade
        let clear_color = [0.1, 0.2, 0.3, 1.0];
        unsafe {
            gl::ClearColor(clear_color[0], clear_color[1], clear_color[2], clear_color[3]);
        }

        Self {
            event_loop,
            windowed_context,
            clear_color,
            camera: None,
            scene: None,
        }
    }

    /// Clears the current OpenGL framebuffer using the stored clear color.
    ///
    /// # Safety
    /// This function calls the unsafe OpenGL `glClear` command, which
    /// relies on a valid current OpenGL context.
    ///
    /// # Usage
    /// Call before rendering a new frame to reset the framebuffer.
    pub fn clear(&self) {
        unsafe {
            gl::Clear(gl::COLOR_BUFFER_BIT);
        }
    }

    /// Swaps the front and back buffers, presenting the rendered frame to the window.
    ///
    /// # Panics
    /// Panics if buffer swap fails, which can indicate a context or window error.
    pub fn swap_buffers(&self) {
        self.windowed_context.swap_buffers().unwrap();
    }

    /// Updates the OpenGL clear color to the specified RGBA value and stores it internally.
    ///
    /// # Parameters
    /// - `r`, `g`, `b`, `a`: Color components as floating-point values between 0.0 and 1.0.
    ///
    /// This will affect the color used in subsequent `clear` calls.
    pub fn set_clear_color(&mut self, r: f32, g: f32, b: f32, a: f32) {
        self.clear_color = [r, g, b, a];
        unsafe {
            gl::ClearColor(r, g, b, a);
        }
    }

    /// Resizes the window to the specified width and height in physical pixels.
    ///
    /// # Parameters
    /// - `width`: New window width in physical pixels.
    /// - `height`: New window height in physical pixels.
    ///
    /// This method requests the underlying window to resize,
    /// which will typically trigger a redraw event.
    pub fn set_size(&self, width: u32, height: u32) {
        self.windowed_context.window().set_inner_size(PhysicalSize::new(width, height));
    }

    /// Starts the renderer's event loop, handling window events and redraw requests.
    ///
    /// This method **never returns** until the window is closed by the user or the event loop exits.
    /// It processes:
    /// - `WindowEvent::CloseRequested`: Exits the application.
    /// - `Event::RedrawRequested`: Clears the framebuffer and swaps buffers to present the frame.
    ///
    /// It also ensures the window continuously requests redraws,
    /// driving a rendering loop at the native vsync rate.
    ///
    /// # Detailed Design Notes
    /// - Wraps the `windowed_context` in `Rc<RefCell<_>>` to allow mutable access
    ///   inside the closure passed to the event loop.
    /// - Sets the control flow to `ControlFlow::Wait` to efficiently sleep until new events.
    /// - On each redraw event, clears and swaps buffers to update the screen.
    /// - Requests redraw on every iteration to keep the rendering loop alive.
    pub fn run(self) {
        let Renderer {
            event_loop,
            windowed_context,
            clear_color: _,
            camera,
            scene,
        } = self;

        let context = Rc::new(RefCell::new(windowed_context));
        let camera = Rc::new(RefCell::new(camera));
        let scene = Rc::new(RefCell::new(scene));

        event_loop.run(move |event, _, control_flow| {
            *control_flow = ControlFlow::Wait;

            match event {
                Event::WindowEvent { event, .. } => match event {
                    WindowEvent::CloseRequested => *control_flow = ControlFlow::Exit,
                    _ => {}
                },

                Event::RedrawRequested(_) => {
                    unsafe {
                        gl::Clear(gl::COLOR_BUFFER_BIT);
                    }

                    let cam_ref = camera.borrow();
                    let mut scene_ref = scene.borrow_mut();

                    if let (Some(cam), Some(scene)) = (&*cam_ref, &mut *scene_ref) {
                        scene.draw(cam);
                    }

                    context.borrow().swap_buffers().unwrap();
                }

                _ => {}
            }

            // Continuously redraw at vsync rate
            context.borrow().window().request_redraw();
        });
    }

}
