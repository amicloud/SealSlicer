// Copyright © SixtyFPS GmbH <info@slint.dev>
// SPDX-License-Identifier: MIT
mod mesh_data;
mod stl_processor;
mod camera;
mod texture;
mod mesh_renderer;
use camera::CameraMove;
use mesh_data::MeshData;
use mesh_renderer::MeshRenderer;
use nalgebra::{Matrix4, Point3, Vector3};
use slint::SharedString;
use std::num::NonZeroU32;
use std::rc::Rc;
slint::include_modules!();
use glow::HasContext;
use std::cell::RefCell;

macro_rules! define_scoped_binding {
    (struct $binding_ty_name:ident => $obj_name:path, $param_name:path, $binding_fn:ident, $target_name:path) => {
        struct $binding_ty_name {
            saved_value: Option<$obj_name>,
            gl: Rc<glow::Context>,
        }

        impl $binding_ty_name {
            unsafe fn new(gl: &Rc<glow::Context>, new_binding: Option<$obj_name>) -> Self {
                let saved_value =
                    NonZeroU32::new(gl.get_parameter_i32($param_name) as u32).map($obj_name);

                gl.$binding_fn($target_name, new_binding);
                Self {
                    saved_value,
                    gl: gl.clone(),
                }
            }
        }

        impl Drop for $binding_ty_name {
            fn drop(&mut self) {
                unsafe {
                    self.gl.$binding_fn($target_name, self.saved_value);
                }
            }
        }
    };
    (struct $binding_ty_name:ident => $obj_name:path, $param_name:path, $binding_fn:ident) => {
        struct $binding_ty_name {
            saved_value: Option<$obj_name>,
            gl: Rc<glow::Context>,
        }

        impl $binding_ty_name {
            unsafe fn new(gl: &Rc<glow::Context>, new_binding: Option<$obj_name>) -> Self {
                let saved_value =
                    NonZeroU32::new(gl.get_parameter_i32($param_name) as u32).map($obj_name);

                gl.$binding_fn(new_binding);
                Self {
                    saved_value,
                    gl: gl.clone(),
                }
            }
        }

        impl Drop for $binding_ty_name {
            fn drop(&mut self) {
                unsafe {
                    self.gl.$binding_fn(self.saved_value);
                }
            }
        }
    };
}

define_scoped_binding!(struct ScopedTextureBinding => glow::NativeTexture, glow::TEXTURE_BINDING_2D, bind_texture, glow::TEXTURE_2D);
define_scoped_binding!(struct ScopedFrameBufferBinding => glow::NativeFramebuffer, glow::DRAW_FRAMEBUFFER_BINDING, bind_framebuffer, glow::DRAW_FRAMEBUFFER);
define_scoped_binding!(struct ScopedVBOBinding => glow::NativeBuffer, glow::ARRAY_BUFFER_BINDING, bind_buffer, glow::ARRAY_BUFFER);
define_scoped_binding!(struct ScopedVAOBinding => glow::NativeVertexArray, glow::VERTEX_ARRAY_BINDING, bind_vertex_array);
// Camera state struct



fn main() {
    // Initialize the Slint application
    let app = App::new().unwrap();

    // Create a shared, mutable reference to MeshRenderer
    let mesh_renderer = Rc::new(RefCell::new(None));

    // Create a weak reference to the app for use inside the closure
    let app_weak = app.as_weak();

    // Set the rendering notifier with a closure
    if let Err(error) = app.window().set_rendering_notifier({
        // Move clones into the closure
        let mesh_renderer_clone = Rc::clone(&mesh_renderer);
        let app_weak_clone = app_weak.clone(); // Clone app_weak for use inside the closure
        move |state, graphics_api| {
            match state {
                slint::RenderingState::RenderingSetup => {
                    // Initialize OpenGL context
                    let context = match graphics_api {
                        slint::GraphicsAPI::NativeOpenGL { get_proc_address } => unsafe {
                            glow::Context::from_loader_function_cstr(|s| get_proc_address(s))
                        },
                        _ => {
                            eprintln!("Unsupported Graphics API");
                            return;
                        }
                    };

                    // Initialize the MeshRenderer
                    let mut renderer = MeshRenderer::new(context);

                    // Import the example STL and add to renderer
                    let example_stl = "ogre.stl"; // Ensure this file exists
                    let mut example_data = MeshData::default();
                    example_data.import_stl(example_stl);
                    renderer.add_mesh(example_data);

                    // Store the renderer in the shared Rc<RefCell<_>>
                    *mesh_renderer_clone.borrow_mut() = Some(renderer);
                }
                slint::RenderingState::BeforeRendering => {
                    // Access the renderer
                    if let Some(renderer) = mesh_renderer_clone.borrow_mut().as_mut() {
                        // Get actual window size
                        if let Some(app) = app_weak_clone.upgrade() {
                            let size = app.window().size();
                            let width = size.width;
                            let height = size.height;

                            // Render and get the texture
                            let texture = renderer.render(width as u32, height as u32);

                            // Update the app's texture
                            app.set_texture(slint::Image::from(texture));
                            app.window().request_redraw();
                        }
                    }
                }
                slint::RenderingState::AfterRendering => {
                    // Optional: Perform any post-rendering tasks
                }
                slint::RenderingState::RenderingTeardown => {
                    // Clean up the renderer
                    *mesh_renderer_clone.borrow_mut() = None;
                }
                _ => {}
            }
        }
    }) {
        match error {
            slint::SetRenderingNotifierError::Unsupported => eprintln!(
                "This example requires the use of the GL backend. Please run with the environment variable SLINT_BACKEND=GL set."
            ),
            _ => unreachable!(),
        }
        std::process::exit(1);
    }

    // Set up the adjust_camera callback
    {
        let app_weak_clone = app_weak.clone(); // Clone app_weak again for this closure
        let mesh_renderer_clone = Rc::clone(&mesh_renderer); // Clone mesh_renderer for this closure

        app.on_adjust_camera(move |direction_string| {
            // Convert direction_string to CameraMove
            let camera_move = match direction_string.as_str() {
                "up" => camera::CameraMove::Up,
                "down" => camera::CameraMove::Down,
                "left" => camera::CameraMove::Left,
                "right" => camera::CameraMove::Right,
                "zoom_in" => camera::CameraMove::ZoomIn,
                "zoom_out" => camera::CameraMove::ZoomOut,
                _ => return,
            };

            // Access the renderer
            if let Some(renderer) = mesh_renderer_clone.borrow_mut().as_mut() {
                // Move the camera
                renderer.move_camera(camera_move);

                // Trigger a redraw
                if let Some(app) = app_weak_clone.upgrade() {
                    app.window().request_redraw();
                }
            }
        });
    }

    // Run the Slint application
    app.run().unwrap();
}