// Copyright © SixtyFPS GmbH <info@slint.dev>
// SPDX-License-Identifier: MIT

use std::num::NonZeroU32;
use std::rc::Rc;

slint::include_modules!();

use glow::HasContext;

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
                Self { saved_value, gl: gl.clone() }
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
                Self { saved_value, gl: gl.clone() }
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

struct DemoTexture {
    texture: glow::Texture,
    width: u32,
    height: u32,
    fbo: glow::Framebuffer,
    gl: Rc<glow::Context>,
}

impl DemoTexture {
    unsafe fn new(gl: &Rc<glow::Context>, width: u32, height: u32) -> Self {
        let fbo = gl.create_framebuffer().expect("Unable to create framebuffer");

        let texture = gl.create_texture().expect("Unable to allocate texture");

        let _saved_texture_binding = ScopedTextureBinding::new(gl, Some(texture));

        let old_unpack_alignment = gl.get_parameter_i32(glow::UNPACK_ALIGNMENT);
        let old_unpack_row_length = gl.get_parameter_i32(glow::UNPACK_ROW_LENGTH);
        let old_unpack_skip_pixels = gl.get_parameter_i32(glow::UNPACK_SKIP_PIXELS);
        let old_unpack_skip_rows = gl.get_parameter_i32(glow::UNPACK_SKIP_ROWS);

        gl.pixel_store_i32(glow::UNPACK_ALIGNMENT, 1);
        gl.tex_parameter_i32(glow::TEXTURE_2D, glow::TEXTURE_MIN_FILTER, glow::LINEAR as i32);
        gl.tex_parameter_i32(glow::TEXTURE_2D, glow::TEXTURE_MAG_FILTER, glow::LINEAR as i32);
        gl.tex_parameter_i32(glow::TEXTURE_2D, glow::TEXTURE_WRAP_S, glow::CLAMP_TO_EDGE as i32);
        gl.tex_parameter_i32(glow::TEXTURE_2D, glow::TEXTURE_WRAP_T, glow::CLAMP_TO_EDGE as i32);
        gl.pixel_store_i32(glow::UNPACK_ROW_LENGTH, width as i32);
        gl.pixel_store_i32(glow::UNPACK_SKIP_PIXELS, 0);
        gl.pixel_store_i32(glow::UNPACK_SKIP_ROWS, 0);

        gl.tex_image_2d(
            glow::TEXTURE_2D,
            0,
            glow::RGBA as _,
            width as _,
            height as _,
            0,
            glow::RGBA as _,
            glow::UNSIGNED_BYTE as _,
            None,
        );

        let _saved_fbo_binding = ScopedFrameBufferBinding::new(gl, Some(fbo));

        gl.framebuffer_texture_2d(
            glow::FRAMEBUFFER,
            glow::COLOR_ATTACHMENT0,
            glow::TEXTURE_2D,
            Some(texture),
            0,
        );

        debug_assert_eq!(
            gl.check_framebuffer_status(glow::FRAMEBUFFER),
            glow::FRAMEBUFFER_COMPLETE
        );

        gl.pixel_store_i32(glow::UNPACK_ALIGNMENT, old_unpack_alignment);
        gl.pixel_store_i32(glow::UNPACK_ROW_LENGTH, old_unpack_row_length);
        gl.pixel_store_i32(glow::UNPACK_SKIP_PIXELS, old_unpack_skip_pixels);
        gl.pixel_store_i32(glow::UNPACK_SKIP_ROWS, old_unpack_skip_rows);

        Self { texture, width, height, fbo, gl: gl.clone() }
    }

    unsafe fn with_texture_as_active_fbo<R>(&self, callback: impl FnOnce() -> R) -> R {
        let _saved_fbo = ScopedFrameBufferBinding::new(&self.gl, Some(self.fbo));
        callback()
    }
}

impl Drop for DemoTexture {
    fn drop(&mut self) {
        unsafe {
            self.gl.delete_framebuffer(self.fbo);
            self.gl.delete_texture(self.texture);
        }
    }
}

struct DemoRenderer {
    gl: Rc<glow::Context>,
    program: glow::Program,
    vbo: glow::Buffer,
    vao: glow::VertexArray,
    view_proj_location: glow::UniformLocation,
    displayed_texture: DemoTexture,
    next_texture: DemoTexture,
}

