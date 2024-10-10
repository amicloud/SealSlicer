use std::fs;
use std::rc::Rc;
slint::include_modules!();
use crate::camera::Camera;
use crate::camera::CameraMove;
use crate::mesh_data::MeshData;
use crate::texture::Texture;
use crate::ScopedVAOBinding;
use crate::ScopedVBOBinding;
use glow::HasContext;
pub struct MeshRenderer {
    gl: Rc<glow::Context>,
    program: glow::Program,
    vao: glow::VertexArray,
    vbo: glow::Buffer,
    ebo: glow::Buffer,
    view_proj_location: glow::UniformLocation,
    displayed_texture: Texture,
    next_texture: Texture,
    meshes: Vec<MeshData>,
    camera: Camera,
    mesh_changed: bool,
}

impl MeshRenderer {
    pub fn new(gl: glow::Context) -> Self {
        let gl = Rc::new(gl);
        unsafe {
            // Create shader program
            let shader_program = gl.create_program().expect("Cannot create program");

            let manifest_dir = env!("CARGO_MANIFEST_DIR");
            let vertex_shader_path = format!("{}/shaders/vertex_shader.glsl", manifest_dir);
            let fragment_shader_path = format!("{}/shaders/fragment_shader.glsl", manifest_dir);

            let vertex_shader_source =
                fs::read_to_string(&vertex_shader_path).expect("Failed to read vertex shader file");
            let fragment_shader_source = fs::read_to_string(&fragment_shader_path)
                .expect("Failed to read fragment shader file");

            // Compile shaders and link program
            let shader_sources = [
                (glow::VERTEX_SHADER, vertex_shader_source),
                (glow::FRAGMENT_SHADER, fragment_shader_source),
            ];

            let mut shaders = Vec::with_capacity(shader_sources.len());

            for (shader_type, shader_source) in &shader_sources {
                let shader = gl
                    .create_shader(*shader_type)
                    .expect("Cannot create shader");
                gl.shader_source(shader, shader_source);
                gl.compile_shader(shader);
                if !gl.get_shader_compile_status(shader) {
                    panic!(
                        "Fatal Error: Shader compile error: {}",
                        gl.get_shader_info_log(shader)
                    );
                }
                gl.attach_shader(shader_program, shader);
                shaders.push(shader);
            }

            gl.link_program(shader_program);
            if !gl.get_program_link_status(shader_program) {
                panic!(
                    "Fatal Error: Shader program link error: {}",
                    gl.get_program_info_log(shader_program)
                );
            }

            for shader in shaders {
                gl.detach_shader(shader_program, shader);
                gl.delete_shader(shader);
            }

            // Get attribute and uniform locations
            let view_proj_location = gl
                .get_uniform_location(shader_program, "view_proj")
                .unwrap();
            let position_location =
                gl.get_attrib_location(shader_program, "position").unwrap() as u32;
            let normal_location = gl.get_attrib_location(shader_program, "normal").unwrap() as u32;

            // Set up VBO, EBO, VAO
            let vbo = gl.create_buffer().expect("Cannot create buffer");
            gl.bind_buffer(glow::ARRAY_BUFFER, Some(vbo));

            let vao = gl
                .create_vertex_array()
                .expect("Cannot create vertex array");
            gl.bind_vertex_array(Some(vao));

            let ebo = gl.create_buffer().expect("Cannot create EBO");
            gl.bind_buffer(glow::ELEMENT_ARRAY_BUFFER, Some(ebo));

            // Position attribute
            gl.enable_vertex_attrib_array(position_location);
            gl.vertex_attrib_pointer_f32(
                position_location,
                3,           // size
                glow::FLOAT, // type
                false,       // normalized
                6 * 4,       // stride (6 floats per vertex)
                0,           // offset
            );

            // Normal attribute
            gl.enable_vertex_attrib_array(normal_location);
            gl.vertex_attrib_pointer_f32(
                normal_location,
                3,
                glow::FLOAT,
                true,
                6 * 4, // stride (6 floats per vertex)
                3 * 4, // offset (after the first 3 floats)
            );

            gl.bind_buffer(glow::ARRAY_BUFFER, None);
            gl.bind_vertex_array(None);
            let width = 1920;
            let height = 1080;

            let depth_buffer = gl.create_renderbuffer().unwrap();
            gl.bind_renderbuffer(glow::RENDERBUFFER, Some(depth_buffer));
            gl.renderbuffer_storage(
                glow::RENDERBUFFER,
                glow::DEPTH_COMPONENT16,
                width as i32,
                height as i32,
            );
            gl.framebuffer_renderbuffer(
                glow::FRAMEBUFFER,
                glow::DEPTH_ATTACHMENT,
                glow::RENDERBUFFER,
                Some(depth_buffer),
            );
            gl.enable(glow::DEPTH_TEST);
            gl.depth_func(glow::LESS);

            // Initialize textures
            let displayed_texture = Texture::new(&gl, width, height);
            let next_texture = Texture::new(&gl, width, height);
            let meshes = Vec::new();
            let camera = Camera::new();
            let mesh_changed = false;
            Self {
                gl,
                program: shader_program,
                view_proj_location,
                vao,
                vbo,
                ebo,
                displayed_texture,
                next_texture,
                meshes,
                camera,
                mesh_changed,
            }
        }
    }

