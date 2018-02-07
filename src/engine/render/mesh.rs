use webgl::*;
use std::mem::size_of;

use super::ShaderProgram;
use engine::core::ComponentBased;
use std::cell::RefCell;

trait IntoBytes {
    fn into_bytes(self) -> Vec<u8>;
}

impl<T> IntoBytes for Vec<T> {
    fn into_bytes(self) -> Vec<u8> {
        let len = size_of::<T>() * self.len();
        unsafe {
            let slice = self.into_boxed_slice();
            Vec::<u8>::from_raw_parts(Box::into_raw(slice) as _, len, len)
        }
    }
}

pub struct Mesh {
    pub mesh_buffer: MeshBuffer,
    gl_state: RefCell<Option<MeshGLState>>,
}

impl ComponentBased for Mesh {}

struct MeshGLState {
    pub vb: WebGLBuffer,
    pub uvb: Option<WebGLBuffer>,
    pub nb: Option<WebGLBuffer>,
    pub ib: WebGLBuffer,
}

impl Mesh {
    pub fn new(mesh_buffer: MeshBuffer) -> Mesh {
        Mesh {
            mesh_buffer: mesh_buffer,
            gl_state: RefCell::new(None),
        }
    }

    pub fn bind(&self, gl: &WebGLRenderingContext, program: &ShaderProgram) {
        self.prepare(gl);

        let state_option = self.gl_state.borrow();
        let state = state_option.as_ref().unwrap();

        /*======= Associating shaders to buffer objects =======*/

        // Bind vertex buffer object
        gl.bind_buffer(BufferKind::Array, &state.vb);

        // Point an position attribute to the currently bound VBO
        if let Some(coord) = program.get_coord(gl, "aVertexPosition") {
            gl.vertex_attrib_pointer(coord, AttributeSize::Three, DataType::Float, false, 0, 0);
        }

        if let Some(ref nb) = state.nb {
            gl.bind_buffer(BufferKind::Array, nb);
            // Point an normal attribute to the currently bound VBO

            if let Some(coord) = program.get_coord(gl, "aVertexNormal") {
                gl.vertex_attrib_pointer(coord, AttributeSize::Three, DataType::Float, false, 0, 0);
            }
        }

        if let Some(ref uvb) = state.uvb {
            gl.bind_buffer(BufferKind::Array, uvb);
            // Point an uv attribute to the currently bound VBO

            if let Some(coord) = program.get_coord(gl, "aTextureCoord") {
                gl.vertex_attrib_pointer(coord, AttributeSize::Two, DataType::Float, false, 0, 0);
            }
        }

        // Bind index buffer object
        gl.bind_buffer(BufferKind::ElementArray, &state.ib);
    }

    pub fn render(&self, gl: &WebGLRenderingContext) {
        gl.draw_elements(
            Primitives::Triangles,
            self.indices().len(),
            DataType::U16,
            0,
        );
    }

    pub fn indices(&self) -> &Vec<u16> {
        &self.mesh_buffer.indices
    }

    pub fn prepare(&self, gl: &WebGLRenderingContext) {
        if self.gl_state.borrow().is_none() {
            self.gl_state.replace(Some(mesh_bind_buffer(
                &self.mesh_buffer.vertices,
                &self.mesh_buffer.uvs,
                &self.mesh_buffer.normals,
                &self.mesh_buffer.indices,
                gl,
            )));
        }
    }
}

pub struct MeshBuffer {
    #[allow(dead_code)]
    pub vertices: Vec<f32>,
    pub uvs: Option<Vec<f32>>,
    pub normals: Option<Vec<f32>>,
    pub indices: Vec<u16>,
}

fn mesh_bind_buffer(
    vertices: &Vec<f32>,
    uvs: &Option<Vec<f32>>,
    normals: &Option<Vec<f32>>,
    indices: &Vec<u16>,
    gl: &WebGLRenderingContext,
) -> MeshGLState {
    // Create an empty buffer object to store vertex buffer
    let vertex_buffer = gl.create_buffer();
    {
        // Bind appropriate array buffer to it
        gl.bind_buffer(BufferKind::Array, &vertex_buffer);

        // Pass the vertex data to the buffer
        let cv = vertices.clone();
        gl.buffer_data(BufferKind::Array, &cv.into_bytes(), DrawMode::Static);

        // Unbind the buffer
        gl.unbind_buffer(BufferKind::Array);
    }

    // Create an empty buffer object to store uv buffer
    let uv_buffer = match uvs {
        &Some(ref uvs) => {
            let uv_buffer = gl.create_buffer();
            {
                // Bind appropriate array buffer to it
                gl.bind_buffer(BufferKind::Array, &uv_buffer);

                // Pass the vertex data to the buffer
                let uvv = uvs.clone();
                gl.buffer_data(BufferKind::Array, &uvv.into_bytes(), DrawMode::Static);

                // Unbind the buffer
                gl.unbind_buffer(BufferKind::Array);

                Some(uv_buffer)
            }
        }
        _ => None,
    };

    // Create an Normal Buffer
    let normal_buffer = match normals {
        &Some(ref normals) => {
            let normal_buffer = gl.create_buffer();
            {
                // Bind appropriate array buffer to it
                gl.bind_buffer(BufferKind::Array, &normal_buffer);

                let ns = normals.clone();
                gl.buffer_data(BufferKind::Array, &ns.into_bytes(), DrawMode::Static);

                // Unbind the buffer
                gl.unbind_buffer(BufferKind::Array);

                Some(normal_buffer)
            }
        }
        _ => None,
    };

    // Create an empty buffer object to store Index buffer
    let index_buffer = gl.create_buffer();
    {
        // Bind appropriate array buffer to it
        gl.bind_buffer(BufferKind::ElementArray, &index_buffer);

        // Pass the vertex data to the buffer
        let ci = indices.clone();
        gl.buffer_data(BufferKind::ElementArray, &ci.into_bytes(), DrawMode::Static);

        // Unbind the buffer
        gl.unbind_buffer(BufferKind::ElementArray);
    }

    MeshGLState {
        vb: vertex_buffer,
        uvb: uv_buffer,
        nb: normal_buffer,
        ib: index_buffer,
    }
}