impl DemoRenderer {
    fn new(gl: glow::Context) -> Self {
        let gl = Rc::new(gl);
        unsafe {
            // Create shader program
            let program = gl.create_program().expect("Cannot create program");

            // Vertex Shader (GLSL)
            let vertex_shader_source = r#"
            #version 300 es

            precision mediump float;
            precision mediump int;

            layout(location = 0) in vec3 position;
            layout(location = 1) in vec3 normal;

            uniform mat4 view_proj;

            out vec3 v_normal;

            void main() {
                gl_Position = view_proj * vec4(position, 1.0);
                v_normal = normal;
            }
            "#;

            // Fragment Shader (GLSL)
            let fragment_shader_source = r#"
            #version 300 es

            precision mediump float;
            precision mediump int;

            in vec3 v_normal;

            out vec4 fragColor;

            void main() {
                vec3 light_dir = normalize(vec3(0.0, 1.0, 1.0));
                float brightness = max(dot(normalize(v_normal), light_dir), 0.0);
                fragColor = vec4(vec3(brightness), 1.0);
            }
            "#;

            // Compile shaders and link program
            let shader_sources = [
                (glow::VERTEX_SHADER, vertex_shader_source),
                (glow::FRAGMENT_SHADER, fragment_shader_source),
            ];

            let mut shaders = Vec::with_capacity(shader_sources.len());

            for (shader_type, shader_source) in &shader_sources {
                let shader = gl.create_shader(*shader_type).expect("Cannot create shader");
                gl.shader_source(shader, shader_source);
                gl.compile_shader(shader);
                if !gl.get_shader_compile_status(shader) {
                    panic!("Shader compile error: {}", gl.get_shader_info_log(shader));
                }
                gl.attach_shader(program, shader);
                shaders.push(shader);
            }

            gl.link_program(program);
            if !gl.get_program_link_status(program) {
                panic!("Program link error: {}", gl.get_program_info_log(program));
            }

            for shader in shaders {
                gl.detach_shader(program, shader);
                gl.delete_shader(shader);
            }

            // Get attribute and uniform locations
            let view_proj_location = gl.get_uniform_location(program, "view_proj").unwrap();
            let position_location = gl.get_attrib_location(program, "position").unwrap() as u32;
            let normal_location = gl.get_attrib_location(program, "normal").unwrap() as u32;

            // Prepare vertex data with positions and normals
            let vertices: [f32; 24] = [
                // Positions        // Normals
                -1.0, -1.0, 0.0,    0.0, 0.0, 1.0,  // Vertex 1
                 1.0, -1.0, 0.0,    0.0, 0.0, 1.0,  // Vertex 2
                 1.0,  1.0, 0.0,    0.0, 0.0, 1.0,  // Vertex 3
                -1.0,  1.0, 0.0,    0.0, 0.0, 1.0,  // Vertex 4
            ];

            // Set up VBO and VAO
            let vbo = gl.create_buffer().expect("Cannot create buffer");
            gl.bind_buffer(glow::ARRAY_BUFFER, Some(vbo));
            gl.buffer_data_u8_slice(
                glow::ARRAY_BUFFER,
                bytemuck::cast_slice(&vertices),
                glow::STATIC_DRAW,
            );

            let vao = gl.create_vertex_array().expect("Cannot create vertex array");
            gl.bind_vertex_array(Some(vao));

            // Position attribute
            gl.enable_vertex_attrib_array(position_location);
            gl.vertex_attrib_pointer_f32(
                position_location,
                3,                 // size
                glow::FLOAT,       // type
                false,             // normalized
                6 * 4,             // stride (6 floats per vertex)
                0,                 // offset
            );

            // Normal attribute
            gl.enable_vertex_attrib_array(normal_location);
            gl.vertex_attrib_pointer_f32(
                normal_location,
                3,
                glow::FLOAT,
                false,
                6 * 4,
                (3 * 4),           // offset (after the first 3 floats)
            );

            gl.bind_buffer(glow::ARRAY_BUFFER, None);
            gl.bind_vertex_array(None);

            // Initialize textures
            let displayed_texture = DemoTexture::new(&gl, 320, 200);
            let next_texture = DemoTexture::new(&gl, 320, 200);

            Self {
                gl,
                program,
                view_proj_location,
                vbo,
                vao,
                displayed_texture,
                next_texture,
            }
        }
    }