    pub fn render(&mut self, width: u32, height: u32) -> slint::Image {
        unsafe {
            let gl = &self.gl;
            gl.use_program(Some(self.program));
            let _saved_vbo = ScopedVBOBinding::new(gl, Some(self.vbo));
            let _saved_vao = ScopedVAOBinding::new(gl, Some(self.vao));
            // Enable face culling
            gl.enable(glow::CULL_FACE);
            gl.cull_face(glow::BACK);

            // Resize texture if necessary
            if self.next_texture.width != width || self.next_texture.height != height {
                let mut new_texture = Texture::new(gl, width, height);
                std::mem::swap(&mut self.next_texture, &mut new_texture);
            }

            self.next_texture.with_texture_as_active_fbo(|| {
                if gl.check_framebuffer_status(glow::FRAMEBUFFER) != glow::FRAMEBUFFER_COMPLETE {
                    panic!("Framebuffer is not complete!");
                }
                // **Enable depth testing inside the framebuffer binding**
                gl.enable(glow::DEPTH_TEST);
                gl.depth_func(glow::LEQUAL);
                // Clear color and depth buffers
                gl.clear(glow::COLOR_BUFFER_BIT | glow::DEPTH_BUFFER_BIT);
                // Save and set viewport
                let mut saved_viewport: [i32; 4] = [0; 4];
                gl.get_parameter_i32_slice(glow::VIEWPORT, &mut saved_viewport);
                gl.viewport(
                    0,
                    0,
                    self.next_texture.width as i32,
                    self.next_texture.height as i32,
                );

                // Compute view and projection matrices
                let aspect_ratio = width as f32 / height as f32;
                let projection = self.camera.projection_matrix(aspect_ratio);
                let view = self.camera.view_matrix();
                let view_proj = projection * view;

                // Convert to column-major array
                let view_proj_matrix: [f32; 16] = view_proj
                    .as_slice()
                    .try_into()
                    .expect("Slice with incorrect length");

                // Set the view_proj uniform
                gl.uniform_matrix_4_f32_slice(
                    Some(&self.view_proj_location),
                    false,
                    &view_proj_matrix,
                );

                // Bind VAO and draw
                gl.bind_vertex_array(Some(self.vao));

                // Calculate the number of vertices
                let total_vertices = self
                    .meshes
                    .iter()
                    .map(|mesh| mesh.vertices.len() as i32)
                    .sum::<i32>(); 

                if gl.check_framebuffer_status(glow::FRAMEBUFFER) != glow::FRAMEBUFFER_COMPLETE {
                    panic!("Framebuffer is not complete!");
                }
                gl.draw_arrays(glow::TRIANGLES, 0, total_vertices);

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

    pub fn update_buffers(&mut self) {
        if !self.mesh_changed {
            return;
        } else {
            unsafe {
                // Collect all vertex-normal data from meshes
                let mut all_vertices: Vec<f32> = Vec::new();
                let mut all_indices: Vec<usize> = Vec::new();
                let mut index_offset = 0;

                for mesh in &self.meshes {
                    all_vertices.extend(mesh.vertices.iter().flat_map(|v| {
                        vec![
                            v.position[0],
                            v.position[1],
                            v.position[2],
                            v.normal[0],
                            v.normal[1],
                            v.normal[2],
                        ]
                    }));
                    all_indices.extend(
                        mesh.indices
                            .iter()
                            .map(|i| i.iter().map(|f| f + index_offset))
                            .flatten(),
                    );
                    index_offset += mesh.vertices.len() as usize;
                }

                // Bind the VBO
                self.gl.bind_buffer(glow::ARRAY_BUFFER, Some(self.vbo));

                // Upload the vertex data to the GPU
                self.gl.buffer_data_u8_slice(
                    glow::ARRAY_BUFFER,
                    bytemuck::cast_slice(&all_vertices),
                    glow::STATIC_DRAW, // Use DYNAMIC_DRAW if you plan to update frequently
                );

                // Bind the EBO
                self.gl
                    .bind_buffer(glow::ELEMENT_ARRAY_BUFFER, Some(self.ebo));

                // Upload the index data to the GPU
                self.gl.buffer_data_u8_slice(
                    glow::ELEMENT_ARRAY_BUFFER,
                    bytemuck::cast_slice(&all_indices),
                    glow::STATIC_DRAW,
                );

                // Unbind the buffers
                self.gl.bind_buffer(glow::ARRAY_BUFFER, None);
                self.gl.bind_buffer(glow::ELEMENT_ARRAY_BUFFER, None);
            }
            self.mesh_changed = false;
        }
    }

    pub fn camera_pitch_yaw(&mut self, delta_x: f32, delta_y: f32) {
        self.camera.pitch_yaw(delta_x, -delta_y);
    }


    pub fn add_mesh(&mut self, mesh: MeshData) {
        self.meshes.push(mesh);
        self.mesh_changed = true;
        self.update_buffers();
    }

    #[allow(dead_code)] // It will be used eventually!
    /// Removes a mesh by index and updates the buffers.
    pub fn remove_mesh(&mut self, index: usize) {
        if index < self.meshes.len() {
            self.meshes.remove(index);
            self.mesh_changed = true;
        }
        self.update_buffers();
    }

    pub fn move_camera(&mut self, camera_move: CameraMove) {
        let amount = 1.0; // Adjust the amount as needed
        match camera_move {
            CameraMove::Up => self.camera.move_up(amount),
            CameraMove::Down => self.camera.move_down(amount),
            CameraMove::Left => self.camera.move_left(amount),
            CameraMove::Right => self.camera.move_right(amount),
            CameraMove::ZoomIn => self.camera.zoom(amount * 10.0),
            CameraMove::ZoomOut => self.camera.zoom(amount * 10.0),
        }
    }

    pub(crate) fn zoom(&mut self, amt: f32) {
        self.camera.zoom(amt);
    }
}

impl Drop for MeshRenderer {
    fn drop(&mut self) {
        unsafe {
            self.gl.delete_program(self.program);
            self.gl.delete_vertex_array(self.vao);
            self.gl.delete_buffer(self.vbo);
        }
    }
}
