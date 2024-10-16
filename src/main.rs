
// Distributed under the GNU Affero General Public License v3.0 or later.
// See accompanying file LICENSE or https://www.gnu.org/licenses/agpl-3.0.html for details.

mod body;
mod camera;
mod cpu_slicer;
mod gpu_slicer;
mod mesh;
mod mesh_renderer;
mod stl_processor;
mod texture;
use body::Body;
use cpu_slicer::CPUSlicer;
use glow::Context as GlowContext;
use glow::HasContext;
use gpu_slicer::GPUSlicer;
use image::EncodableLayout;
use image::Rgb;
use image::{ImageBuffer, Luma};
use log::debug;
use mesh_renderer::MeshRenderer;
use nalgebra::Vector3;
use rayon::iter::IntoParallelRefIterator;
use rayon::prelude::*;
use rfd::AsyncFileDialog;
use slint::platform::PointerEventButton;
use slint::SharedString;
use std::cell::RefCell;
use std::fs;
use std::num::NonZeroU32;
use std::rc::Rc;
use std::time::SystemTime;
use std::time::UNIX_EPOCH;
use stl_io::Triangle;
use stl_processor::StlProcessor;
use webp::Encoder as WebpEncoder;
slint::include_modules!();
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
    let printer_x = 1920;
    let printer_y = 1080;

    let state = AppState {
        mouse_state: Rc::new(RefCell::new(MouseState::default())),
        shared_mesh_renderer: Rc::new(RefCell::new(None)),
        shared_bodies: Rc::new(RefCell::new(Vec::<Rc<RefCell<Body>>>::new())), // Initialized as empty Vec
        shared_cpu_slicer: Rc::new(RefCell::new(CPUSlicer::default())),
        shared_gpu_slicer: Rc::new(RefCell::new(None)),
    };

    // let size = app.window().size();
    let internal_render_width = 1920;
    let internal_render_height = 1080;
    {
        // Set the rendering notifier with a closure
        // Create a weak reference to the app for use inside the closure
        let app_weak_clone = app_weak.clone(); // Clone app_weak for use inside the closure
        let mesh_renderer_clone = Rc::clone(&state.shared_mesh_renderer);
        let bodies_clone = Rc::clone(&state.shared_bodies);
        let gpu_slicer_clone = Rc::clone(&state.shared_gpu_slicer);
        let cpu_slicer_clone = Rc::clone(&state.shared_cpu_slicer);
        if let Err(error) = app.window().set_rendering_notifier({
            // Move clones into the closure

            move |rendering_state, graphics_api| {
                match rendering_state {
                    slint::RenderingState::RenderingSetup => {
                        // Initialize OpenGL context
                        let gl: GlowContext = match graphics_api {
                            slint::GraphicsAPI::NativeOpenGL { get_proc_address } => unsafe {
                                GlowContext::from_loader_function_cstr(|s| get_proc_address(s))
                            },
                            _ => panic!("Unsupported Graphics API"),
                        };
                        let gl = Rc::new(gl); // Wrap in Rc

                        // Use 'gl' to get OpenGL version strings etc.
                        let version = unsafe { gl.get_parameter_string(glow::VERSION) };
                        println!("OpenGL Version: {}", version);

                        let shading_language_version =
                            unsafe { gl.get_parameter_string(glow::SHADING_LANGUAGE_VERSION) };
                        println!("GLSL Version: {}", shading_language_version);

                        let major_version = unsafe { gl.get_parameter_i32(glow::MAJOR_VERSION) };
                        let minor_version = unsafe { gl.get_parameter_i32(glow::MINOR_VERSION) };
                        println!(
                            "OpenGL Major Version: {}. OpenGL Minor Version: {}",
                            major_version, minor_version
                        );

                        // Initialize renderer and slicers with cloned Rc
                        let renderer = MeshRenderer::new(
                            gl.clone(),
                            internal_render_width,
                            internal_render_height,
                        );
                        *mesh_renderer_clone.borrow_mut() = Some(renderer);
                        let slice_thickness = 0.050; // 50 Microns. I think I want to change this to an i32 of microns
                        let gpu_slicer =
                            GPUSlicer::new(gl.clone(), printer_x, printer_y, slice_thickness);
                        // *gpu_slicer_clone.borrow_mut() = Some(gpu_slicer);
                        *gpu_slicer_clone.borrow_mut() = None; // Disabling the gpu slicer for now

                        let cpu_slicer = CPUSlicer::new(printer_x, printer_y, slice_thickness);
                        *cpu_slicer_clone.borrow_mut() = cpu_slicer;
                    }
                    slint::RenderingState::BeforeRendering => {
                        // Access the renderer
                        if let Some(renderer) = mesh_renderer_clone.borrow_mut().as_mut() {
                            // Get actual window size
                            if let Some(app) = app_weak_clone.upgrade() {
                                // Render and get the texture
                                let texture = renderer.render(
                                    internal_render_width as u32,
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
            println!("Loaded body: {}", path.file_name());
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
                let bodies = bodies_clone.borrow();
                for body_rc in bodies.iter() {
                    let mut body = body_rc.borrow_mut();
                    if body.eq_uuid_ss(&uuid) {
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
                let bodies = bodies_clone.borrow();
                for body_rc in bodies.iter() {
                    let mut body = body_rc.borrow_mut();
                    if body.eq_uuid_ss(&uuid) {
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
                let bodies = bodies_clone.borrow();
                for body_rc in bodies.iter() {
                    let mut body = body_rc.borrow_mut();
                    if body.eq_uuid_ss(&uuid) {
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
            let bodies = bodies_clone.borrow();
            for body_rc in bodies.iter() {
                let mut body = body_rc.borrow_mut();
                if body.eq_uuid_ss(&uuid) {
                    body.selected = !body.selected;
                }
            }
        });
    }

    async fn slice_all_bodies(
        bodies_clone: Rc<RefCell<Vec<Rc<RefCell<Body>>>>>,
        gpu_slicer_clone: Rc<RefCell<Option<GPUSlicer>>>,
        cpu_slicer_clone: Rc<RefCell<CPUSlicer>>,
    ) -> Vec<ImageBuffer<Luma<u8>, Vec<u8>>> {
        // Clone the Rc<RefCell<Body>>s into a new vector to avoid borrowing issues
        let bodies_vec = {
            let bodies_ref = bodies_clone.borrow();
            bodies_ref.as_slice().to_vec()
        };
        let output: Vec<ImageBuffer<Luma<u8>, Vec<u8>>>;
        if let Some(gpu_slicer) = gpu_slicer_clone.borrow_mut().as_mut() {
            output = gpu_slicer.slice_bodies(bodies_vec).unwrap()
        } else {
            output = cpu_slicer_clone
                .borrow_mut()
                .slice_bodies(bodies_vec)
                .unwrap()
        }
        // For now let's try just writing the data to a series of images in the test slices dir inside of a new dir with a current unix timestamp as the name
        // insert folder and file writing code here.
        let start = SystemTime::now();
        let since_the_epoch = start
            .duration_since(UNIX_EPOCH)
            .expect("Time went backwards");
        let timestamp = since_the_epoch.as_secs();

        // Create a new directory inside "test slices" with the timestamp as its name
        let dir_path = format!("slices/{}", timestamp);
        fs::create_dir_all(&dir_path).expect("Failed to create directory");

        // Iterate over the output images and save each one to a file in lossless WebP format
        output.par_iter().enumerate().for_each(|(i, image)| {
            let file_path = format!("{}/slice_{:04}.webp", dir_path, i);

            // Convert ImageBuffer<Luma<u8>, Vec<u8>> to ImageBuffer<Rgb<u8>, Vec<u8>>
            let rgb_image: ImageBuffer<Rgb<u8>, Vec<u8>> = convert_luma_to_rgb(image);

            // Retrieve width and height before moving rgb_image
            let width = rgb_image.width();
            let height = rgb_image.height();

            // Flatten the RGB image into a Vec<u8>
            let rgb_data = rgb_image.into_raw();

            // Create a WebP encoder with lossless encoding
            let encoder = WebpEncoder::from_rgb(&rgb_data, width, height);

            // Encode the image in lossless mode
            let webp_data = encoder.encode_lossless();

            // Convert WebPMemory to Vec<u8> using `as_bytes()`
            let webp_bytes = webp_data.as_bytes();

            // Save the encoded WebP data to a file
            fs::write(&file_path, webp_bytes).expect("Failed to save WebP image");
        });
        output
    }

    async fn slice_selected_bodies(
        bodies_clone: Rc<RefCell<Vec<Rc<RefCell<Body>>>>>,
        gpu_slicer_clone: Rc<RefCell<Option<GPUSlicer>>>,
        cpu_slicer_clone: Rc<RefCell<CPUSlicer>>,
    ) -> Vec<ImageBuffer<Luma<u8>, Vec<u8>>> {
        // Clone the Rc<RefCell<Body>>s into a new vector to avoid borrowing issues
        let bodies_vec = {
            let bodies_ref = bodies_clone.borrow();
            bodies_ref.as_slice().to_vec()
        };

        let mut bodies_vec_filtered = Vec::new();
        for b in bodies_vec {
            if b.borrow().selected {bodies_vec_filtered.push(b)};
        }
        let output: Vec<ImageBuffer<Luma<u8>, Vec<u8>>>;
        if let Some(gpu_slicer) = gpu_slicer_clone.borrow_mut().as_mut() {
            output = gpu_slicer.slice_bodies(bodies_vec_filtered).unwrap()
        } else {
            output = cpu_slicer_clone
                .borrow_mut()
                .slice_bodies(bodies_vec_filtered)
                .unwrap()
        }
        // For now let's try just writing the data to a series of images in the test slices dir inside of a new dir with a current unix timestamp as the name
        // insert folder and file writing code here.
        let start = SystemTime::now();
        let since_the_epoch = start
            .duration_since(UNIX_EPOCH)
            .expect("Time went backwards");
        let timestamp = since_the_epoch.as_secs();

        // Create a new directory inside "test slices" with the timestamp as its name
        let dir_path = format!("slices/{}", timestamp);
        fs::create_dir_all(&dir_path).expect("Failed to create directory");

        // Iterate over the output images and save each one to a file in lossless WebP format
        output.par_iter().enumerate().for_each(|(i, image)| {
            let file_path = format!("{}/slice_{:04}.webp", dir_path, i);

            // Convert ImageBuffer<Luma<u8>, Vec<u8>> to ImageBuffer<Rgb<u8>, Vec<u8>>
            let rgb_image: ImageBuffer<Rgb<u8>, Vec<u8>> = convert_luma_to_rgb(image);

            // Retrieve width and height before moving rgb_image
            let width = rgb_image.width();
            let height = rgb_image.height();

            // Flatten the RGB image into a Vec<u8>
            let rgb_data = rgb_image.into_raw();

            // Create a WebP encoder with lossless encoding
            let encoder = WebpEncoder::from_rgb(&rgb_data, width, height);

            // Encode the image in lossless mode
            let webp_data = encoder.encode_lossless();

            // Convert WebPMemory to Vec<u8> using `as_bytes()`
            let webp_bytes = webp_data.as_bytes();

            // Save the encoded WebP data to a file
            fs::write(&file_path, webp_bytes).expect("Failed to save WebP image");
        });
        output
    }

    /// Converts an ImageBuffer with Luma<u8> pixels to an ImageBuffer with Rgb<u8> pixels
    fn convert_luma_to_rgb(
        image: &ImageBuffer<Luma<u8>, Vec<u8>>,
    ) -> ImageBuffer<Rgb<u8>, Vec<u8>> {
        let (width, height) = image.dimensions();
        let mut rgb_image = ImageBuffer::new(width, height);

        for (x, y, pixel) in image.enumerate_pixels() {
            let luma = pixel[0];
            rgb_image.put_pixel(x, y, Rgb([luma, luma, luma]));
        }

        rgb_image
    }

    let bodies_clone = Rc::clone(&state.shared_bodies);
    let gpu_slicer_clone = Rc::clone(&state.shared_gpu_slicer);
    let cpu_slicer_clone = Rc::clone(&state.shared_cpu_slicer);
    app.on_slice_selected(move || {
        let bodies_clone = Rc::clone(&bodies_clone);
        let gpu_slicer_clone = Rc::clone(&gpu_slicer_clone);
        let cpu_slicer_clone = Rc::clone(&cpu_slicer_clone);
        let slint_future = async move {
            slice_selected_bodies(bodies_clone, gpu_slicer_clone, cpu_slicer_clone).await;
            // replace with slice selected bodies
        };
        slint::spawn_local(async_compat::Compat::new(slint_future)).unwrap();
    });

    // Slicing button callbacks
    {
        let bodies_clone = Rc::clone(&state.shared_bodies);
        let gpu_slicer_clone = Rc::clone(&state.shared_gpu_slicer);
        let cpu_slicer_clone = Rc::clone(&state.shared_cpu_slicer);
        app.on_slice_all(move || {
            // Clone the Rc pointers inside the closure
            let bodies_clone = Rc::clone(&bodies_clone);
            let gpu_slicer_clone = Rc::clone(&gpu_slicer_clone);
            let cpu_slicer_clone = Rc::clone(&cpu_slicer_clone);
            let slint_future = async move {
                slice_all_bodies(bodies_clone, gpu_slicer_clone, cpu_slicer_clone).await
            };
            slint::spawn_local(async_compat::Compat::new(slint_future)).unwrap();
        });
    }

    // Delete item callbacks
    { 
        app.on_delete_item_by_uuid(move|uuid:SharedString|{
            let mesh_renderer_clone:SharedMeshRenderer = Rc::clone(&state.shared_mesh_renderer);
            let bodies_clone: SharedBodies = Rc::clone(&state.shared_bodies);
            delete_body_by_uuid(&mesh_renderer_clone, &bodies_clone, uuid);
        });
    }
    fn delete_body_by_uuid(
        mesh_renderer_clone: &Rc<RefCell<Option<MeshRenderer>>>,
        bodies_clone: &Rc<RefCell<Vec<Rc<RefCell<Body>>>>>,
        uuid: SharedString,
    ) {
        // Find the body to remove without mutably borrowing bodies_clone
        let body_to_remove = {
            let bodies = bodies_clone.borrow();
            bodies.iter().find(|body_rc| {
                let body = body_rc.borrow();
                body.eq_uuid_ss(&uuid)
            }).cloned()
        };
    
        if let Some(body_rc) = body_to_remove {
            // Remove the body from the renderer
            if let Some(renderer) = mesh_renderer_clone.borrow_mut().as_mut() {
                renderer.remove_body(body_rc.clone());
            }
    
            // Remove the body from bodies_clone
            let mut bodies = bodies_clone.borrow_mut();
            if let Some(pos) = bodies.iter().position(|x| Rc::ptr_eq(x, &body_rc)) {
                bodies.remove(pos);
            }
        }
    }
    // Run the Slint application
    app.run().unwrap();
}