    fn render(&mut self, width: u32, height: u32) -> slint::Image {
        unsafe {
            let gl = &self.gl;

            gl.use_program(Some(self.program));

            let _saved_vbo = ScopedVBOBinding::new(gl, Some(self.vbo));
            let _saved_vao = ScopedVAOBinding::new(gl, Some(self.vao));

            // Resize texture if necessary
            if self.next_texture.width != width || self.next_texture.height != height {
                let mut new_texture = DemoTexture::new(gl, width, height);
                std::mem::swap(&mut self.next_texture, &mut new_texture);
            }

            self.next_texture.with_texture_as_active_fbo(|| {
                // Save and set viewport
                let mut saved_viewport: [i32; 4] = [0, 0, 0, 0];
                gl.get_parameter_i32_slice(glow::VIEWPORT, &mut saved_viewport);
                gl.viewport(0, 0, self.next_texture.width as i32, self.next_texture.height as i32);

                // Set the view_proj uniform (identity matrix)
                let view_proj_matrix: [f32; 16] = [
                    1.0, 0.0, 0.0, 0.0, // Column 1
                    0.0, 1.0, 0.0, 0.0, // Column 2
                    0.0, 0.0, 1.0, 0.0, // Column 3
                    0.0, 0.0, 0.0, 1.0, // Column 4
                ];
                gl.uniform_matrix_4_f32_slice(
                    Some(&self.view_proj_location),
                    false,
                    &view_proj_matrix,
                );

                // Draw the scene
                gl.bind_vertex_array(Some(self.vao));
                gl.draw_arrays(glow::TRIANGLE_FAN, 0, 4);
                gl.bind_vertex_array(None);

                // Restore viewport
                gl.viewport(
                    saved_viewport[0],
                    saved_viewport[1],
                    saved_viewport[2],
                    saved_viewport[3],
                );
            });

            gl.use_program(None);
        }

        // Create the result texture
        let result_texture = unsafe {
            slint::BorrowedOpenGLTextureBuilder::new_gl_2d_rgba_texture(
                self.next_texture.texture.0,
                (self.next_texture.width, self.next_texture.height).into(),
            )
            .build()
        };

        // Swap textures for the next frame
        std::mem::swap(&mut self.next_texture, &mut self.displayed_texture);

        result_texture
    }
}

impl Drop for DemoRenderer {
    fn drop(&mut self) {
        unsafe {
            self.gl.delete_program(self.program);
            self.gl.delete_vertex_array(self.vao);
            self.gl.delete_buffer(self.vbo);
        }
    }
}
fn main() {
    let app = App::new().unwrap();

    let mut underlay = None;

    let app_weak = app.as_weak();

    if let Err(error) = app.window().set_rendering_notifier(move |state, graphics_api| {
        // eprintln!("rendering state {:#?}", state);

        match state {
            slint::RenderingState::RenderingSetup => {
                let context = match graphics_api {
                    slint::GraphicsAPI::NativeOpenGL { get_proc_address } => unsafe {
                        glow::Context::from_loader_function_cstr(|s| get_proc_address(s))
                    },
                    _ => return,
                };
                underlay = Some(DemoRenderer::new(context))
            }
            slint::RenderingState::BeforeRendering => {
                if let (Some(underlay), Some(app)) = (underlay.as_mut(), app_weak.upgrade()) {
                    let texture = underlay.render(
                        50, 50,
                    );
                    app.set_texture(slint::Image::from(texture));
                    app.window().request_redraw();
                }
            }
            slint::RenderingState::AfterRendering => {}
            slint::RenderingState::RenderingTeardown => {
                drop(underlay.take());
            }
            _ => {}
        }
    }) {
        match error {
            slint::SetRenderingNotifierError::Unsupported => eprintln!("This example requires the use of the GL backend. Please run with the environment variable SLINT_BACKEND=GL set."),
            _ => unreachable!()
        }
        std::process::exit(1);
    }

    app.run().unwrap();
}
