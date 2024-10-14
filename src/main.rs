mod body;
mod camera;
mod gpu_slicer;
mod mesh;
mod mesh_renderer;
mod cpu_slicer;
mod stl_processor;
mod texture;
mod slicer;
use body::Body;
use glow::HasContext;
use image::{ImageBuffer, Luma};
use log::debug;
use mesh::Vertex;
use mesh_renderer::MeshRenderer;
use nalgebra::Vector3;
use rfd::AsyncFileDialog;
use slint::platform::PointerEventButton;
use std::cell::RefCell;
use std::num::NonZeroU32;
use std::rc::Rc;
use stl_processor::StlProcessor;
use cpu_slicer::CPUSlicer;
use gpu_slicer::GPUSlicer;
use glow::Context as GlowContext;

slint::include_modules!();
use tokio::task;
macro_rules! define_scoped_binding {
    (struct $binding_ty_name:ident => $obj_name:path, $param_name:path, $binding_fn:ident, $target_name:path) => {
        struct $binding_ty_name {
            saved_value: Option<$obj_name>,
            gl: Rc<GlowContext>,
        }

        impl $binding_ty_name {
            unsafe fn new(gl: &Rc<GlowContext>, new_binding: Option<$obj_name>) -> Self {
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
            gl: Rc<GlowContext>,
        }

        impl $binding_ty_name {
            unsafe fn new(gl: &Rc<GlowContext>, new_binding: Option<$obj_name>) -> Self {
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

// define_scoped_binding!(struct ScopedTextureBinding => glow::NativeTexture, glow::TEXTURE_BINDING_2D, bind_texture, glow::TEXTURE_2D);
define_scoped_binding!(struct ScopedFrameBufferBinding => glow::NativeFramebuffer, glow::DRAW_FRAMEBUFFER_BINDING, bind_framebuffer, glow::DRAW_FRAMEBUFFER);
define_scoped_binding!(struct ScopedVBOBinding => glow::NativeBuffer, glow::ARRAY_BUFFER_BINDING, bind_buffer, glow::ARRAY_BUFFER);
define_scoped_binding!(struct ScopedVAOBinding => glow::NativeVertexArray, glow::VERTEX_ARRAY_BINDING, bind_vertex_array);
#[derive(Default)]
struct MouseState {
    x: f32,
    y: f32,
    p_x: f32,
    p_y: f32,
    left_pressed: bool,
    middle_pressed: bool,
    right_pressed: bool,
    other_pressed: bool,
    back_pressed: bool,
    forward_pressed: bool,
}

type SharedBodies = Rc<RefCell<Vec<Rc<RefCell<Body>>>>>;
type SharedMeshRenderer = Rc<RefCell<Option<MeshRenderer>>>;
type SharedMouseState = Rc<RefCell<MouseState>>;
type SharedCPUSlicer = Rc<RefCell<CPUSlicer>>;
type SharedGPUSlicer = Rc<RefCell<Option<GPUSlicer>>>;
// type SharedGlContext = Rc<RefCell<Option<GlowContext>>>;

struct AppState {
    mouse_state: SharedMouseState,
    shared_mesh_renderer: SharedMeshRenderer,
    shared_bodies: SharedBodies,
    shared_cpu_slicer: SharedCPUSlicer,
    shared_gpu_slicer: SharedGPUSlicer,
    // let_shared_gl_context: SharedGlContext
}

fn main() {
    // Initialize the Slint application
    let app = App::new().unwrap();
    let app_weak = app.as_weak();
    let printer_width = 1000;
    let printer_length = 500;


    let state = AppState {
        mouse_state: Rc::new(RefCell::new(MouseState::default())),
        shared_mesh_renderer: Rc::new(RefCell::new(None)),
        shared_bodies: Rc::new(RefCell::new(Vec::<Rc<RefCell<Body>>>::new())), // Initialized as empty Vec
        shared_cpu_slicer: Rc::new(RefCell::new(CPUSlicer::default())),
        shared_gpu_slicer: Rc::new(RefCell::new(None))
    };

    // let size = app.window().size();
    let interal_render_width = 1920;
    let internal_render_height = 1080;
    {
        // Set the rendering notifier with a closure
        // Create a weak reference to the app for use inside the closure
        let app_weak_clone = app_weak.clone(); // Clone app_weak for use inside the closure
        let mesh_renderer_clone = Rc::clone(&state.shared_mesh_renderer);
        let bodies_clone = Rc::clone(&state.shared_bodies);
        if let Err(error) = app.window().set_rendering_notifier({
            // Move clones into the closure

            move |state, graphics_api| {
                match state {
                    slint::RenderingState::RenderingSetup => {
                        // Initialize OpenGL context
                        let gl: GlowContext = match graphics_api {
                            slint::GraphicsAPI::NativeOpenGL { get_proc_address } => unsafe {
                                GlowContext::from_loader_function_cstr(|s| get_proc_address(s))
                            },
                            _ => panic!("Unsupported Graphics API"),
                        };
                        // Assume 'gl' is your GlowContext instance

                        // Get OpenGL version string
                        let version = unsafe { gl.get_parameter_string(glow::VERSION) };
                        println!("OpenGL Version: {}", version);

                        // Get GLSL version string
                        let shading_language_version =
                            unsafe { gl.get_parameter_string(glow::SHADING_LANGUAGE_VERSION) };
                        println!("GLSL Version: {}", shading_language_version);

                        // Get OpenGL major and minor version numbers
                        let major_version = unsafe { gl.get_parameter_i32(glow::MAJOR_VERSION) };
                        let minor_version = unsafe { gl.get_parameter_i32(glow::MINOR_VERSION) };
                        println!(
                            "OpenGL Major Version: {}. OpenGL Minor Version: {}",
                            major_version, minor_version
                        );
                        // Because the renderer needs access to the OpenGL context, we need to initialize
                        // it here instead of earlier in the state initialization
                        let renderer =
                            MeshRenderer::new(gl, interal_render_width, internal_render_height);
                        // Store the renderer in the shared Rc<RefCell<_>>
                        *mesh_renderer_clone.borrow_mut() = Some(renderer);

                        // The gpu slicer of course also needs the OpenGL context
                        // let gpu_slicer = GPUSlicer::new(&gl, printer_width, printer_length);
                    }
                    slint::RenderingState::BeforeRendering => {
                        // Access the renderer
                        if let Some(renderer) = mesh_renderer_clone.borrow_mut().as_mut() {
                            // Get actual window size
                            if let Some(app) = app_weak_clone.upgrade() {
                                // Render and get the texture
                                let texture = renderer.render(
                                    interal_render_width as u32,
                                    internal_render_height as u32,
                                );

                                let mut bodies_ui_vec: Vec<BodyUI> = Vec::new();
                                let mut num_bodies = 0;
                                for body in bodies_clone.borrow_mut().iter() {
                                    num_bodies += 1;
                                    let b = body.borrow_mut();
                                    // println!("{:.3},{:.3},{:.3}",b.position.x,b.position.y,b.position.z);
                                    // println!("{:.3},{:.3},{:.3}, {:.3}",b.rotation.i, b.rotation.j, b.rotation.k, b.rotation.w);
                                    // println!("{:.3},{:.3},{:.3}",b.scale.x,b.scale.y,b.scale.z);
                                    bodies_ui_vec.push(BodyUI {
                                        enabled: b.enabled.clone(),
                                        name: b.name.clone().into(),
                                        uuid: b.uuid.clone().to_string().into(),
                                        visible: b.visible.clone(),
                                        selected: b.selected.clone(),
                                        p_x: b.position.x.to_string().clone().into(),
                                        p_y: b.position.y.to_string().clone().into(),
                                        p_z: b.position.z.to_string().clone().into(),
                                        r_x: b.rotation.i.to_string().clone().into(),
                                        r_y: b.rotation.j.to_string().clone().into(),
                                        r_z: b.rotation.k.to_string().clone().into(),
                                        s_x: b.scale.x.to_string().clone().into(),
                                        s_y: b.scale.y.to_string().clone().into(),
                                        s_z: b.scale.z.to_string().clone().into(),
                                    })
                                }

                                let bodies_model: Rc<slint::VecModel<BodyUI>> =
                                    std::rc::Rc::new(slint::VecModel::from(bodies_ui_vec));
                                // Update the app's texture
                                app.set_texture(slint::Image::from(texture));
                                app.set_bodies(bodies_model.into());

                                app.set_num_bodies(num_bodies);
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
    }

    // Handler for scrollwheel zooming TODO: Consider renaming for clarity
    {
        let app_weak_clone = app_weak.clone(); // Clone app_weak again for this closure
        let mesh_renderer_clone = Rc::clone(&state.shared_mesh_renderer); // Clone mesh_renderer for this closure
        app.on_zoom(move |amt| {
            // Access the renderer
            if let Some(renderer) = mesh_renderer_clone.borrow_mut().as_mut() {
                // Move the camera
                renderer.zoom(amt);

                // Trigger a redraw
                if let Some(app) = app_weak_clone.upgrade() {
                    app.window().request_redraw();
                }
            }
        });
    }

    // Handler for mouse movement in renderer
    {
        let app_weak_clone = app_weak.clone(); // Clone app_weak again for this closure
        let mesh_renderer_clone = Rc::clone(&state.shared_mesh_renderer); // Clone mesh_renderer for this closure
        let mouse_state_clone = Rc::clone(&state.mouse_state);
        app.on_mouse_move_renderer(move |x, y| {
            debug!("On mouse move event received");

            let mut mouse_state = mouse_state_clone.borrow_mut();

            // If the previous coords are still 0,0 then let's not move a bunch and return 0
            let delta_x = x - if mouse_state.p_x != 0.0 {
                mouse_state.p_x
            } else {
                x
            };
            let delta_y = y - if mouse_state.p_y != 0.0 {
                mouse_state.p_y
            } else {
                y
            };
            mouse_state.p_x = x;
            mouse_state.p_y = y;
            mouse_state.x = x;
            mouse_state.y = y;
            debug!("Delta x: {:.3}, Delta y: {:.3}", delta_x, delta_y);
            debug!("Mouse pressed? {}", mouse_state.left_pressed);

            // Access the renderer
            if let Some(renderer) = mesh_renderer_clone.borrow_mut().as_mut() {
                if mouse_state.left_pressed {
                    renderer.camera_pitch_yaw(delta_x, delta_y);
                }
                if mouse_state.middle_pressed {
                    renderer.camera_pan(delta_x, delta_y);
                }
                // Trigger a redraw
                if let Some(app) = app_weak_clone.upgrade() {
                    app.window().request_redraw();
                }
            }
        });
    }

    // Mouse down handler for renderer
    {
        let mouse_state_clone = Rc::clone(&state.mouse_state);
        app.on_mouse_down_renderer(move |button| {
            debug!("On mouse down received");
            let mut mouse_state = mouse_state_clone.borrow_mut();
            match button {
                PointerEventButton::Left => mouse_state.left_pressed = true,
                PointerEventButton::Other => mouse_state.other_pressed = true,
                PointerEventButton::Right => mouse_state.right_pressed = true,
                PointerEventButton::Middle => mouse_state.middle_pressed = true,
                PointerEventButton::Back => mouse_state.back_pressed = true,
                PointerEventButton::Forward => mouse_state.forward_pressed = true,
                _ => {}
            }
        });
    }
    // Mouse up handler for renderer
    {
        let mouse_state_clone = Rc::clone(&state.mouse_state);
        app.on_mouse_up_renderer(move |button| {
            debug!("On mouse up received");
            let mut mouse_state = mouse_state_clone.borrow_mut();
            match button {
                PointerEventButton::Left => mouse_state.left_pressed = false,
                PointerEventButton::Other => mouse_state.other_pressed = false,
                PointerEventButton::Right => mouse_state.right_pressed = false,
                PointerEventButton::Middle => mouse_state.middle_pressed = false,
                PointerEventButton::Back => mouse_state.back_pressed = false,
                PointerEventButton::Forward => mouse_state.forward_pressed = false,
                _ => {}
            }
        });
    }

    async fn open_files_from_dialog(
        mesh_renderer_clone: &Rc<RefCell<Option<MeshRenderer>>>,
        bodies_clone: &Rc<RefCell<Vec<Rc<RefCell<Body>>>>>,
    ) {
        let paths = AsyncFileDialog::new()
            .add_filter("stl", &["stl", "STL"])
            .set_directory("~")
            .pick_files()
            .await
            .unwrap();

        let stl_processor = StlProcessor::new();
        let mut bodies_vec: Vec<Rc<RefCell<Body>>> = Vec::new();

        for path in paths {
            let body = Rc::new(RefCell::new(Body::new_from_stl(
                path.path().as_os_str(),
                &stl_processor,
            )));
            bodies_vec.push(Rc::clone(&body));
        }
        bodies_vec.iter_mut().for_each(|body| {
            if let Some(renderer) = mesh_renderer_clone.borrow_mut().as_mut() {
                renderer.add_body(Rc::clone(&body));
            }
        });
        bodies_clone.borrow_mut().append(&mut bodies_vec);
    }

    // Handler for opening STL importer file picker
    {
        let mesh_renderer_clone = Rc::clone(&state.shared_mesh_renderer);
        let bodies_clone = Rc::clone(&state.shared_bodies);
        app.on_click_import_stl(move || {
            let mrc_clone = Rc::clone(&mesh_renderer_clone);
            let bc_clone = Rc::clone(&bodies_clone);
            let slint_future = async move {
                open_files_from_dialog(&mrc_clone, &bc_clone).await;
            };
            slint::spawn_local(async_compat::Compat::new(slint_future)).unwrap();
        });
    }

    // Handlers for objectlistitem editing
    {
        let bodies_clone = Rc::clone(&state.shared_bodies);
        app.on_body_position_edited_single_axis(
            move |uuid: slint::SharedString, amt: f32, axis: i32| {
                let bodies = bodies_clone.borrow_mut();
                for body_rc in bodies.iter() {
                    let mut body = body_rc.borrow_mut();
                    if body.uuid.to_string() == uuid.to_string() {
                        let v = match axis {
                            0 => Vector3::new(amt, body.position.y, body.position.z),
                            1 => Vector3::new(body.position.x, amt, body.position.z),
                            2 => Vector3::new(body.position.x, body.position.y, amt),
                            _ => Vector3::default(),
                        };
                        body.set_position(v);
                    }
                }
            },
        );

        let bodies_clone = Rc::clone(&state.shared_bodies);
        app.on_body_rotation_edited_single_axis(
            move |uuid: slint::SharedString, amt: f32, axis: i32| {
                let bodies = bodies_clone.borrow_mut();
                for body_rc in bodies.iter() {
                    let mut body = body_rc.borrow_mut();
                    if body.uuid.to_string() == uuid.to_string() {
                        let rotation = Body::quaternion_to_euler(&body.rotation);
                        let v = match axis {
                            0 => Vector3::new(amt, rotation.y, rotation.z),
                            1 => Vector3::new(rotation.x, amt, rotation.z),
                            2 => Vector3::new(rotation.x, rotation.y, amt),
                            _ => Vector3::default(),
                        };
                        body.set_rotation(v);
                    }
                }
            },
        );

        let bodies_clone = Rc::clone(&state.shared_bodies);
        app.on_body_scale_edited_single_axis(
            move |uuid: slint::SharedString, amt: f32, axis: i32| {
                let bodies = bodies_clone.borrow_mut();
                for body_rc in bodies.iter() {
                    let mut body = body_rc.borrow_mut();
                    if body.uuid.to_string() == uuid.to_string() {
                        let v = match axis {
                            0 => Vector3::new(amt, body.scale.y, body.scale.z),
                            1 => Vector3::new(body.scale.x, amt, body.scale.z),
                            2 => Vector3::new(body.scale.x, body.scale.y, amt),
                            _ => Vector3::default(),
                        };
                        body.set_scale(v);
                    }
                }
            },
        );

        let bodies_clone = Rc::clone(&state.shared_bodies);
        app.on_toggle_body_selected(move |uuid| {
            println!("trying to toggle body {}", uuid.to_string());
            let bodies = bodies_clone.borrow_mut();
            for body_rc in bodies.iter() {
                let mut body = body_rc.borrow_mut();
                println!("Body: {}", body.uuid);
                println!("UUID trying to match: {}", uuid.to_string());
                if body.uuid.to_string() == uuid.to_string() {
                    println!("Match: {}", body.uuid);
                    body.selected = !body.selected;
                }
            }
        });
    }

    // async fn slice_bodies(bodies_clone:Rc<RefCell<Vec<Rc<RefCell<Body>>>>>, slice_increment:f32, image_width: u32, image_height: u32,
    //      gpu_slicer: Option<GPUSlicer>, cpu_slicer: CPUSlicer) -> Vec<ImageBuffer<Luma<u8>, Vec<u8>>> {
    //     let vertices_sets = bodies_clone.borrow_mut().iter().map(|f| f.borrow_mut().mesh.vertices);
    //     let triangles: Vec<Triangle> = Vec::new();
    //     for vertices in vertices_sets {

    //     }
        
    //     if let Some(gpu_slicer)  = gpu_slicer {
    //         gpu_slicer.generate_slice_images(
    //             vertices,
    //             slice_increment,
    //             image_width,
    //             image_height,
    //         ).unwrap()
    //     } else {

    //     }
    // }

    // Slicing button callbacks
    {
        let bodies_clone: Rc<RefCell<Vec<Rc<RefCell<Body>>>>> = Rc::clone(&state.shared_bodies);
        let triangles: Vec<Vertex> = Vec::new();
        // let slicer_clone: Rc<RefCell<Slicer>> = 
        app.on_slice_selected(move || for body in bodies_clone.borrow_mut().iter() {
            // body.borrow_mut().mesh.vertices
        });

        let bodies_clone = Rc::clone(&state.shared_bodies);
        app.on_slice_all(|| {});
    }

    // Run the Slint application
    app.run().unwrap();
}
